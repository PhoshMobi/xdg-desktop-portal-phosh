/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use std::cell::{Cell, RefCell};
use std::ffi::OsStr;
use std::str::FromStr;

use adw::prelude::*;
use adw::subclass::prelude::*;
use ashpd::backend::app_chooser::{Choice, DesktopID};
use ashpd::backend::Result;
use ashpd::{AppID, PortalError};
use gtk::glib::subclass::InitializingObject;
use gtk::{gio, glib, CompositeTemplate, TemplateChild};
use tokio::sync::oneshot::Sender;

use super::AppChooserRow;
use crate::utils::gettextf;
use crate::{Request, Responder};

/*
 * `AppChooserWindow` handles the AppChooser interface. It shows a dialog which displays the list of
 * appplications that can open the given URI. Users are also given option to launch Software to
 * search for a better application.
 */

const LOG_DOMAIN: &str = "xdpp-app-chooser-window";

const GNOME_SOFTWARE: &str = "gnome-software";
const MAX_LOCATION_LENGTH: usize = 100;

fn ellipsize_middle(text: &str, length: usize) -> String {
    if text.len() <= length {
        return text.to_string();
    }

    let half = length / 2;
    format!("{}…{}", &text[..half], &text[text.len() - half..])
}

mod imp {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/mobi/phosh/xdpp/ui/app_chooser_window.ui")]
    pub struct AppChooserWindow {
        #[template_child]
        pub open_but: TemplateChild<gtk::Button>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub prefs_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,

        pub last_choice: RefCell<String>,
        pub content_type: RefCell<Option<String>>,

        pub sender: Cell<Option<Sender<Result<Choice>>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppChooserWindow {
        const NAME: &'static str = "XdppAppChooserWindow";
        type Type = super::AppChooserWindow;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AppChooserWindow {}

    impl WidgetImpl for AppChooserWindow {}

    impl WindowImpl for AppChooserWindow {}

    impl AdwWindowImpl for AppChooserWindow {}

    #[gtk::template_callbacks]
    impl AppChooserWindow {
        #[template_callback]
        fn on_cancel_clicked(&self, _button: &gtk::Button) {
            let error = PortalError::Cancelled(String::from("Cancelled by user"));
            self.send_response(Err(error));
        }

        #[template_callback]
        fn on_open_clicked(&self, _button: &gtk::Button) {
            self.send_app_id()
        }

        #[template_callback]
        fn on_row_activated(&self, _row: &gtk::ListBoxRow, _list_box: &gtk::ListBox) {
            self.send_app_id()
        }

        #[template_callback]
        fn on_row_selected(&self, row: Option<&gtk::ListBoxRow>, _list_box: &gtk::ListBox) {
            self.open_but.set_sensitive(row.is_some());
        }

        #[template_callback]
        fn on_open_software_clicked(&self, _button: &gtk::Button) {
            let mut args: Vec<&OsStr> = vec![&OsStr::new(GNOME_SOFTWARE)];
            let content_type = self.content_type.borrow();
            let search_term = format!("--search={}", content_type.as_ref().map_or("", |v| v));
            if content_type.is_some() {
                args.push(OsStr::new(&search_term));
            } else {
                args.push(OsStr::new("--mode=overview"));
            }

            if let Err(error) = gio::Subprocess::newv(&args[..], gio::SubprocessFlags::NONE) {
                let dialog = adw::AlertDialog::new(
                    Some(&gettextf("Failed to launch GNOME Software", &[])),
                    Some(error.message()),
                );
                dialog.add_response("close", &gettextf("Close", &[]));
                dialog.present(Some(self.obj().as_ref()));
            }
        }

        fn send_app_id(&self) {
            let row = self.list_box.selected_row();
            if row.is_none() {
                glib::g_critical!(LOG_DOMAIN, "Trying to send app-id when no row is selected");
                return;
            }
            let row = row.unwrap();

            let app_id_str = row.dynamic_cast_ref::<AppChooserRow>().unwrap().app_id();
            let app_id = AppID::from_str(&app_id_str);

            if let Ok(app_id) = app_id {
                let choice = Choice::new(app_id);
                self.send_response(Ok(choice));
            } else {
                glib::g_critical!(LOG_DOMAIN, "Invalid app-id `{app_id_str}` on selected row");
                let error = PortalError::Failed(String::from("Internal error"));
                self.send_response(Err(error));
            }
        }

        fn send_response(&self, response: Result<Choice>) {
            let sender = self.sender.take();
            if let Some(sender) = sender {
                if sender.send(response).is_err() {
                    glib::g_critical!(LOG_DOMAIN, "Unable to send response through sender");
                }
            } else {
                glib::g_critical!(LOG_DOMAIN, "Sender is not available");
            }
            self.obj().close();
        }

        pub fn update_choices(&self, choices: Vec<DesktopID>) {
            self.list_box.remove_all();

            let last_app_id = self.last_choice.borrow();

            if !last_app_id.is_empty() {
                let row = AppChooserRow::from_app_id(&last_app_id);
                self.list_box.append(&row);
            }

            for desktop_id in choices {
                let app_id = desktop_id.to_string();
                if *last_app_id == app_id {
                    continue;
                }
                let row = AppChooserRow::from_app_id(&app_id);
                self.list_box.append(&row);
            }

            let page_name = if let Some(row) = self.list_box.row_at_index(0) {
                self.list_box.select_row(Some(&row));
                "list"
            } else {
                self.open_but.set_sensitive(false);
                "empty"
            };
            self.stack.set_visible_child_name(page_name);
        }
    }
}

glib::wrapper! {
    pub struct AppChooserWindow(ObjectSubclass<imp::AppChooserWindow>)
        @extends adw::Window, gtk::Window, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl AppChooserWindow {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}

impl Responder for AppChooserWindow {
    fn respond(&self, request: Request) {
        if let Request::AppChooserChooseApplication {
            application,
            choices,
            options,
            sender,
        } = request
        {
            let imp = self.imp();

            let uri = options.uri();
            let filename = options.filename();
            let prefs_desc;
            let status_desc;
            if let Some(filename) = filename {
                let target = ellipsize_middle(filename, MAX_LOCATION_LENGTH);
                prefs_desc = gettextf("Choose an application to open {}.", &[&target]);
                status_desc = gettextf("No application found to open {}, but you can search Software to find suitable applications.", &[&target]);
            } else if let Some(uri) = uri {
                let target = ellipsize_middle(uri.as_ref(), MAX_LOCATION_LENGTH);
                prefs_desc = gettextf("Choose an application to open the URI {}.", &[&target]);
                status_desc = gettextf("No application found to open the URI {}, but you can search Software to find suitable applications.", &[&target]);
            } else {
                let error = PortalError::InvalidArgument(String::from(
                    "Either filename or URI must be provided",
                ));
                if sender.send(Err(error)).is_err() {
                    glib::g_critical!(LOG_DOMAIN, "Unable to send response through sender");
                };
                return;
            }
            imp.prefs_group.set_description(Some(&prefs_desc));
            imp.status_page.set_description(Some(&status_desc));

            *imp.last_choice.borrow_mut() = if let Some(desktop_id) = options.last_choice() {
                desktop_id.to_string()
            } else {
                String::new()
            };
            *imp.content_type.borrow_mut() = options.content_type().map(String::from);
            imp.update_choices(choices);
            imp.sender.set(Some(sender));

            if let Some(identifier) = application.window_identifier {
                identifier.set_parent_of(self);
            }
            self.set_modal(options.modal().unwrap_or(false));

            self.present();
        } else if let Request::AppChooserUpdateChoices { choices, sender } = request {
            let imp = self.imp();
            imp.update_choices(choices);
            if sender.send(Ok(())).is_err() {
                glib::g_critical!(LOG_DOMAIN, "Unable to send response through sender");
            };
        } else {
            glib::g_critical!(LOG_DOMAIN, "Unknown request {request:#?}");
            panic!();
        }
    }

    fn cancel(&self) {
        self.close();
    }
}
