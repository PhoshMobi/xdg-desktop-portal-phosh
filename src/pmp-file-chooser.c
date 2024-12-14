/*
 * Copyright © 2016 Red Hat, Inc
 *             2024 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Authors: Matthias Clasen <mclasen@redhat.com>
 *          Guido Günther <agx@sigxcpu.org>
 *
 * Based on xdg-desktop-portal-gtk's filechooser
 */

#include "pmp-config.h"

#include <errno.h>
#include <locale.h>
#include <string.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>

#include <pfs.h>

#include <gio/gio.h>
#include <gio/gdesktopappinfo.h>
#include <gio/gunixfdlist.h>

#include <glib/gi18n.h>

#include "xdg-desktop-portal-dbus.h"

#include "pmp-file-chooser.h"
#include "pmp-request.h"
#include "pmp-utils.h"
#include "pmp-external-win.h"


typedef struct {
  PmpImplFileChooser    *impl;
  GDBusMethodInvocation *invocation;
  Request *request;
  PfsFileSelector       *file_selector;
  PfsFileSelectorMode    mode;
  gboolean multiple;
  PmpExternalWin        *external_parent;

  GStrv files;
  GtkFileFilter         *filter;

  int       response;
  GStrv     uris;

  gboolean  allow_write;

  GVariant *selected_choices;
} FileDialogHandle;


static void
file_selector_handle_free (gpointer data)
{
  FileDialogHandle *handle = data;

  g_clear_object (&handle->external_parent);
  g_clear_object (&handle->file_selector);
  g_clear_object (&handle->request);
  g_clear_object (&handle->filter);
  g_clear_pointer (&handle->files, g_strfreev);
  g_clear_pointer (&handle->uris, g_strfreev);
  g_clear_pointer (&handle->selected_choices, g_variant_unref);

  g_free (handle);
}


static void
file_selector_handle_close (FileDialogHandle *handle)
{
  file_selector_handle_free (handle);
}


static void
add_recent_entry (const char *app_id, const char *uri)
{
  GtkRecentManager *recent;
  GtkRecentData data;

  /* These fields are ignored by everybody, so it is not worth
   * spending effort on filling them out. Just use defaults.
   */
  data.display_name = NULL;
  data.description = NULL;
  data.mime_type = "application/octet-stream";
  data.app_name = (char *)app_id;
  data.app_exec = "gio open %u";
  data.groups = NULL;
  data.is_private = FALSE;

  recent = gtk_recent_manager_get_default ();
  gtk_recent_manager_add_full (recent, uri, &data);
}


static void
send_response (FileDialogHandle *handle)
{
  GVariantBuilder uri_builder;
  GVariantBuilder opt_builder;
  const char *method_name;

  method_name = g_dbus_method_invocation_get_method_name (handle->invocation);

  g_variant_builder_init (&opt_builder, G_VARIANT_TYPE_VARDICT);

  if (handle->mode == PFS_FILE_SELECTOR_MODE_SAVE_FILES && handle->uris && handle->files) {
    g_autoptr (GFile) base_dir = g_file_new_for_uri (handle->uris[0]);
    g_autoptr (GStrvBuilder) builder = g_strv_builder_new ();

    g_clear_pointer (&handle->uris, g_strfreev);
    for (guint i = 0; handle->files[i]; i++) {
      int uniqifier = 0;
      const char *file_name = handle->files[i];
      g_autoptr (GFile) file = g_file_get_child (base_dir, file_name);

      while (g_file_query_exists (file, NULL)) {
        g_autofree char *base_name = g_file_get_basename (file);
        g_auto (GStrv) parts = NULL;
        g_autoptr (GString) unique_name = NULL;

        parts = g_strsplit (base_name, ".", 2);

        unique_name = g_string_new (parts[0]);
        g_string_append_printf (unique_name, "(%i)", ++uniqifier);
        if (parts[1] != NULL)
          g_string_append (unique_name, parts[1]);

        file = g_file_get_child (base_dir, unique_name->str);
      }
      g_strv_builder_add (builder, g_file_get_uri (file));
    }
    handle->uris = g_strv_builder_end (builder);
  }

  g_variant_builder_init (&uri_builder, G_VARIANT_TYPE_STRING_ARRAY);
  for (guint i = 0; handle->uris && handle->uris[i]; i++) {
    add_recent_entry (handle->request->app_id, handle->uris[i]);
    g_variant_builder_add (&uri_builder, "s", handle->uris[i]);
  }
  g_variant_builder_add (&opt_builder, "{sv}", "uris", g_variant_builder_end (&uri_builder));

  g_variant_builder_add (&opt_builder, "{sv}", "writable",
                         g_variant_new_boolean (handle->allow_write));

  if (handle->filter) {
    GVariant *current_filter_variant = gtk_file_filter_to_gvariant (handle->filter);
    g_variant_builder_add (&opt_builder, "{sv}", "current_filter", current_filter_variant);
  }

  if (handle->selected_choices)
    g_variant_builder_add (&opt_builder, "{sv}", "choices", handle->selected_choices);

  if (handle->request->exported)
    request_unexport (handle->request);

  if (g_str_equal (method_name, "OpenFile")) {
    pmp_impl_file_chooser_complete_open_file (handle->impl,
                                              handle->invocation,
                                              handle->response,
                                              g_variant_builder_end (&opt_builder));
  } else if (g_str_equal (method_name, "SaveFile")) {
    pmp_impl_file_chooser_complete_save_file (handle->impl,
                                              handle->invocation,
                                              handle->response,
                                              g_variant_builder_end (&opt_builder));
  } else if (g_str_equal (method_name, "SaveFiles")) {
    pmp_impl_file_chooser_complete_save_files (handle->impl,
                                               handle->invocation,
                                               handle->response,
                                               g_variant_builder_end (&opt_builder));
  } else {
    g_assert_not_reached ();
  }

  file_selector_handle_close (handle);
}


static void
on_file_selector_done (PfsFileSelector *file_selector, gboolean success, gpointer user_data)
{
  FileDialogHandle *handle = user_data;
  g_autoptr (GVariant) choices = NULL;

  g_assert (PFS_IS_FILE_SELECTOR (file_selector));

  g_debug ("FileSelector done, success: %d", success);
  if (success) {
    g_autoptr (GListModel) filters = NULL;
    guint pos;

    handle->response = 0;
    handle->uris = pfs_file_selector_get_selected (file_selector);
    for (guint i = 0; i < g_strv_length (handle->uris); i++)
      g_debug ("Got uri: %s", handle->uris[i]);
    g_object_get (file_selector,
                  "filters", &filters,
                  "current-filter", &pos,
                  "selected-choices", &choices,
                  NULL);
    g_assert (G_IS_LIST_MODEL (filters));
    handle->filter = g_list_model_get_item (filters, pos);
    handle->selected_choices = g_steal_pointer (&choices);
  } else {
    handle->response = 1;
    handle->filter = NULL;
    handle->uris = NULL;
  }

  send_response (handle);
}


static gboolean
on_handle_close (PmpImplRequest        *object,
                 GDBusMethodInvocation *invocation,
                 FileDialogHandle      *handle)
{
  GVariantBuilder opt_builder;

  g_variant_builder_init (&opt_builder, G_VARIANT_TYPE_VARDICT);

  if (handle->mode == PFS_FILE_SELECTOR_MODE_OPEN_FILE) {
    pmp_impl_file_chooser_complete_open_file (handle->impl,
                                              handle->invocation,
                                              2,
                                              g_variant_builder_end (&opt_builder));
  } else if (handle->mode == PFS_FILE_SELECTOR_MODE_SAVE_FILE) {
    pmp_impl_file_chooser_complete_save_file (handle->impl,
                                              handle->invocation,
                                              2,
                                              g_variant_builder_end (&opt_builder));
  } else {
    pmp_impl_file_chooser_complete_save_files (handle->impl,
                                               handle->invocation,
                                               2,
                                               g_variant_builder_end (&opt_builder));
  }

  if (handle->request->exported)
    request_unexport (handle->request);

  file_selector_handle_close (handle);

  pmp_impl_request_complete_close (object, invocation);

  return TRUE;
}


static gboolean
on_handle_open_file (PmpImplFileChooser    *object,
                     GDBusMethodInvocation *invocation,
                     const char            *arg_handle,
                     const char            *arg_app_id,
                     const char            *arg_parent_window,
                     const char            *arg_title,
                     GVariant              *arg_options)
{
  g_autoptr (Request) request = NULL;
  const gchar *method_name;
  const gchar *sender;
  PfsFileSelectorMode mode;
  gboolean multiple = FALSE;
  gboolean directory = FALSE;
  gboolean modal;
  PfsFileSelector *file_selector;
  PmpExternalWin *external_parent = NULL;
  FileDialogHandle *handle;
  const char *accept_label;
  const char *path = g_get_home_dir ();
  g_autoptr (GVariant) current_filter = NULL;
  g_autoptr (GVariantIter) filters_iter = NULL;
  g_autoptr (GVariant) choices = NULL;
  g_autoptr (GListStore) filters = NULL;
  guint filter_pos;

  method_name = g_dbus_method_invocation_get_method_name (invocation);
  sender = g_dbus_method_invocation_get_sender (invocation);

  request = request_new (sender, arg_app_id, arg_handle);

  if (g_str_equal (method_name, "SaveFile")) {
    mode = PFS_FILE_SELECTOR_MODE_SAVE_FILE;
  } else if (g_str_equal (method_name, "SaveFiles")) {
    mode = PFS_FILE_SELECTOR_MODE_SAVE_FILES;
  } else {
    mode = PFS_FILE_SELECTOR_MODE_OPEN_FILE;
    g_variant_lookup (arg_options, "multiple", "b", &multiple);
    g_variant_lookup (arg_options, "directory", "b", &directory);
  }

  if (!g_variant_lookup (arg_options, "modal", "b", &modal))
    modal = TRUE;

  if (!g_variant_lookup (arg_options, "accept_label", "&s", &accept_label)) {
    if (g_str_equal (method_name, "OpenFile"))
      accept_label = multiple ? _("_Open") : _("_Select");
    else
      accept_label = _("_Save");
  }

  if (arg_parent_window) {
    external_parent = pmp_external_win_new_from_handle (arg_parent_window);
    if (!external_parent)
      g_warning ("Failed to associate portal window with parent window %s", arg_parent_window);
  }

  file_selector = g_object_new (PFS_TYPE_FILE_SELECTOR,
                                "title", arg_title,
                                "accept-label", accept_label,
                                NULL);
  pfs_file_selector_set_mode (file_selector, mode);
  gtk_window_set_modal (GTK_WINDOW (file_selector), modal);

  handle = g_new0 (FileDialogHandle, 1);
  handle->impl = object;
  handle->invocation = invocation;
  handle->request = g_object_ref (request);
  handle->file_selector = g_object_ref (file_selector);
  handle->mode = mode;
  handle->multiple = multiple;
  handle->external_parent = external_parent;
  handle->allow_write = TRUE;

  g_signal_connect (request, "handle-close", G_CALLBACK (on_handle_close), handle);
  g_signal_connect (file_selector, "done", G_CALLBACK (on_file_selector_done), handle);

  /* File filter */
  filter_pos = GTK_INVALID_LIST_POSITION;
  filters = g_list_store_new (GTK_TYPE_FILE_FILTER);
  g_variant_lookup (arg_options, "current_filter", "@(sa(us))", &current_filter);

  if (g_variant_lookup (arg_options, "filters", "a(sa(us))", &filters_iter)) {
    GVariant *variant;
    guint position = 0;

    while (g_variant_iter_next (filters_iter, "@(sa(us))", &variant)) {
      g_autoptr (GtkFileFilter) filter = gtk_file_filter_new_from_gvariant (variant);

      g_list_store_append (filters, filter);
      if (current_filter != NULL && g_variant_equal (variant, current_filter)) {
        filter_pos = position;

        g_variant_unref (variant);
        position++;
      }
    }
  }

  /* If we just got the current filter select that */
  if (current_filter != NULL && g_list_model_get_n_items (G_LIST_MODEL (filters)) == 0) {
    g_autoptr (GtkFileFilter) filter = gtk_file_filter_new_from_gvariant (current_filter);

    g_list_store_append (filters, filter);
    filter_pos = 0;
  }
  g_object_set (file_selector, "filters", filters, "current-filter", filter_pos, NULL);

  /* Initial dir and filename */
  if (g_str_equal (method_name, "SaveFile")) {
    if (g_variant_lookup (arg_options, "current_file", "^&ay", &path)) {
      g_autoptr (GFile) file = g_file_new_for_path (path);
      g_autoptr (GFile) parent = NULL;
      g_autofree char *filename = g_file_get_basename (file);

      parent = g_file_get_parent (file);

      pfs_file_selector_set_filename (file_selector, filename);
      pfs_file_selector_set_current_directory (file_selector, g_file_get_path (parent));
    } else {
      const char *suggested;

      if (g_variant_lookup (arg_options, "current_name", "&s", &suggested))
        pfs_file_selector_set_filename (file_selector, suggested);

      g_variant_lookup (arg_options, "current_folder", "^&ay", &path);
      pfs_file_selector_set_current_directory (file_selector, path);
    }
  } else {
    if (g_str_equal (method_name, "SaveFiles"))
      g_variant_lookup (arg_options, "files", "^aay", &handle->files);

    g_variant_lookup (arg_options, "current_folder", "^&ay", &path);
    pfs_file_selector_set_current_directory (file_selector, path);
  }

  /* Additional choices */
  choices = g_variant_lookup_value (arg_options, "choices", G_VARIANT_TYPE ("a(ssa(ss)s)"));
  if (choices)
    g_object_set (file_selector, "choices", choices, NULL);

  if (directory)
    g_object_set (file_selector, "directory", TRUE, NULL);

  gtk_window_present (GTK_WINDOW (file_selector));

  if (external_parent) {
    GdkSurface *surface = gtk_native_get_surface (GTK_NATIVE (file_selector));
    pmp_external_win_set_parent_of (external_parent, surface);
  }

  request_export (request, g_dbus_method_invocation_get_connection (invocation));

  return TRUE;
}


gboolean
pmp_file_chooser_init (GDBusConnection *bus, GError **error)
{
  GDBusInterfaceSkeleton *helper;

  pfs_init ();

  helper = G_DBUS_INTERFACE_SKELETON (pmp_impl_file_chooser_skeleton_new ());

  g_object_connect (helper,
                    "signal::handle-open-file", on_handle_open_file, NULL,
                    "signal::handle-save-file", on_handle_open_file, NULL,
                    "signal::handle-save-files", on_handle_open_file, NULL,
                    NULL);

  if (!g_dbus_interface_skeleton_export (helper,
                                         bus,
                                         DESKTOP_PORTAL_OBJECT_PATH,
                                         error))
    return FALSE;

  g_debug ("providing %s", g_dbus_interface_skeleton_get_info (helper)->name);

  return TRUE;
}
