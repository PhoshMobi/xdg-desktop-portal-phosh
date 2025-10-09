/*
 * Copyright (C) 2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use gtk::glib;

/// Different styles of displaying a wallpaper.
///
/// Duplicate of [GNOME's `GDesktopBackgroundStyle`](https://gitlab.gnome.org/GNOME/gsettings-desktop-schemas/-/blob/main/headers/gdesktop-enums.h).
#[derive(Clone, Copy, Debug, Default, PartialEq, glib::Enum)]
#[enum_type(name = "GDesktopBackgroundStyle")]
pub enum DesktopBackgroundStyle {
    #[default]
    #[enum_value(nick = "none")]
    None = 0,
    #[enum_value(nick = "wallpaper")]
    Wallpaper,
    #[enum_value(nick = "centered")]
    Centered,
    #[enum_value(nick = "scaled")]
    Scaled,
    #[enum_value(nick = "stretched")]
    Stretched,
    #[enum_value(nick = "zoom")]
    Zoom,
    #[enum_value(nick = "spanned")]
    Spanned,
}
