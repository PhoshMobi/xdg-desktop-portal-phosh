/*
 * Copyright (C) 2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

#include "pt-config.h"

#include <glib/gi18n.h>
#include "application.h"

/**
 * PtService:
 *
 * Uses PtApplication to run the D-Bus service.
 */


int
main (int argc, char *argv[])
{
  g_autoptr (PtApplication) application = NULL;
  int status;

  setlocale (LC_ALL, "");
  bindtextdomain (GETTEXT_PACKAGE, LOCALEDIR);
  bind_textdomain_codeset (GETTEXT_PACKAGE, "UTF-8");
  textdomain (GETTEXT_PACKAGE);

  application = pt_application_new ();
  status = g_application_run (G_APPLICATION (application), argc, argv);

  return status;
}
