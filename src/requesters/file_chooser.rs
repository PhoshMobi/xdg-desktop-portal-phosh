/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use std::collections::HashMap;
use std::sync::RwLock;

use ashpd::async_trait::async_trait;
use ashpd::backend::file_chooser::{
    FileChooserImpl, OpenFileOptions, SaveFileOptions, SaveFilesOptions, SelectedFiles,
};
use ashpd::backend::request::RequestImpl;
use ashpd::backend::Result;
use ashpd::desktop::HandleToken;
use ashpd::{AppID, WindowIdentifierType};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;

use crate::{Application, Message, Request, Requester};

/*
 * Handler for FileChooser interface requests.
 */

pub struct FileChooser {
    sender: Sender<Message>,
    map: RwLock<HashMap<HandleToken, usize>>,
}

impl Requester for FileChooser {
    fn new(sender: Sender<Message>) -> Self {
        FileChooser {
            sender,
            map: RwLock::new(HashMap::new()),
        }
    }

    fn sender(&self) -> &Sender<Message> {
        &self.sender
    }

    fn map(&self) -> &RwLock<HashMap<HandleToken, usize>> {
        &self.map
    }
}

#[async_trait]
impl RequestImpl for FileChooser {
    async fn close(&self, token: HandleToken) {
        self.send_cancel(&token).await;
    }
}

#[async_trait]
impl FileChooserImpl for FileChooser {
    async fn open_file(
        &self,
        token: HandleToken,
        app_id: Option<AppID>,
        window_identifier: Option<WindowIdentifierType>,
        title: &str,
        options: OpenFileOptions,
    ) -> Result<SelectedFiles> {
        let (sender, receiver) = oneshot::channel();
        let request = Request::FileChooserOpenFile {
            application: Application {
                app_id,
                window_identifier,
            },
            title: String::from(title),
            options,
            sender,
        };
        let result = self.send_request(&token, request, receiver).await;
        self.send_done(&token).await;
        return result;
    }

    async fn save_file(
        &self,
        token: HandleToken,
        app_id: Option<AppID>,
        window_identifier: Option<WindowIdentifierType>,
        title: &str,
        options: SaveFileOptions,
    ) -> Result<SelectedFiles> {
        let (sender, receiver) = oneshot::channel();
        let request = Request::FileChooserSaveFile {
            application: Application {
                app_id,
                window_identifier,
            },
            title: String::from(title),
            options,
            sender,
        };
        let result = self.send_request(&token, request, receiver).await;
        self.send_done(&token).await;
        return result;
    }

    async fn save_files(
        &self,
        token: HandleToken,
        app_id: Option<AppID>,
        window_identifier: Option<WindowIdentifierType>,
        title: &str,
        options: SaveFilesOptions,
    ) -> Result<SelectedFiles> {
        let (sender, receiver) = oneshot::channel();
        let request = Request::FileChooserSaveFiles {
            application: Application {
                app_id,
                window_identifier,
            },
            title: String::from(title),
            options,
            sender,
        };
        let result = self.send_request(&token, request, receiver).await;
        self.send_done(&token).await;
        return result;
    }
}
