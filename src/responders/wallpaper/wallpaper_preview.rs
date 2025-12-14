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
use glib::translate::FromGlib;
use gtk::glib::subclass::InitializingObject;
use gtk::glib::Properties;
use gtk::{gdk, gdk_pixbuf, gio, glib, CompositeTemplate, TemplateChild};

use super::{transformations, DesktopBackgroundStyle};

/*
 * `WallpaperPreview` displays a preview of the wallpaper when it is drawn for given monitor.
 */

const LOG_DOMAIN: &str = "xdpp-wallpaper-preview";

const HOME_SCHEMA: &str = "org.gnome.desktop.background";
const LOCKSCREEN_SCHEMA: &str = "org.gnome.desktop.screensaver";
const PRIMARY_COLOR_KEY: &str = "primary-color";
const STYLE_KEY: &str = "picture-options";

const INTERFACE_SCHEMA: &str = "org.gnome.desktop.interface";
const CLOCK_FORMAT_KEY: &str = "clock-format";

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn get_background_style(home: bool) -> (u32, DesktopBackgroundStyle) {
    let schema = if home { HOME_SCHEMA } else { LOCKSCREEN_SCHEMA };
    let settings = gio::Settings::new(schema);

    let value = settings.enum_(STYLE_KEY);
    let style = unsafe { DesktopBackgroundStyle::from_glib(value) };

    let primary_color = gdk::RGBA::parse(settings.string(PRIMARY_COLOR_KEY)).unwrap();
    let primary_color = (((primary_color.red() * 256.0) as u32) << 24)
        | (((primary_color.green() * 256.0) as u32) << 16)
        | (((primary_color.blue() * 256.0) as u32) << 8)
        | ((primary_color.alpha() * 256.0) as u32);

    (primary_color, style)
}

fn create_background(
    wallpaper: &gdk_pixbuf::Pixbuf,
    primary_color: u32,
    style: DesktopBackgroundStyle,
    dst_w: i32,
    dst_h: i32,
) -> gdk_pixbuf::Pixbuf {
    let (src_w, src_h) = (wallpaper.width(), wallpaper.height());

    glib::g_debug!(
        LOG_DOMAIN,
        "To create background: wallpaper: {src_w} ✕ {src_h}, \
                               monitor: {dst_w} ✕ {dst_h}, \
                               primary_color: {primary_color}, \
                               style: {style:#?}"
    );

    let canvas = gdk_pixbuf::Pixbuf::new(
        wallpaper.colorspace(),
        true,
        wallpaper.bits_per_sample(),
        dst_w,
        dst_h,
    )
    .unwrap();

    canvas.fill(primary_color);

    let transformation = match style {
        DesktopBackgroundStyle::Scaled => {
            transformations::content_fit_scale_down(src_w, src_h, dst_w, dst_h)
        }
        DesktopBackgroundStyle::Zoom => {
            transformations::content_fit_cover(src_w, src_h, dst_w, dst_h)
        }
        style => {
            let enum_type =
                glib::EnumClass::with_type(DesktopBackgroundStyle::static_type()).unwrap();
            let nick = enum_type.value(style as i32).unwrap().nick();
            glib::g_warning!(
                LOG_DOMAIN,
                "Desktop background style '{nick}' is not supported, using 'zoom'"
            );
            transformations::content_fit_cover(src_w, src_h, dst_w, dst_h)
        }
    };
    glib::g_debug!(LOG_DOMAIN, "Wallpaper transformation: {transformation:#?}");

    let wallpaper = wallpaper
        .scale_simple(
            transformation.new_w,
            transformation.new_h,
            gdk_pixbuf::InterpType::Bilinear,
        )
        .unwrap();
    wallpaper.copy_area(
        transformation.x,
        transformation.y,
        transformation.w,
        transformation.h,
        &canvas,
        transformation.dst_x,
        transformation.dst_y,
    );

    canvas
}

fn get_time_format() -> &'static str {
    let settings = gio::Settings::new(INTERFACE_SCHEMA);
    let is_24h = settings.string(CLOCK_FORMAT_KEY) == "24h";

    if is_24h {
        "%R"
    } else {
        "%I:%M %p"
    }
}

mod imp {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::WallpaperPreview)]
    #[template(resource = "/mobi/phosh/xdpp/ui/wallpaper_preview.ui")]
    pub struct WallpaperPreview {
        #[property(construct_only, get, set)]
        monitor: RefCell<Option<gdk::Monitor>>,
        #[property(construct_only, get, set)]
        wallpaper_uri: RefCell<String>,
        #[property(get, set)]
        show_home: Cell<bool>,
        #[property(get, set)]
        show_lockscreen: Cell<bool>,

        #[template_child]
        frame: TemplateChild<gtk::AspectFrame>,
        #[template_child]
        home_pic: TemplateChild<gtk::Picture>,
        #[template_child]
        home_time: TemplateChild<gtk::Label>,
        #[template_child]
        lockscreen_pic: TemplateChild<gtk::Picture>,
        #[template_child]
        lockscreen_time: TemplateChild<gtk::Label>,
        #[template_child]
        lockscreen_date: TemplateChild<gtk::Label>,

        cancellable: RefCell<gio::Cancellable>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WallpaperPreview {
        const NAME: &'static str = "XdppWallpaperPreview";
        type Type = super::WallpaperPreview;
        type ParentType = adw::BreakpointBin;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("xdpp-wallpaper-preview");
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for WallpaperPreview {
        fn constructed(&self) {
            if self.monitor.borrow().is_none() {
                glib::g_critical!(LOG_DOMAIN, "monitor must be set");
                return;
            }

            let provider = gtk::CssProvider::new();
            provider.load_from_resource("/mobi/phosh/xdpp/css/wallpaper_preview.css");
            gtk::style_context_add_provider_for_display(
                &gdk::Display::default().unwrap(),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );

            *self.cancellable.borrow_mut() = gio::Cancellable::new();

            let file = gio::File::for_uri(&self.wallpaper_uri.borrow());
            file.read_async(
                glib::Priority::DEFAULT,
                Some(&*self.cancellable.borrow()),
                glib::clone!(
                    #[weak(rename_to = this)]
                    self,
                    move |result| this.on_file_read_ready(result)
                ),
            );

            let now = glib::DateTime::now_local().unwrap();

            let time_format = get_time_format();
            let time = now.format(time_format).unwrap();
            self.home_time.set_label(&time);
            self.lockscreen_time.set_label(&time);

            let date_format = "%x";
            let date = now.format(date_format).unwrap();
            self.lockscreen_date.set_label(&date);
        }

        fn dispose(&self) {
            self.cancellable.borrow().cancel();
        }
    }

    impl WidgetImpl for WallpaperPreview {}

    impl BreakpointBinImpl for WallpaperPreview {}

    impl WallpaperPreview {
        fn on_pixbuf_ready(&self, result: Result<gdk_pixbuf::Pixbuf, glib::Error>) {
            if let Err(err) = result {
                glib::g_critical!(LOG_DOMAIN, "{self:p}: Unable to load pixbuf: {err}");
                return;
            }

            let wallpaper = result.unwrap();
            let monitor = self.monitor.borrow();
            let geo = monitor.as_ref().unwrap().geometry();

            #[allow(clippy::cast_precision_loss)]
            self.frame
                .set_ratio((geo.width() as f32) / (geo.height() as f32));

            let (primary_color, style) = get_background_style(true);
            let pixbuf =
                create_background(&wallpaper, primary_color, style, geo.width(), geo.height());
            self.home_pic.set_pixbuf(Some(&pixbuf));

            let (primary_color, style) = get_background_style(false);
            let pixbuf =
                create_background(&wallpaper, primary_color, style, geo.width(), geo.height());
            self.lockscreen_pic.set_pixbuf(Some(&pixbuf));
        }

        fn on_file_read_ready(&self, result: Result<gio::FileInputStream, glib::Error>) {
            if let Err(err) = result {
                glib::g_critical!(LOG_DOMAIN, "{self:p}: Unable to open file: {err}");
                return;
            }

            let stream = result.unwrap();
            gdk_pixbuf::Pixbuf::from_stream_async(
                &stream,
                Some(&*self.cancellable.borrow()),
                glib::clone!(
                    #[weak(rename_to = this)]
                    self,
                    move |result| this.on_pixbuf_ready(result),
                ),
            );
        }
    }
}

glib::wrapper! {
    pub struct WallpaperPreview(ObjectSubclass<imp::WallpaperPreview>)
        @extends adw::BreakpointBin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl WallpaperPreview {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn from_parameters(monitor: &gdk::Monitor, wallpaper_uri: &str) -> Self {
        glib::Object::builder()
            .property("monitor", monitor)
            .property("wallpaper-uri", wallpaper_uri)
            .build()
    }
}

impl Default for WallpaperPreview {
    fn default() -> Self {
        Self::new()
    }
}
