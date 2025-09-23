/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use std::cell::Cell;
use std::path::PathBuf;

use ashpd::backend::file_chooser::SelectedFiles;
use ashpd::backend::Result;
use ashpd::desktop::file_chooser::{Choice, FileFilter};
use ashpd::url::Url;
use ashpd::PortalError;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use pfs::file_selector::{FileSelector, FileSelectorMode};
use tokio::sync::oneshot::Sender;

use crate::utils::gettextf;
use crate::{Request, Responder};

/*
 * `FileChooser` handles the File Chooser interface. It uses Phosh File Selector to display a dialog
 * to let users choose files.
 */

const LOG_DOMAIN: &str = "xdpp-file-chooser";

/// Split the string by extension.
///
/// The extension is the substring from the first `.` to the end of the string. If the string starts
/// with a `.`, then the extension is searched from the second `.`.
///
/// Example:
/// ```ignore
/// assert_eq!(split_ext(".foo.tar.gz"), (".foo", ".tar.gz"));
/// ```
fn split_ext(file_name: &str) -> (&str, &str) {
    let mut idx = file_name.len();
    let chars = file_name.chars();
    for (i, ch) in chars.enumerate() {
        if i != 0 && ch == '.' {
            idx = i;
            break;
        }
    }

    let prefix = &file_name[..idx];
    let suffix = &file_name[idx..];

    (prefix, suffix)
}

fn get_unique_file_uri(original: &str, directory: &gio::File) -> Url {
    let (prefix, suffix) = split_ext(original);
    let mut file = directory.child(original);
    let mut count = 2;

    while file.query_exists(gio::Cancellable::NONE) {
        let new_name = format!("{prefix} ({count}){suffix}");
        file = directory.child(&new_name);
        count += 1;
    }

    let uri = file.uri();
    Url::parse(&uri).unwrap()
}

fn convert_file_filter(filter: &FileFilter) -> gtk::FileFilter {
    let gtk_filter = gtk::FileFilter::new();
    gtk_filter.set_name(Some(filter.label()));

    for mime_type in filter.mimetype_filters() {
        gtk_filter.add_mime_type(mime_type);
    }

    for pattern in filter.pattern_filters() {
        gtk_filter.add_pattern(pattern);
    }

    gtk_filter
}

fn convert_filters(
    current_filter: Option<&FileFilter>,
    filters: &[FileFilter],
) -> (u32, gio::ListModel) {
    let model = gio::ListStore::with_type(gtk::FileFilter::static_type());
    let mut current_filter_pos = gtk::INVALID_LIST_POSITION;

    for (i, filter) in filters.into_iter().enumerate() {
        model.append(&convert_file_filter(filter));
        if current_filter.is_some() && current_filter.unwrap() == filter {
            current_filter_pos = i.try_into().unwrap();
        }
    }

    if current_filter.is_some() && filters.len() == 0 {
        let current_filter = current_filter.unwrap();
        model.append(&convert_file_filter(current_filter));
        current_filter_pos = 0;
    }

    (current_filter_pos, model.into())
}

fn convert_choices(choices: &[Choice]) -> glib::Variant {
    let mut choices_vec = Vec::new();
    for choice in choices {
        choices_vec.push((
            choice.id(),
            choice.label(),
            choice.pairs().to_variant(),
            choice.initial_selection(),
        ));
    }
    choices_vec.to_variant()
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct FileChooser {
        pub mode: Cell<Option<FileSelectorMode>>,
        pub filters: Cell<Vec<FileFilter>>,
        pub files: Cell<Vec<PathBuf>>,
        pub window: Cell<Option<FileSelector>>,
        pub sender: Cell<Option<Sender<Result<SelectedFiles>>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileChooser {
        const NAME: &'static str = "XdppFileChooser";
        type Type = super::FileChooser;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for FileChooser {}

    impl FileChooser {
        pub fn on_file_selector_done(&self, success: bool) {
            if !success {
                let error = PortalError::Cancelled(String::from("Cancelled by user"));
                self.send_response(Err(error));
                return;
            }

            let Some(window) = self.window.take() else {
                glib::g_critical!(LOG_DOMAIN, "No window available to take");
                let error = PortalError::Failed(String::from("Internal error"));
                self.send_response(Err(error));
                return;
            };

            let Some(uris) = window.selected() else {
                let error = PortalError::Cancelled(String::from("Cancelled by user"));
                self.send_response(Err(error));
                return;
            };

            if uris.len() == 0 {
                let error = PortalError::Cancelled(String::from("Cancelled by user"));
                self.send_response(Err(error));
                return;
            }

            let mut files = SelectedFiles::default();

            let Some(mode) = self.mode.take() else {
                glib::g_critical!(LOG_DOMAIN, "No mode available to take");
                let error = PortalError::Failed(String::from("Internal error"));
                self.send_response(Err(error));
                return;
            };

            match mode {
                FileSelectorMode::OpenFile | FileSelectorMode::SaveFile => {
                    for uri in uris {
                        let url = Url::parse(&uri).unwrap();
                        files = files.uri(url);
                    }

                    let current_filter_pos: u32 = window.property("current-filter");
                    let mut filters = self.filters.take();
                    if (current_filter_pos as usize) < filters.len() {
                        let current_filter = filters.remove(current_filter_pos as usize);
                        files = files.current_filter(current_filter);
                    }

                    let choices_variant: glib::Variant = window.property("selected-choices");
                    let choices = <Vec<(String, String)>>::from_variant(&choices_variant).unwrap();
                    for (key, value) in choices {
                        files = files.choice(&key, &value);
                    }
                }
                FileSelectorMode::SaveFiles => {
                    let directory = gio::File::for_uri(&uris[0]);
                    for file_name in self.files.take() {
                        let os_str = file_name.as_os_str();
                        let file_name_str = os_str.to_str().unwrap();
                        let uri = get_unique_file_uri(file_name_str, &directory);
                        files = files.uri(uri);
                    }
                }
            }

            self.send_response(Ok(files));
        }

        fn send_response(&self, response: Result<SelectedFiles>) {
            let sender = self.sender.take();
            if let Some(sender) = sender {
                if sender.send(response).is_err() {
                    glib::g_critical!(LOG_DOMAIN, "Unable to send response through sender");
                }
            } else {
                glib::g_critical!(LOG_DOMAIN, "Sender is not available");
            }
        }
    }
}

glib::wrapper! {
pub struct FileChooser(ObjectSubclass<imp::FileChooser>);
}

impl FileChooser {
    pub fn new() -> Self {
        pfs::init::init();
        glib::Object::builder().build()
    }
}

impl Responder for FileChooser {
    fn respond(&self, request: Request) {
        let application;
        let sender;
        let mode;
        let mut filters = Vec::new();
        let mut files = Vec::new();
        let modal;
        let mut props = Vec::new();

        if let Request::FileChooserOpenFile {
            application: application_in,
            title,
            options,
            sender: sender_in,
        } = request
        {
            application = application_in;
            sender = sender_in;

            mode = FileSelectorMode::OpenFile;
            props.push(("mode", mode.into()));

            props.push(("title", title.into()));

            if let Some(accept_label) = options.accept_label() {
                props.push(("accept-label", accept_label.into()));
            } else {
                props.push(("accept-label", gettextf("Open", &[]).into()));
            }

            modal = options.modal().unwrap_or(true);

            props.push(("directory", options.directory().unwrap_or(false).into()));

            let (current_filter, file_filters) =
                convert_filters(options.current_filter(), options.filters());
            props.push(("current_filter", current_filter.into()));
            props.push(("filters", file_filters.into()));
            filters.extend(options.filters().iter().map(|filter| filter.to_owned()));

            let choices = convert_choices(options.choices());
            props.push(("choices", choices.into()));

            if let Some(current_folder_path) = options.current_folder() {
                let current_folder = gio::File::for_path(current_folder_path);
                props.push(("current-folder", current_folder.into()));
            } else {
                let current_folder = gio::File::for_path(glib::home_dir());
                props.push(("current-folder", current_folder.into()));
            }
        } else if let Request::FileChooserSaveFile {
            application: application_in,
            title,
            options,
            sender: sender_in,
        } = request
        {
            application = application_in;
            sender = sender_in;

            mode = FileSelectorMode::SaveFile;
            props.push(("mode", mode.into()));

            props.push(("title", title.into()));

            if let Some(accept_label) = options.accept_label() {
                props.push(("accept-label", accept_label.into()));
            } else {
                props.push(("accept-label", gettextf("Save", &[]).into()));
            }

            modal = options.modal().unwrap_or(true);

            let (current_filter, file_filters) =
                convert_filters(options.current_filter(), options.filters());
            props.push(("current_filter", current_filter.into()));
            props.push(("filters", file_filters.into()));
            filters.extend(options.filters().iter().map(|filter| filter.to_owned()));

            let choices = convert_choices(options.choices());
            props.push(("choices", choices.into()));

            if let Some(current_file_path) = options.current_file() {
                let current_file = gio::File::for_path(current_file_path);
                let current_folder = current_file.parent();
                let current_name = current_file.basename();
                props.push(("current-folder", current_folder.into()));
                props.push(("filename", current_name.into()));
            } else if let Some(current_folder_path) = options.current_folder() {
                let current_folder = gio::File::for_path(current_folder_path);
                props.push(("current-folder", current_folder.into()));
                props.push(("filename", options.current_name().unwrap_or("").into()));
            } else {
                let current_folder = gio::File::for_path(glib::home_dir());
                props.push(("current-folder", current_folder.into()));
            }
        } else if let Request::FileChooserSaveFiles {
            application: application_in,
            title,
            options,
            sender: sender_in,
        } = request
        {
            application = application_in;
            sender = sender_in;

            mode = FileSelectorMode::SaveFiles;
            props.push(("mode", mode.into()));

            props.push(("title", title.into()));

            if let Some(accept_label) = options.accept_label() {
                props.push(("accept-label", accept_label.into()));
            } else {
                props.push(("accept-label", gettextf("Save", &[]).into()));
            }

            modal = options.modal().unwrap_or(true);

            if let Some(current_folder_path) = options.current_folder() {
                let current_folder = gio::File::for_path(current_folder_path);
                props.push(("current-folder", current_folder.into()));
            } else {
                let current_folder = gio::File::for_path(glib::home_dir());
                props.push(("current-folder", current_folder.into()));
            }

            files.extend(
                options
                    .files()
                    .iter()
                    .map(|path| PathBuf::from(path.as_ref())),
            );
        } else {
            glib::g_critical!(LOG_DOMAIN, "Unknown request {request:#?}");
            panic!();
        }

        let window = FileSelector::new();
        window.set_properties_from_value(&props);

        let imp = self.imp();

        window.connect_closure(
            "done",
            false,
            glib::closure_local!(
                #[weak(rename_to = this)]
                imp,
                move |_: FileSelector, success: bool| this.on_file_selector_done(success),
            ),
        );

        if let Some(identifier) = application.window_identifier {
            identifier.set_parent_of(&window);
        } else {
            glib::g_warning!(LOG_DOMAIN, "Application does not have window identifier");
        }
        window.set_modal(modal);

        window.present();

        imp.mode.set(Some(mode));
        imp.filters.set(filters);
        imp.files.set(files);
        imp.window.set(Some(window));
        imp.sender.set(Some(sender));
    }

    fn cancel(&self) {
        let imp = self.imp();
        let window = imp.window.take();
        if let Some(window) = window {
            window.close()
        } else {
            glib::g_critical!(LOG_DOMAIN, "No window available to close");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_split_ext() {
        assert_eq!(split_ext("foo.txt"), ("foo", ".txt"));
        assert_eq!(split_ext("foo.tar.gz"), ("foo", ".tar.gz"));
        assert_eq!(split_ext("foo."), ("foo", "."));
        assert_eq!(split_ext("foo"), ("foo", ""));
        assert_eq!(split_ext(".foo"), (".foo", ""));
        assert_eq!(split_ext(".foo."), (".foo", "."));
        assert_eq!(split_ext(".foo.tar.gz"), (".foo", ".tar.gz"));
        assert_eq!(split_ext(".foo.txt"), (".foo", ".txt"));
    }
}
