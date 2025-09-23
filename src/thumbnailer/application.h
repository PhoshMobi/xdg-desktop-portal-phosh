/*
 * Copyright (C) 2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#pragma once

#include <gio/gio.h>

#define PT_TYPE_APPLICATION pt_application_get_type ()
G_DECLARE_FINAL_TYPE (PtApplication, pt_application, PT, APPLICATION, GApplication)

PtApplication *pt_application_new (void);
