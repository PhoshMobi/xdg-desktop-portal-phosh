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
 *
 * `i18n_init` can be used to exclusively set up the `gettext` domain.
 */

static LIB_INITIALIZED: AtomicBool = AtomicBool::new(false);
static I18N_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[allow(clippy::missing_panics_doc)]
pub fn i18n_init() {
    if I18N_INITIALIZED.load(Ordering::Acquire) {
        return;
    }

    bindtextdomain(GETTEXT_PACKAGE, LOCALE_DIR).unwrap();
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8").unwrap();

    I18N_INITIALIZED.store(true, Ordering::Release);
}

#[allow(clippy::missing_panics_doc)]
pub fn init() {
    if LIB_INITIALIZED.load(Ordering::Acquire) {
        return;
    }

    i18n_init();

    gtk::disable_portals();

    unsafe {
        env::set_var("ADW_DISABLE_PORTAL", "1");
    }

    adw::init().unwrap();

    gio::resources_register_include_impl(include_bytes!(concat!(
        env!("RESOURCES_DIR"),
        "/",
        "xdg-desktop-portal-phrosh.gresource"
    )))
    .unwrap();

    LIB_INITIALIZED.store(true, Ordering::Release);
}
