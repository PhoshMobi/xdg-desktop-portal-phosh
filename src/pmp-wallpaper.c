/*
 * Copyright © 2023-2024 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Heavily based on the xdg-desktop-portal-gnome
 *
 * Authors:
 *       Georges Basile Stavracas Neto <georges.stavracas@gmail.com>
 *       Guido Günther <agx@sigxcpu.org>
 */

#include "pmp-config.h"

#include <gtk/gtk.h>

#include <glib/gi18n.h>
#include <gdesktop-enums.h>

#include "xdg-desktop-portal-dbus.h"

#include "pmp-external-win.h"
#include "pmp-request.h"
#include "pmp-utils.h"
#include "pmp-wallpaper-dialog.h"
#include "pmp-wallpaper.h"

#define BACKGROUND_SCHEMA "org.gnome.desktop.background"

typedef struct {
  PmpImplWallpaper      *impl;
  GDBusMethodInvocation *invocation;
  Request               *request;
  GtkWindow             *dialog;
  PmpExternalWin        *external_parent;

  guint                  response;
  gchar                 *picture_uri;
} PmpWallpaperDialogHandle;

static void
wallpaper_dialog_handle_free (gpointer data)
{
  PmpWallpaperDialogHandle *handle = data;

  g_clear_object (&handle->external_parent);
  g_clear_object (&handle->request);
  g_clear_pointer (&handle->picture_uri, g_free);

  g_free (handle);
}

static void
wallpaper_dialog_handle_close (PmpWallpaperDialogHandle *handle)
{
  g_clear_pointer (&handle->dialog, gtk_window_destroy);
  wallpaper_dialog_handle_free (handle);
}

static void
send_response (PmpWallpaperDialogHandle *handle)
{
  if (handle->request->exported)
    request_unexport (handle->request);

  pmp_impl_wallpaper_complete_set_wallpaper_uri (handle->impl,
                                                 handle->invocation,
                                                 handle->response);

  wallpaper_dialog_handle_close (handle);
}

static gboolean
set_gsettings (gchar *schema, gchar *uri)
{
  g_autoptr (GSettings) settings = NULL;

  settings = g_settings_new (schema);

  return (g_settings_set_string (settings, "picture-uri", uri) &&
          g_settings_set_string (settings, "picture-uri-dark", uri) &&
          g_settings_set_enum (settings, "picture-options", G_DESKTOP_BACKGROUND_STYLE_ZOOM));
}

static void
on_file_copy_cb (GObject      *source_object,
                 GAsyncResult *result,
                 gpointer      data)
{
  PmpWallpaperDialogHandle *handle = data;
  g_autoptr (GFile) destination = NULL;
  GFile *picture_file = G_FILE (source_object);
  g_autoptr (GError) error = NULL;
  g_autofree gchar *uri = NULL;
  gchar *contents = NULL;
  gsize length = 0;

  handle->response = 2;

  uri = g_file_get_uri (picture_file);
  if (!g_file_load_contents_finish (picture_file, result, &contents, &length, NULL, &error)) {
    g_warning ("Failed to copy '%s': %s", uri, error->message);

    goto out;
  }

  destination = g_file_new_for_uri (handle->picture_uri);
  if (!g_file_replace_contents (destination,
                                contents,
                                length,
                                NULL, FALSE,
                                G_FILE_CREATE_REPLACE_DESTINATION,
                                NULL, NULL,
                                &error)) {
    g_warning ("Failed to store image as '%s': %s", handle->picture_uri, error->message);
    goto out;
  }

  if (set_gsettings (BACKGROUND_SCHEMA, handle->picture_uri))
    handle->response = 0;
  else
    handle->response = 1;

out:
  send_response (handle);
}

static void
set_wallpaper (PmpWallpaperDialogHandle *handle,
               const gchar              *uri)
{
  g_autoptr (GFile) source = NULL;
  g_autofree gchar *path = NULL;

  path = g_build_filename (g_get_user_config_dir (), "background", NULL);
  handle->picture_uri = g_filename_to_uri (path, NULL, NULL);

  source = g_file_new_for_uri (uri);
  g_file_load_contents_async (source,
                              NULL,
                              on_file_copy_cb,
                              handle);
}

static void
handle_wallpaper_dialog_response (PmpWallpaperDialog *dialog,
                                  gint                response,
                                  gpointer            data)
{
  PmpWallpaperDialogHandle *handle = data;

  switch (response)
  {
  default:
    g_warning ("Unexpected response: %d", response);
    G_GNUC_FALLTHROUGH;

  case GTK_RESPONSE_DELETE_EVENT:
    handle->response = 2;
    break;

  case GTK_RESPONSE_CANCEL:
    handle->response = 1;
    break;

  case GTK_RESPONSE_APPLY:
    handle->response = 0;
    set_wallpaper (handle, pmp_wallpaper_dialog_get_uri (dialog));
    return;
  }

  send_response (handle);
}

static gboolean
handle_set_wallpaper_uri (PmpImplWallpaper      *object,
                          GDBusMethodInvocation *invocation,
                          const char            *arg_handle,
                          const char            *arg_app_id,
                          const char            *arg_parent_window,
                          const char            *arg_uri,
                          GVariant              *arg_options)
{
  g_autoptr (Request) request = NULL;
  PmpWallpaperDialogHandle *handle;
  const char *sender, *set_on;
  gboolean show_preview = FALSE, on_lockscreen = FALSE;
  PmpExternalWin *external_parent = NULL;
  GdkSurface *surface;
  GtkWidget *fake_parent;
  GtkWindow *dialog;

  sender = g_dbus_method_invocation_get_sender (invocation);
  request = request_new (sender, arg_app_id, arg_handle);

  g_variant_lookup (arg_options, "show-preview", "b", &show_preview);
  g_variant_lookup (arg_options, "set-on", "&s", &set_on);

  handle = g_new0 (PmpWallpaperDialogHandle, 1);
  handle->impl = object;
  handle->invocation = invocation;
  handle->request = g_object_ref (request);

  if (!show_preview) {
    set_wallpaper (handle, arg_uri);
    goto out;
  }

  if (arg_parent_window) {
    external_parent = pmp_external_win_new_from_handle (arg_parent_window);
    if (!external_parent)
      g_warning ("Failed to associate portal window with parent window %s", arg_parent_window);
  }

  fake_parent = g_object_new (GTK_TYPE_WINDOW, NULL);
  g_object_ref_sink (fake_parent);

  if (g_strcmp0 (set_on, "lockscreen") == 0)
      on_lockscreen = TRUE;

  dialog = GTK_WINDOW (pmp_wallpaper_dialog_new (arg_uri, arg_app_id, on_lockscreen));
  gtk_window_set_transient_for (dialog, GTK_WINDOW (fake_parent));
  handle->dialog = g_object_ref_sink (dialog);

  g_signal_connect (dialog, "response",
                    G_CALLBACK (handle_wallpaper_dialog_response), handle);
  gtk_widget_realize (GTK_WIDGET (dialog));

  surface = gtk_native_get_surface (GTK_NATIVE (dialog));
  if (external_parent)
    pmp_external_win_set_parent_of (external_parent, surface);

  gtk_window_present (dialog);

out:
  request_export (request, g_dbus_method_invocation_get_connection (invocation));

  return TRUE;
}

gboolean
pmp_wallpaper_init (GDBusConnection *bus, GError **error)
{
  GDBusInterfaceSkeleton *helper;

  helper = G_DBUS_INTERFACE_SKELETON (pmp_impl_wallpaper_skeleton_new ());

  g_signal_connect (helper, "handle-set-wallpaper-uri", G_CALLBACK (handle_set_wallpaper_uri),
                    NULL);

  if (!g_dbus_interface_skeleton_export (helper,
                                         bus,
                                         DESKTOP_PORTAL_OBJECT_PATH,
                                         error))
    return FALSE;

  g_debug ("providing %s", g_dbus_interface_skeleton_get_info (helper)->name);

  return TRUE;
}
