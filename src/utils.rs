/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use ashpd::WindowIdentifierType;
use gettextrs::gettext;
use gtk::glib;
use gtk::prelude::*;

use crate::Application;

/*
 * Utility functions that are used in more than one place.
 */

const LOG_DOMAIN: &str = "xdpp-utils";

// Thanks to Pika Backup.
// https://gitlab.gnome.org/World/pika-backup/-/blob/81a9b0eefbd5099296b1655cc7a7eb8849153795/src/prelude.rs#L15
#[must_use]
pub fn gettextf(format: &str, args: &[&str]) -> String {
    let mut s = gettext(format);

    for arg in args {
        s = s.replacen("{}", arg, 1);
    }
    s
}

#[must_use]
pub fn get_application_name(application: &Application) -> Option<String> {
    let app_id = application.app_id.as_ref()?;
    let app_info = gio_unix::DesktopAppInfo::new(&format!("{app_id}.desktop"))?;
    let app_name = app_info.display_name().to_string();
    Some(app_name)
}

// Present the window and set it transient to given parent.
// Window must be presented first to avoid Phoc not maximizing it.
pub fn present_and_set_transient(
    window: &impl gtk::prelude::IsA<gtk::Window>,
    parent: Option<WindowIdentifierType>,
) {
    window.present();
    if let Some(identifier) = parent {
        identifier.set_parent_of(window);
    } else {
        glib::g_warning!(LOG_DOMAIN, "Application does not have window identifier");
    }
}
