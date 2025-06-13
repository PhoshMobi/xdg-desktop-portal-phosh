/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use ashpd::async_trait::async_trait;
use ashpd::backend::account::{AccountImpl, UserInformationOptions};
use ashpd::backend::request::RequestImpl;
use ashpd::backend::Result;
use ashpd::desktop::account::UserInformation;
use ashpd::desktop::HandleToken;
use ashpd::{AppID, WindowIdentifierType};
use std::collections::HashMap;
use std::sync::RwLock;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;

use crate::{Application, Message, Request, Requester};

/*
 * Handler for Account interface requests.
 */

pub struct Account {
    sender: Sender<Message>,
    map: RwLock<HashMap<HandleToken, usize>>,
}

impl Requester for Account {
    fn new(sender: Sender<Message>) -> Self {
        Account {
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
impl RequestImpl for Account {
    async fn close(&self, token: HandleToken) {
        self.send_cancel(&token).await;
    }
}

#[async_trait]
impl AccountImpl for Account {
    async fn get_user_information(
        &self,
        token: HandleToken,
        app_id: Option<AppID>,
        window_identifier: Option<WindowIdentifierType>,
        options: UserInformationOptions,
    ) -> Result<UserInformation> {
        let (sender, receiver) = oneshot::channel();
        let request = Request::AccountGetUserInformation {
            application: Application {
                app_id,
                window_identifier,
            },
            options,
            sender,
        };
        let result = self.send_request(&token, request, receiver).await;
        self.send_done(&token).await;
        return result;
    }
}
