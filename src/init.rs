/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use std::env;
use std::sync::atomic::{AtomicBool, Ordering};

use gettextrs::{bind_textdomain_codeset, bindtextdomain};
use gtk::gio;

use crate::lib_config::{GETTEXT_PACKAGE, LOCALE_DIR};

/*
 * The entry-point to the backend library.
 *
 * The `init` function initializes the library. It disables portals, initializes Adwaita, sets up
 * the `gettext` domain and registers resources.
 */

static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn init() {
    if INITIALIZED.load(Ordering::Acquire) {
        return;
    }

    gtk::disable_portals();

    unsafe {
        env::set_var("ADW_DISABLE_PORTAL", "1");
    }

    adw::init().unwrap();

    bindtextdomain(GETTEXT_PACKAGE, LOCALE_DIR).unwrap();
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8").unwrap();

    gio::resources_register_include_impl(include_bytes!(concat!(
        env!("RESOURCES_DIR"),
        "/",
        "xdg-desktop-portal-phrosh.gresource"
    )))
    .unwrap();

    INITIALIZED.store(true, Ordering::Release);
}
