/*
 * Copyright © 2019 Red Hat, Inc
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Authors:
 *       Matthias Clasen <mclasen@redhat.com>
 */

#pragma once

#include <gio/gio.h>

G_BEGIN_DECLS

typedef void (*ActivateAction) (GDBusConnection *connection,
                                const char      *app_id,
                                const char      *id,
                                const char      *name,
                                GVariant        *parameter,
                                const char      *activation_token,
                                gpointer         data);

void                     pmp_fdo_add_notification                (GDBusConnection *connection,
                                                                  const char      *app_id,
                                                                  const char      *id,
                                                                  GVariant        *notification,
                                                                  ActivateAction   activate,
                                                                  gpointer         data);
gboolean                 pmp_fdo_remove_notification             (GDBusConnection *connection,
                                                                  const char      *app_id,
                                                                  const char      *id);

G_END_DECLS
