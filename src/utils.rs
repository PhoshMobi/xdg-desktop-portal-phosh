/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use crate::Application;
use gettextrs::gettext;
use gio::prelude::*;
use gtk::gio;

/*
 * Utility functions that are used in more than one place.
 */

// Thanks to Pika Backup.
// https://gitlab.gnome.org/World/pika-backup/-/blob/81a9b0eefbd5099296b1655cc7a7eb8849153795/src/prelude.rs#L15
pub fn gettextf(format: &str, args: &[&str]) -> String {
    let mut s = gettext(format);

    for arg in args {
        s = s.replacen("{}", arg, 1);
    }
    s
}

pub fn get_application_name(application: &Application) -> Option<String> {
    let app_id = application.app_id.as_ref()?;
    let app_info = app_id.app_info()?;
    let app_name = app_info.display_name().to_string();
    Some(app_name)
}
