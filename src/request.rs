/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use ashpd::backend::app_chooser::{Choice, ChooserOptions, DesktopID};
use ashpd::backend::Result;
use ashpd::desktop::account::{UserInformation, UserInformationOptions};
use ashpd::desktop::file_chooser::{
    OpenFileOptions, SaveFileOptions, SaveFilesOptions, SelectedFiles,
};
use ashpd::desktop::wallpaper::WallpaperOptions;
use ashpd::{AppID, Uri, WindowIdentifierType};
use tokio::sync::oneshot::Sender;
use url::Url;

/// Essential information about the external application which does a portal request.
#[derive(Debug)]
pub struct Application {
    pub app_id: Option<AppID>,
    pub window_identifier: Option<WindowIdentifierType>,
}

/// Different types of portal requests. The `GLib` world picks the matching responder and passes the
/// request to it. Each request has a `sender` through which the responder will send the reply.
#[derive(Debug)]
pub enum Request {
    AccountGetUserInformation {
        application: Application,
        options: UserInformationOptions,
        sender: Sender<Result<UserInformation>>,
    },
    AppChooserChooseApplication {
        application: Application,
        choices: Vec<DesktopID>,
        options: ChooserOptions,
        sender: Sender<Result<Choice>>,
    },
    AppChooserUpdateChoices {
        choices: Vec<DesktopID>,
        sender: Sender<Result<()>>,
    },
    FileChooserOpenFile {
        application: Application,
        title: String,
        options: OpenFileOptions,
        sender: Sender<Result<SelectedFiles>>,
    },
    FileChooserSaveFile {
        application: Application,
        title: String,
        options: SaveFileOptions,
        sender: Sender<Result<SelectedFiles>>,
    },
    FileChooserSaveFiles {
        application: Application,
        title: String,
        options: SaveFilesOptions,
        sender: Sender<Result<SelectedFiles>>,
    },
    WallpaperWithUri {
        application: Application,
        uri: Uri,
        options: WallpaperOptions,
        sender: Sender<Result<()>>,
    },
}
