/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use std::env;
use std::sync::atomic::{AtomicBool, Ordering};

use gettextrs::{bind_textdomain_codeset, bindtextdomain, gettext};
use gtk::prelude::*;
use gtk::{gio, glib};

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

    setup_settings();

    gio::resources_register_include_impl(include_bytes!(concat!(
        env!("RESOURCES_DIR"),
        "/",
        "xdg-desktop-portal-phrosh.gresource"
    )))
    .unwrap();

    glib::set_prgname(Some("xdg-desktop-portal-phrosh"));
    glib::set_application_name(&gettext("XDG Desktop Portal Phosh"));

    LIB_INITIALIZED.store(true, Ordering::Release);
}

fn map_high_contrast(variant: &glib::Variant, _type: glib::Type) -> Option<glib::Value> {
    let high_contrast = if <bool>::from_variant(variant)? {
        gtk::InterfaceContrast::More
    } else {
        gtk::InterfaceContrast::NoPreference
    };
    Some(high_contrast.into())
}

fn map_text_scaling_factor(variant: &glib::Variant, _type: glib::Type) -> Option<glib::Value> {
    let xft_dpi = <f64>::from_variant(variant)? * 96.0 * 1024.0;
    #[allow(clippy::cast_possible_truncation)]
    Some((xft_dpi.round() as i32).into())
}

type Schema = (
    &'static str,
    &'static [(
        &'static str,
        &'static str,
        Option<fn(&glib::Variant, glib::Type) -> Option<glib::Value>>,
    )],
);
// Based on https://gitlab.gnome.org/GNOME/libgxdp/-/merge_requests/8.
#[rustfmt::skip]
const SCHEMAS: &[Schema] = &[
    (
        "org.gnome.desktop.a11y",
        &[
            ("always-show-text-caret", "gtk-keynav-use-caret", None)
        ],
    ),
    (
        "org.gnome.desktop.a11y.interface",
        &[
            ("high-contrast", "gtk-interface-contrast", Some(map_high_contrast)),
            ("show-status-shapes", "gtk-show-status-shapes", None),
        ],
    ),
    (
        "org.gnome.desktop.interface",
        &[
            ("cursor-blink", "gtk-cursor-blink", None),
            ("cursor-blink-timeout", "gtk-cursor-blink-timeout", None),
            ("cursor-size", "gtk-cursor-theme-size", None),
            ("cursor-theme", "gtk-cursor-theme-name", None),
            ("enable-animations", "gtk-enable-animations", None),
            ("font-name", "gtk-font-name", None),
            ("icon-theme", "gtk-icon-theme-name", None),
            ("overlay-scrolling", "gtk-overlay-scrolling", None),
            ("text-scaling-factor", "gtk-xft-dpi", Some(map_text_scaling_factor)),
        ],
    ),
    (
        "org.gnome.desktop.peripherals.mouse",
        &[
            ("double-click", "gtk-double-click-time", None),
            ("drag-threshold", "gtk-dnd-drag-threshold", None),
        ],
    ),
    (
        "org.gnome.desktop.privacy",
        &[
            ("recent-files-max-age", "gtk-recent-files-max-age", None),
            ("remember-recent-files", "gtk-recent-files-enabled", None),
        ],
    ),
    (
        "org.gnome.desktop.sound",
        &[
            ("event-sounds", "gtk-enable-event-sounds", None),
            ("input-feedback-sounds", "gtk-enable-input-feedback-sounds", None),
        ],
    ),
    (
        "org.gnome.desktop.wm.preferences",
        &[
            ("action-double-click-titlebar", "gtk-titlebar-double-click", None),
            ("action-middle-click-titlebar", "gtk-titlebar-middle-click", None),
            ("action-right-click-titlebar", "gtk-titlebar-right-click", None),
            ("button-layout", "gtk-decoration-layout", None),
        ],
    ),
];

fn setup_settings() {
    let gtk_settings = gtk::Settings::default().unwrap();

    for (schema, values) in SCHEMAS {
        let settings = gio::Settings::new(schema);
        for (schema_key, gtk_key, mapping) in *values {
            let mut builder = settings.bind(schema_key, &gtk_settings, gtk_key).get_only();
            builder = if let Some(mapping) = mapping {
                builder.mapping(mapping)
            } else {
                builder
            };
            builder.build();
        }
    }

    // NOTE Add this to SCHEMAS once `gsettings-desktop-schemas` package is out of alpha.
    let source = gio::SettingsSchemaSource::default().unwrap();
    let schema = source
        .lookup("org.gnome.desktop.a11y.interface", true)
        .unwrap();
    if schema.has_key("reduced-motion") {
        let settings = gio::Settings::new("org.gnome.desktop.a11y.interface");
        settings
            .bind(
                "reduced-motion",
                &gtk_settings,
                "gtk-interface-reduced-motion",
            )
            .get_only()
            .build();
    }
}
