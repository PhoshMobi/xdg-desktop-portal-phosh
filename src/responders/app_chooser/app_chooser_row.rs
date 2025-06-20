/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::gio;
use gtk::glib;
use gtk::glib::subclass::*;
use gtk::glib::Properties;
use gtk::{CompositeTemplate, TemplateChild};
use std::cell::RefCell;

/*
 * `AppChooserRow` is used by `AppChooserWindow` to display an application representing given
 * `AppID`.
 */

const LOG_DOMAIN: &str = "xdpp-app-chooser-row";

mod imp {
    use super::*;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::AppChooserRow)]
    #[template(resource = "/mobi/phosh/xdpp/ui/app_chooser_row.ui")]
    pub struct AppChooserRow {
        #[property(construct_only, get, set=Self::set_app_id)]
        app_id: RefCell<String>,

        #[template_child]
        image: TemplateChild<gtk::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppChooserRow {
        const NAME: &'static str = "XdppAppChooserRow";
        type Type = super::AppChooserRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AppChooserRow {}

    impl WidgetImpl for AppChooserRow {}

    impl ListBoxRowImpl for AppChooserRow {}

    impl PreferencesRowImpl for AppChooserRow {}

    impl ActionRowImpl for AppChooserRow {}

    impl AppChooserRow {
        fn set_app_id(&self, app_id: String) {
            if app_id.is_empty() {
                glib::g_critical!(LOG_DOMAIN, "app-id is empty");
                return;
            }

            let info = gio::DesktopAppInfo::new(&format!("{app_id}.desktop"));
            if info.is_none() {
                glib::g_critical!(LOG_DOMAIN, "app-id `{app_id}` has no app-info");
                return;
            }
            let info = info.unwrap();

            let name = info.display_name();
            self.obj().set_title(&name);

            let icon = info.icon();
            if icon.is_some() {
                let icon = icon.unwrap();
                self.image.set_from_gicon(&icon);
            }

            *self.app_id.borrow_mut() = app_id;
        }
    }
}

glib::wrapper! {
    pub struct AppChooserRow(ObjectSubclass<imp::AppChooserRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl AppChooserRow {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}

impl Default for AppChooserRow {
    fn default() -> Self {
        Self::new()
    }
}

impl AppChooserRow {
    pub fn from_app_id(app_id: &str) -> Self {
        glib::Object::builder().property("app-id", app_id).build()
    }
}
