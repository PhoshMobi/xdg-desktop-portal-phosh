/*
 * Copyright (C) 2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

#define G_LOG_DOMAIN "pts"

#include "pt-config.h"

#include "phosh-thumbnailer-service.h"

#include <gio/gio.h>
#include <glib/gi18n.h>

/**
 * PtCli:
 *
 * Provides a CLI interface to interact with Phosh Thumbnailer Service.
 */


static gboolean
stop_thumbnailing (void)
{
  g_autoptr (PtImplThumbnailer) proxy = NULL;
  g_autoptr (GError) error = NULL;

  proxy = pt_impl_thumbnailer_proxy_new_for_bus_sync (G_BUS_TYPE_SESSION, G_DBUS_PROXY_FLAGS_NONE,
                                                      PT_SERVICE_DBUS_NAME, PT_SERVICE_OBJECT_PATH,
                                                      NULL, &error);
  if (!proxy) {
    g_printerr ("Failed to create proxy: %s\n", error->message);
    return FALSE;
  }

  if (!pt_impl_thumbnailer_call_stop_thumbnailing_sync (proxy, g_variant_new ("a{sv}", NULL), NULL,
                                                        &error)) {
    g_printerr ("Failed to stop thumbnailing: %s\n", error->message);
    return FALSE;
  }

  return TRUE;
}


static gboolean
thumbnail_directory_or_files (GStrv files)
{
  g_autoptr (PtImplThumbnailer) proxy = NULL;
  g_autoptr (GError) error = NULL;
  uint files_count = 0;
  g_autofree char *directory_uri = NULL;
  g_autoptr (GStrvBuilder) builder = g_strv_builder_new ();

  proxy = pt_impl_thumbnailer_proxy_new_for_bus_sync (G_BUS_TYPE_SESSION, G_DBUS_PROXY_FLAGS_NONE,
                                                      PT_SERVICE_DBUS_NAME, PT_SERVICE_OBJECT_PATH,
                                                      NULL, &error);
  if (!proxy) {
    g_printerr ("Failed to create proxy: %s\n", error->message);
    return FALSE;
  }

  for (uint i = 0; files[i]; i++) {
    g_autoptr (GFile) file = g_file_new_for_path (files[i]);
    g_autofree char *uri = g_file_get_uri (file);

    if (g_file_query_file_type (file, G_FILE_QUERY_INFO_NONE, NULL) == G_FILE_TYPE_DIRECTORY) {
      if (directory_uri) {
        g_printerr ("Expected either a directory or list of files "
                    "but got more than one directory\n");
        return FALSE;
      }
      directory_uri = g_strdup (uri);
    } else {
      files_count += 1;
      g_strv_builder_add (builder, uri);
    }
  }

  if (directory_uri && files_count != 0) {
    g_printerr ("Expected either a directory or list of files but got both\n");
    return FALSE;
  }

  if (directory_uri) {
    if (!pt_impl_thumbnailer_call_thumbnail_directory_sync (proxy, directory_uri,
                                                            g_variant_new ("a{sv}", NULL), NULL,
                                                            &error)) {
      g_printerr ("Failed to thumbnail directory: %s\n", error->message);
      return FALSE;
    }
  } else {
    g_auto (GStrv) arg_files = g_strv_builder_end (builder);
    if (!pt_impl_thumbnailer_call_thumbnail_files_sync (proxy, (const char *const *) arg_files,
                                                        g_variant_new ("a{sv}", NULL), NULL,
                                                        &error)) {
      g_printerr ("Failed to thumbnail files: %s\n", error->message);
      return FALSE;
    }
  }

  return TRUE;
}


int
main (int argc, char *argv[])
{
  g_autoptr (GError) error = NULL;
  g_autoptr (GOptionContext) context = NULL;
  g_auto (GStrv) files = NULL;
  gboolean stop = FALSE;
  gboolean show_version = FALSE;
  gboolean success;

  GOptionEntry entries[] = {
    { "stop", 's', 0, G_OPTION_ARG_NONE, &stop, "Stop on-going thumbnailing and exit.", NULL },
    { "version", 'v', 0, G_OPTION_ARG_NONE, &show_version, "Print version and exit.", NULL },
    { G_OPTION_REMAINING, 0, 0, G_OPTION_ARG_FILENAME_ARRAY, &files, "Files to thumbnail.", NULL },
    { NULL }
  };

  setlocale (LC_ALL, "");
  bindtextdomain (GETTEXT_PACKAGE, LOCALEDIR);
  bind_textdomain_codeset (GETTEXT_PACKAGE, "UTF-8");
  textdomain (GETTEXT_PACKAGE);

  context = g_option_context_new ("[FILE…]");
  g_option_context_set_summary (context, "A CLI to interact with Phosh Thumbnailer Service.");
  g_option_context_set_description (context,
                                    "This utility can be used to thumbnail all files in a "
                                    "directory, thumbnail all files provided as arguments or stop "
                                    "the on-going thumbnailing operation in the service.\n\n"
                                    "Please report issues at https://gitlab.gnome.org/World/Phosh/xdg-desktop-portal-phosh/-/issues.");
  g_option_context_add_main_entries (context, entries, NULL);

  if (!g_option_context_parse (context, &argc, &argv, &error)) {
    g_printerr ("%s: %s", g_get_application_name (), error->message);
    g_printerr ("\n");
    g_printerr ("Try \"%s --help\" for more information.", g_get_prgname ());
    g_printerr ("\n");
    return EXIT_FAILURE;
  }

  if (show_version) {
    g_print (PT_VERSION "\n");
    return EXIT_SUCCESS;
  }

  if (!files && !stop) {
    g_printerr ("%s: %s", g_get_application_name (),
                "a directory or at least one file must be provided.");
    g_printerr ("\n");
    g_printerr ("Try \"%s --help\" for more information.", g_get_prgname ());
    g_printerr ("\n");
    return EXIT_FAILURE;
  }

  if (stop)
    success = stop_thumbnailing ();
  else
    success = thumbnail_directory_or_files (files);

  return success ? EXIT_SUCCESS : EXIT_FAILURE;
}
