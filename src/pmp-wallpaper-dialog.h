/*
 * Copyright © 2019 Red Hat, Inc
 *             2023-2024 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Heavily based on the xdg-desktop-portal-gnome
 *
 * Authors:
 *       Felipe Borges <feborges@redhat.com>
 *       Guido Günther <agx@sigxcpu.org>
 */

#pragma once

#include <adwaita.h>

G_BEGIN_DECLS

#define PMP_WALLPAPER_TYPE_DIALOG (pmp_wallpaper_dialog_get_type ())

G_DECLARE_FINAL_TYPE (PmpWallpaperDialog, pmp_wallpaper_dialog, PMP, WALLPAPER_DIALOG, AdwWindow);

PmpWallpaperDialog *pmp_wallpaper_dialog_new (const char *picture_uri,
                                              const char *app_id,
                                              gboolean    lockscreen);
const gchar        *pmp_wallpaper_dialog_get_uri (PmpWallpaperDialog *dialog);

G_END_DECLS
