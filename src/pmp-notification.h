/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Authors: Guido Günther <agx@sigxcpu.org>
 */

#pragma once

#include <gio/gio.h>

G_BEGIN_DECLS

gboolean                pmp_notification_init                    (GDBusConnection *bus,
                                                                  GError         **error);

G_END_DECLS
