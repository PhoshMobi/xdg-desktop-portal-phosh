/*
 * Copyright (C) 2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use std::cell::{Cell, RefCell};

use adw::prelude::*;
use adw::subclass::prelude::*;
use ashpd::backend::Result;
use ashpd::desktop::wallpaper::SetOn;
use ashpd::PortalError;
use gtk::glib::subclass::InitializingObject;
use gtk::{gdk, gio, glib, CompositeTemplate, TemplateChild};
use tokio::sync::oneshot::Sender;

use super::WallpaperPreview;
use crate::lib_config::PORTAL_NAME;
use crate::utils::{get_application_name, gettextf, present_and_set_transient};
use crate::{Request, Responder};

/*
 * `WallpaperWindow` handles the Wallpaper interface. It lets user preview the wallpaper on home and
 * lockscreen. If accepted, the image is copied to `XDG_CONFIG_HOME/PORTAL_NAME/wallpaper_home` for
 * home and `XDG_CONFIG_HOME/PORTAL_NAME/wallpaper_lockscreen` for lockscreen. If the wallpaper is
 * required for both home and lockscreen, then it is copied to
 * `XDG_CONFIG_HOME/PORTAL_NAME/wallpaper_background`.
 */

const LOG_DOMAIN: &str = "xdpp-wallpaper-window";

const BACKGROUND_FILENAME: &str = "wallpaper_background";
const HOME_FILENAME: &str = "wallpaper_home";
const LOCKSCREEN_FILENAME: &str = "wallpaper_lockscreen";

const HOME_SCHEMA: &str = "org.gnome.desktop.background";
const LOCKSCREEN_SCHEMA: &str = "org.gnome.desktop.screensaver";
const PICTURE_URI_NORMAL_KEY: &str = "picture-uri";
const PICTURE_URI_DARK_KEY: &str = "picture-uri-dark";

mod imp {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/mobi/phosh/xdpp/ui/wallpaper_window.ui")]
    pub struct WallpaperWindow {
        #[template_child]
        pub carousel: TemplateChild<adw::Carousel>,
        #[template_child]
        pub desc_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub indicator: TemplateChild<adw::CarouselIndicatorLines>,
        #[template_child]
        pub set_on_row: TemplateChild<adw::ComboRow>,

        pub cancellable: RefCell<gio::Cancellable>,
        pub previews: RefCell<Vec<WallpaperPreview>>,
        pub sender: Cell<Option<Sender<Result<()>>>>,
        pub uri: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WallpaperWindow {
        const NAME: &'static str = "XdppWallpaperWindow";
        type Type = super::WallpaperWindow;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WallpaperWindow {
        fn constructed(&self) {
            self.parent_constructed();
            *self.cancellable.borrow_mut() = gio::Cancellable::new();
        }

        fn dispose(&self) {
            self.cancellable.borrow().cancel();
        }
    }

    impl WidgetImpl for WallpaperWindow {}

    impl WindowImpl for WallpaperWindow {}

    impl AdwWindowImpl for WallpaperWindow {}

    #[gtk::template_callbacks]
    impl WallpaperWindow {
        #[template_callback]
        fn on_cancel_clicked(&self, _button: &gtk::Button) {
            let error = PortalError::Cancelled(String::from("Cancelled by user"));
            self.send_response(Err(error));
        }

        fn on_copy_ready(
            &self,
            result: std::result::Result<(), glib::Error>,
            dest: &gio::File,
            set_on: [bool; 2],
        ) {
            if let Err(err) = result {
                glib::g_critical!(LOG_DOMAIN, "Unable to copy file: {err}");
                let error = PortalError::Cancelled(String::from("Failed to set wallpaper"));
                self.send_response(Err(error));
                return;
            }

            for (idx, schema) in [HOME_SCHEMA, LOCKSCREEN_SCHEMA].iter().enumerate() {
                if !set_on[idx] {
                    continue;
                }

                let settings = gio::Settings::new(schema);
                settings
                    .set_string(PICTURE_URI_NORMAL_KEY, &dest.uri())
                    .unwrap();

                // Different wallpaper for dark-mode is available only for background schema.
                if idx == 0 {
                    settings
                        .set_string(PICTURE_URI_DARK_KEY, &dest.uri())
                        .unwrap();
                }
            }
            self.send_response(Ok(()));
        }

        fn on_make_directory_ready(
            &self,
            result: std::result::Result<(), glib::Error>,
            dir: &gio::File,
        ) {
            if let Err(err) = result {
                if !err.matches(gio::IOErrorEnum::Exists) {
                    glib::g_critical!(LOG_DOMAIN, "Unable to create directory: {err}");
                    let error = PortalError::Cancelled(String::from("Failed to set wallpaper"));
                    self.send_response(Err(error));
                    return;
                }
            }

            let for_home = [1, 2].contains(&self.set_on_row.selected());
            let for_lockscreen = [0, 2].contains(&self.set_on_row.selected());
            let src = gio::File::for_uri(&self.uri.borrow());

            let mut dir = dir.path().unwrap();
            if for_home && for_lockscreen {
                dir.push(BACKGROUND_FILENAME);
            } else if for_home {
                dir.push(HOME_FILENAME);
            } else if for_lockscreen {
                dir.push(LOCKSCREEN_FILENAME);
            } else {
                glib::g_critical!(LOG_DOMAIN, "Invalid set_on");
                panic!()
            }

            let dest = gio::File::for_path(dir.as_path());
            src.copy_async(
                &dest,
                gio::FileCopyFlags::OVERWRITE,
                glib::Priority::DEFAULT,
                Some(&*self.cancellable.borrow()),
                None,
                glib::clone!(
                    #[weak(rename_to = this)]
                    self,
                    #[weak]
                    dest,
                    move |result| this.on_copy_ready(result, &dest, [for_home, for_lockscreen])
                ),
            );
        }

        #[template_callback]
        fn on_change_clicked(&self, _button: &gtk::Button) {
            let mut config = glib::user_config_dir();
            config.push(PORTAL_NAME);
            let dir = gio::File::for_path(config.as_path());
            dir.make_directory_async(
                glib::Priority::DEFAULT,
                Some(&*self.cancellable.borrow()),
                glib::clone!(
                    #[weak(rename_to = this)]
                    self,
                    #[weak]
                    dir,
                    move |result| this.on_make_directory_ready(result, &dir)
                ),
            );
        }

        fn send_response(&self, response: Result<()>) {
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

        #[template_callback]
        fn on_selected_changed(&self, _param: &glib::ParamSpec, row: &adw::ComboRow) {
            let show_home = [1, 2].contains(&row.selected());
            let show_lockscreen = [0, 2].contains(&row.selected());

            for preview in &*self.previews.borrow() {
                preview.set_properties_from_value(&[
                    ("show-home", show_home.into()),
                    ("show-lockscreen", show_lockscreen.into()),
                ]);
            }
        }

        pub fn load_previews(&self) {
            let Some(display) = gdk::Display::default() else {
                glib::g_critical!(LOG_DOMAIN, "No display present");
                return;
            };

            let show_home = [1, 2].contains(&self.set_on_row.selected());
            let show_lockscreen = [0, 2].contains(&self.set_on_row.selected());

            let mut previews = self.previews.borrow_mut();
            for element in &display.monitors() {
                let object = element.unwrap();
                let monitor = object.dynamic_cast_ref::<gdk::Monitor>().unwrap();
                let preview = WallpaperPreview::from_parameters(monitor, &self.uri.borrow());
                preview.set_properties_from_value(&[
                    ("show-home", show_home.into()),
                    ("show-lockscreen", show_lockscreen.into()),
                ]);
                self.carousel.append(&preview);
                previews.push(preview);
            }
            self.indicator.set_visible(!previews.is_empty());
        }
    }
}

glib::wrapper! {
    pub struct WallpaperWindow(ObjectSubclass<imp::WallpaperWindow>)
        @extends adw::Window, gtk::Window, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl WallpaperWindow {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}

impl Default for WallpaperWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl Responder for WallpaperWindow {
    fn respond(&self, request: Request) {
        if let Request::WallpaperWithUri {
            application,
            uri,
            options,
            sender,
        } = request
        {
            let imp = self.imp();

            let app_name = get_application_name(&application);
            let desc = match app_name {
                Some(app_name) => gettextf("{} requests to change your wallpaper.", &[&app_name]),
                None => gettextf("An app requests to change your wallpaper.", &[]),
            };
            imp.desc_row.set_subtitle(desc.as_str());

            match options.set_on().unwrap_or(SetOn::Background) {
                SetOn::Lockscreen => imp.set_on_row.set_selected(0),
                SetOn::Background => imp.set_on_row.set_selected(1),
                SetOn::Both => imp.set_on_row.set_selected(2),
            }

            *imp.uri.borrow_mut() = uri.to_string();
            imp.sender.set(Some(sender));
            imp.load_previews();

            present_and_set_transient(self, application.window_identifier);
        } else {
            glib::g_critical!(LOG_DOMAIN, "Unknown request {request:#?}");
            panic!();
        }
    }

    fn cancel(&self) {
        self.close();
    }
}
