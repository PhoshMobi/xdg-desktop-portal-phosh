/*
 * Copyright (C) 2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use std::collections::HashMap;
use std::sync::RwLock;

use ashpd::async_trait::async_trait;
use ashpd::backend::request::RequestImpl;
use ashpd::backend::wallpaper::WallpaperImpl;
use ashpd::backend::Result;
use ashpd::desktop::wallpaper::WallpaperOptions;
use ashpd::desktop::HandleToken;
use ashpd::{AppID, Uri, WindowIdentifierType};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use url::Url;

use crate::{Application, Message, Request, Requester};

/*
 * Handler for Wallpaper interface requests.
 */

pub struct Wallpaper {
    sender: Sender<Message>,
    map: RwLock<HashMap<HandleToken, usize>>,
}

impl Requester for Wallpaper {
    fn new(sender: Sender<Message>) -> Self {
        Wallpaper {
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
impl RequestImpl for Wallpaper {
    async fn close(&self, token: HandleToken) {
        self.send_cancel(&token).await;
    }
}

#[async_trait]
impl WallpaperImpl for Wallpaper {
    async fn with_uri(
        &self,
        token: HandleToken,
        app_id: Option<AppID>,
        window_identifier: Option<WindowIdentifierType>,
        uri: Uri,
        options: WallpaperOptions,
    ) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        let request = Request::WallpaperWithUri {
            application: Application {
                app_id,
                window_identifier,
            },
            uri,
            options,
            sender,
        };
        let result = self.send_request(&token, request, receiver).await;
        self.send_done(&token).await;
        return result;
    }
}
