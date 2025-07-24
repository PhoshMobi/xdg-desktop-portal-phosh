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
use ashpd::backend::app_chooser::{AppChooserImpl, Choice, ChooserOptions, DesktopID};
use ashpd::backend::request::RequestImpl;
use ashpd::backend::Result;
use ashpd::desktop::HandleToken;
use ashpd::{AppID, WindowIdentifierType};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;

use crate::{Application, Message, Request, Requester};

/*
 * Handler for AppChooser interface requests.
 */

pub struct AppChooser {
    sender: Sender<Message>,
    map: RwLock<HashMap<HandleToken, usize>>,
}

impl Requester for AppChooser {
    fn new(sender: Sender<Message>) -> Self {
        AppChooser {
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
impl RequestImpl for AppChooser {
    async fn close(&self, token: HandleToken) {
        self.send_cancel(&token).await;
    }
}

#[async_trait]
impl AppChooserImpl for AppChooser {
    async fn choose_application(
        &self,
        token: HandleToken,
        app_id: Option<AppID>,
        window_identifier: Option<WindowIdentifierType>,
        choices: Vec<DesktopID>,
        options: ChooserOptions,
    ) -> Result<Choice> {
        let (sender, receiver) = oneshot::channel();
        let request = Request::AppChooserChooseApplication {
            application: Application {
                app_id,
                window_identifier,
            },
            choices,
            options,
            sender,
        };
        let result = self.send_request(&token, request, receiver).await;
        self.send_done(&token).await;
        return result;
    }

    async fn update_choices(&self, handle: HandleToken, choices: Vec<DesktopID>) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        let request = Request::AppChooserUpdateChoices { choices, sender };
        let result = self.update_request(&handle, request, receiver).await;
        return result;
    }
}
