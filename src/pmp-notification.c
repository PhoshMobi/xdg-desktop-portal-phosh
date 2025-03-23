/*
 * Copyright (C) 2016 Red Hat, Inc
 *               2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Heavily based on the xdg-desktop-portal-gtk
 *
 * Authors:
 *       Matthias Clasen <mclasen@redhat.com>
 *       Guido Günther <agx@sigxcpu.org>
 */

#define _GNU_SOURCE 1

#include "pmp-config.h"

#include <errno.h>
#include <locale.h>
#include <string.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>

#include <gtk/gtk.h>

#include <gio/gio.h>
#include <gio/gdesktopappinfo.h>
#include <gio/gunixfdlist.h>

#include "xdg-desktop-portal-dbus.h"

#include "pmp-notification.h"
#include "pmp-fdo-notification.h"
#include "pmp-request.h"
#include "pmp-utils.h"


static char *
app_path_for_id (const char *app_id)
{
  char *path;
  gint i;

  path = g_strconcat ("/", app_id, NULL);
  for (i = 0; path[i]; i++) {
    if (path[i] == '.')
      path[i] = '/';
    if (path[i] == '-')
      path[i] = '_';
  }

  return path;
}


static void
activate_action (GDBusConnection *connection,
                 const char      *app_id,
                 const char      *id,
                 const char      *name,
                 GVariant        *parameter,
                 const char      *activation_token)
{
  g_autofree char *object_path = NULL;
  GVariantBuilder pdata, parms;

  object_path = app_path_for_id (app_id);
  g_variant_builder_init (&pdata, G_VARIANT_TYPE_VARDICT);
  g_variant_builder_init (&parms, G_VARIANT_TYPE ("av"));
  if (parameter)
    g_variant_builder_add (&parms, "v", parameter);

  if (activation_token) {
    /* Used by  `GTK` < 4.10 */
    g_variant_builder_add (&pdata, "{sv}",
                           "desktop-startup-id", g_variant_new_string (activation_token));
    /* Used by `GTK` and `QT` */
    g_variant_builder_add (&pdata, "{sv}",
                           "activation-token", g_variant_new_string (activation_token));
  }

  if (name && g_str_has_prefix (name, "app.")) {
    g_dbus_connection_call (connection,
                            app_id,
                            object_path,
                            "org.freedesktop.Application",
                            "ActivateAction",
                            g_variant_new ("(s@av@a{sv})",
                                           name + 4,
                                           g_variant_builder_end (&parms),
                                           g_variant_builder_end (&pdata)),
                            NULL,
                            G_DBUS_CALL_FLAGS_NONE,
                            -1, NULL, NULL, NULL);
  } else {
    g_autoptr (GVariant) ret = NULL;

    g_dbus_connection_call (connection,
                            app_id,
                            object_path,
                            "org.freedesktop.Application",
                            "Activate",
                            g_variant_new ("(@a{sv})",
                                           g_variant_builder_end (&pdata)),
                            NULL,
                            G_DBUS_CALL_FLAGS_NONE,
                            -1, NULL, NULL, NULL);

    g_dbus_connection_emit_signal (connection,
                                   NULL,
                                   "/org/freedesktop/portal/desktop",
                                   "org.freedesktop.impl.portal.Notification",
                                   "ActionInvoked",
                                   g_variant_new ("(sss@av)",
                                                  app_id, id, name,
                                                  g_variant_builder_end (&parms)),
                                   NULL);
  }
}


static gboolean
handle_add_notification (PmpImplNotification   *object,
                         GDBusMethodInvocation *invocation,
                         GUnixFDList           *fds G_GNUC_UNUSED,
                         const char            *arg_app_id,
                         const char            *arg_id,
                         GVariant              *arg_notification)
{
  GDBusConnection *connection;

  connection = g_dbus_method_invocation_get_connection (invocation);

  pmp_fdo_add_notification (connection,
                            arg_app_id,
                            arg_id,
                            arg_notification,
                            activate_action);

  pmp_impl_notification_complete_add_notification (object, invocation, NULL);

  return TRUE;
}


static gboolean
handle_remove_notification (PmpImplNotification   *object,
                            GDBusMethodInvocation *invocation,
                            const char            *arg_app_id,
                            const char            *arg_id)
{
  GDBusConnection *connection;

  connection = g_dbus_method_invocation_get_connection (invocation);

  pmp_fdo_remove_notification (connection, arg_app_id, arg_id);

  pmp_impl_notification_complete_remove_notification (object, invocation);

  return TRUE;
}


static GVariant *
build_options (void)
{
  GVariantBuilder options_builder;
  const char *const categories[] = {
    "im.received",
    "call.unanswered",
    NULL,
  };

  g_variant_builder_init (&options_builder, G_VARIANT_TYPE ("a{sv}"));
  g_variant_builder_add (&options_builder,
                         "{sv}",
                         "category",
                         g_variant_new_strv (categories, -1));

  return g_variant_builder_end (&options_builder);
}


gboolean
pmp_notification_init (GDBusConnection *bus, GError **error)
{
  GDBusInterfaceSkeleton *helper;
  GVariant *options = build_options ();

  helper = G_DBUS_INTERFACE_SKELETON (pmp_impl_notification_skeleton_new ());
  g_object_connect (helper,
                    "signal::handle-add-notification", handle_add_notification, NULL,
                    "signal::handle-remove-notification", handle_remove_notification, NULL,
                    NULL);

  pmp_impl_notification_set_version (PMP_IMPL_NOTIFICATION (helper), 2);
  pmp_impl_notification_set_supported_options (PMP_IMPL_NOTIFICATION (helper), options);

  if (!g_dbus_interface_skeleton_export (helper,
                                         bus,
                                         DESKTOP_PORTAL_OBJECT_PATH,
                                         error))
    return FALSE;

  g_debug ("providing %s", g_dbus_interface_skeleton_get_info (helper)->name);

  return TRUE;
}
