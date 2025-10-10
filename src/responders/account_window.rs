/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use std::cell::{Cell, RefCell};

use adw::prelude::*;
use adw::subclass::prelude::*;
use ashpd::backend::Result;
use ashpd::desktop::account::UserInformation;
use ashpd::url::Url;
use ashpd::PortalError;
use gtk::glib::subclass::InitializingObject;
use gtk::{gdk, gio, glib, CompositeTemplate, TemplateChild};
use tokio::sync::oneshot::Sender;

use crate::utils::{get_application_name, gettextf};
use crate::{Request, Responder};

/*
 * `AccountWindow` handles the Account interface. It shows a dialog which displays the information
 * of user from the system environment. The user can change it as per their requirement and agree to
 * share it with the requesting application. The default profile picture of user is loaded as
 * `$HOME/.face`.
 */

const LOG_DOMAIN: &str = "xdpp-account-window";

const FACE_FILE: &str = ".face";

mod imp {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/mobi/phosh/xdpp/ui/account_window.ui")]
    pub struct AccountWindow {
        #[template_child]
        pub avatar: TemplateChild<adw::Avatar>,
        #[template_child]
        pub del_btn: TemplateChild<gtk::Button>,
        #[template_child]
        pub file_dialog: TemplateChild<gtk::FileDialog>,
        #[template_child]
        pub name_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub desc_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub reason_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub username_row: TemplateChild<adw::EntryRow>,

        pub cancellable: RefCell<gio::Cancellable>,

        pub sender: Cell<Option<Sender<Result<UserInformation>>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountWindow {
        const NAME: &'static str = "XdppAccountWindow";
        type Type = super::AccountWindow;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountWindow {
        fn constructed(&self) {
            self.parent_constructed();
            *self.cancellable.borrow_mut() = gio::Cancellable::new();
        }

        fn dispose(&self) {
            self.cancellable.borrow().cancel();
        }
    }

    impl WidgetImpl for AccountWindow {}

    impl WindowImpl for AccountWindow {}

    impl AdwWindowImpl for AccountWindow {}

    #[gtk::template_callbacks]
    impl AccountWindow {
        #[template_callback]
        fn on_cancel_clicked(&self, _button: &gtk::Button) {
            self.cancellable.borrow().cancel();
            let error = PortalError::Cancelled(String::from("Cancelled by user"));
            self.send_response(Err(error));
        }

        #[template_callback]
        fn on_share_clicked(&self, _button: &gtk::Button) {
            let texture = self.avatar.draw_to_texture(self.avatar.scale_factor());
            let (file, _) = gio::File::new_tmp(Some("XXXXXX-profile-picture.png")).unwrap();
            texture.save_to_png(file.path().unwrap()).unwrap();
            let info = UserInformation::new(
                &self.username_row.text(),
                &self.name_row.text(),
                Url::parse(&file.uri()).unwrap(),
            );
            self.send_response(Ok(info));
        }

        #[template_callback]
        fn on_del_avatar_clicked(&self, _button: &gtk::Button) {
            self.avatar.set_custom_image(gdk::Paintable::NONE);
            self.del_btn.set_visible(false);
        }

        pub fn load_avatar_from_file(&self, file: gio::File) {
            let texture = gdk::Texture::from_file(&file).ok();
            self.avatar.set_custom_image(texture.as_ref());
            self.del_btn.set_visible(texture.is_some());
        }

        #[template_callback]
        fn on_edit_avatar_clicked(&self, _button: &gtk::Button) {
            self.file_dialog.open(
                Some(&*self.obj()),
                Some(&*self.cancellable.borrow()),
                glib::clone!(
                    #[weak(rename_to = this)]
                    self,
                    move |result| {
                        if result.is_err() {
                            return;
                        }

                        let file = result.unwrap();
                        this.load_avatar_from_file(file);
                    },
                ),
            );
        }

        fn send_response(&self, response: Result<UserInformation>) {
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
    }
}

glib::wrapper! {
    pub struct AccountWindow(ObjectSubclass<imp::AccountWindow>)
        @extends adw::Window, gtk::Window, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl AccountWindow {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}

impl Responder for AccountWindow {
    fn respond(&self, request: Request) {
        if let Request::AccountGetUserInformation {
            application,
            options,
            sender,
        } = request
        {
            let imp = self.imp();

            let mut home = glib::home_dir();
            home.push(FACE_FILE);
            imp.load_avatar_from_file(gio::File::for_path(home.as_path()));
            imp.avatar.set_text(glib::real_name().as_os_str().to_str());

            let app_name = get_application_name(&application);
            let desc = match app_name {
                Some(app_name) => gettextf("{} requests your information.", &[&app_name]),
                None => gettextf("An app requests your information.", &[]),
            };
            imp.desc_row.set_subtitle(desc.as_str());

            let reason = options.reason().unwrap_or_default();
            if reason.is_empty() {
                imp.reason_row.set_visible(false);
            }
            imp.reason_row.set_subtitle(reason);

            imp.username_row
                .set_text(glib::user_name().as_os_str().to_str().unwrap());
            imp.name_row
                .set_text(glib::real_name().as_os_str().to_str().unwrap());

            imp.sender.set(Some(sender));

            if let Some(identifier) = application.window_identifier {
                identifier.set_parent_of(self);
            } else {
                glib::g_warning!(LOG_DOMAIN, "Application does not have window identifier");
            }

            self.present();
        } else {
            glib::g_critical!(LOG_DOMAIN, "Unknown request {request:#?}");
            panic!();
        }
    }

    fn cancel(&self) {
        self.close();
    }
}
