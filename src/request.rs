/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use ashpd::backend::account::UserInformationOptions;
use ashpd::backend::Result;
use ashpd::desktop::account::UserInformation;
use ashpd::{AppID, WindowIdentifierType};
use tokio::sync::oneshot::Sender;

/// Essential information about the external application which does a portal request.
#[derive(Debug)]
pub struct Application {
    pub app_id: Option<AppID>,
    pub window_identifier: Option<WindowIdentifierType>,
}

/// Different types of portal requests. The GLib world picks the matching responder and passes the
/// request to it. Each request has a `sender` through which the responder will send the reply.
#[derive(Debug)]
pub enum Request {
    AccountGetUserInformation {
        application: Application,
        options: UserInformationOptions,
        sender: Sender<Result<UserInformation>>,
    },
}
