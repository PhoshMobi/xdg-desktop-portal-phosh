/*
 * Copyright © 2016 Red Hat, Inc
 *             2023-2024 The Phosh Developers
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

#include <adwaita.h>
#include <gtk/gtk.h>

#include <gio/gio.h>
#include <glib/gi18n.h>

#include "xdg-desktop-portal-dbus.h"

#include "pmp-file-chooser.h"
#include "pmp-request.h"
#include "pmp-settings.h"
#include "pmp-wallpaper.h"

static GMainLoop *loop = NULL;
static GHashTable *outstanding_handles = NULL;

static gboolean opt_verbose;
static gboolean opt_replace;
static gboolean show_version;

static GOptionEntry entries[] = {
  { "verbose", 'v', 0, G_OPTION_ARG_NONE, &opt_verbose,
    "Print debug information during command processing", NULL },
  { "replace", 'r', 0, G_OPTION_ARG_NONE, &opt_replace, "Replace a running instance", NULL },
  { "version", 0, 0, G_OPTION_ARG_NONE, &show_version, "Show program version.", NULL},
  { NULL }
};

static void
message_handler (const gchar   *log_domain,
                 GLogLevelFlags log_level,
                 const gchar   *message,
                 gpointer       user_data)
{
  /* Make this look like normal console output */
  if (log_level & G_LOG_LEVEL_DEBUG)
    printf ("XDP: %s\n", message);
  else
    printf ("%s: %s\n", g_get_prgname (), message);
}

static void
printerr_handler (const gchar *string)
{
  int is_tty = isatty (1);
  const char *prefix = "";
  const char *suffix = "";
  if (is_tty) {
    prefix = "\x1b[31m\x1b[1m";   /* red, bold */
    suffix = "\x1b[22m\x1b[0m";   /* bold off, color reset */
  }
  fprintf (stderr, "%serror: %s%s\n", prefix, suffix, string);
}

static void
on_bus_acquired (GDBusConnection *connection,
                 const gchar     *name,
                 gpointer         user_data)
{
  GError *error = NULL;

  if (!pmp_file_chooser_init (connection, &error)) {
    g_warning ("error: %s\n", error->message);
    g_clear_error (&error);
  }
  if (!pmp_settings_init (connection, &error)) {
    g_warning ("error: %s\n", error->message);
    g_clear_error (&error);
  }
  if (!pmp_wallpaper_init (connection, &error)) {
    g_warning ("error: %s\n", error->message);
    g_clear_error (&error);
  }
}

static void
on_name_acquired (GDBusConnection *connection,
                  const gchar     *name,
                  gpointer         user_data)
{
  g_debug (PMP_DBUS_NAME " acquired");
}

static void
on_name_lost (GDBusConnection *connection,
              const gchar     *name,
              gpointer         user_data)
{
  g_main_loop_quit (loop);
}


static gboolean
init_gtk (GError **error)
{
  /* Avoid pointless and confusing recursion */
#if GTK_CHECK_VERSION (4, 17, 1)
  gtk_disable_portals ();
#else
  g_unsetenv ("GTK_USE_PORTAL");
#endif

  /* Don't let adwaita use portals, we're the one */
  if (!g_setenv ("ADW_DISABLE_PORTAL", "1", TRUE)) {
    g_set_error (error, G_IO_ERROR, g_io_error_from_errno (errno),
                 "Failed to set ADW_DISABLE_PORTAL: %s", g_strerror (errno));
    return FALSE;
  }

  if (!gtk_init_check ()) {
    g_set_error (error, G_IO_ERROR, G_IO_ERROR_FAILED,
                 "Failed to initialize GTK");
    return FALSE;
  }

  return TRUE;
}


int
main (int argc, char *argv[])
{
  guint owner_id;
  g_autoptr (GError) error = NULL;
  g_autoptr (GDBusConnection) session_bus = NULL;
  g_autoptr (GOptionContext) context = NULL;

  setlocale (LC_ALL, "");
  bindtextdomain (GETTEXT_PACKAGE, LOCALEDIR);
  bind_textdomain_codeset (GETTEXT_PACKAGE, "UTF-8");
  textdomain (GETTEXT_PACKAGE);

  context = g_option_context_new ("- portal backends");
  g_option_context_set_summary (context,
                                "A backend implementation for xdg-desktop-portal.");
  g_option_context_set_description (context,
                                    "xdg-desktop-portal-phosh provides D-Bus interfaces that\n"
                                    "are used by xdg-desktop-portal to implement portals\n"
                                    "\n"
                                    "Documentation for the available D-Bus interfaces can be found at\n"
                                    "https://flatpak.github.io/xdg-desktop-portal/docs/\n"
                                    "\n"
                                    "Please report issues at https://gitlab.gnome.org/guidog/xdg-desktop-portal-phosh/issues");
  g_option_context_add_main_entries (context, entries, NULL);
  if (!g_option_context_parse (context, &argc, &argv, &error)) {
    g_printerr ("%s: %s", g_get_application_name (), error->message);
    g_printerr ("\n");
    g_printerr ("Try \"%s --help\" for more information.",
                g_get_prgname ());
    g_printerr ("\n");
    return 1;
  }

  if (show_version) {
    g_print (PACKAGE_STRING "\n");
    return 0;
  }

  if (!init_gtk (&error)) {
    g_printerr ("Failed to init GUI bits: %s", error->message);
    return 1;
  }

  g_set_printerr_handler (printerr_handler);

  if (opt_verbose)
    g_log_set_handler (NULL, G_LOG_LEVEL_DEBUG, message_handler, NULL);

  g_set_prgname ("xdg-desktop-portal-phosh");

  loop = g_main_loop_new (NULL, FALSE);

  outstanding_handles = g_hash_table_new (g_str_hash, g_str_equal);

  session_bus = g_bus_get_sync (G_BUS_TYPE_SESSION, NULL, &error);
  if (session_bus == NULL) {
    g_printerr ("No session bus: %s\n", error->message);
    return 2;
  }

  owner_id = g_bus_own_name (G_BUS_TYPE_SESSION,
                             PMP_DBUS_NAME,
                             G_BUS_NAME_OWNER_FLAGS_ALLOW_REPLACEMENT |
                             (opt_replace ? G_BUS_NAME_OWNER_FLAGS_REPLACE : 0),
                             on_bus_acquired,
                             on_name_acquired,
                             on_name_lost,
                             NULL,
                             NULL);

  adw_init ();
  g_main_loop_run (loop);

  g_bus_unown_name (owner_id);

  return 0;
}
