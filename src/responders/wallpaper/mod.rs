/*
 * Copyright (C) 2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

mod desktop_background_style;
pub(super) mod transformations;
mod wallpaper_preview;
mod wallpaper_window;

pub(super) use desktop_background_style::DesktopBackgroundStyle;
pub(super) use wallpaper_preview::WallpaperPreview;
pub use wallpaper_window::WallpaperWindow;
