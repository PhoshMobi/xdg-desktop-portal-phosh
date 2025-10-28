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
use ashpd::backend::Result;
use ashpd::desktop::HandleToken;
use ashpd::PortalError;
use gtk::glib;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::Receiver;

use crate::{Message, Request};

const LOG_DOMAIN: &str = "xdpp-requester";

/// A requester is responsible for getting the portal requests from the ASHPD world and passing it
/// to the `GLib` world. It gets a `sender` through which it can communicate with the `GLib` world
/// about the requests.
#[async_trait]
pub trait Requester {
    fn new(sender: Sender<Message>) -> Self;
    fn sender(&self) -> &Sender<Message>;
    fn map(&self) -> &RwLock<HashMap<HandleToken, usize>>;

    async fn send_cancel(&self, token: &HandleToken) {
        let request_id;
        {
            let mut map = self.map().write().unwrap();
            request_id = map.remove(token);
        }

        if request_id.is_none() {
            glib::g_critical!(LOG_DOMAIN, "Unknown handle: {token:#?}");
            return;
        }

        let message = Message::cancel(request_id.unwrap());
        if let Err(error) = self.sender().send(message).await {
            glib::g_critical!(LOG_DOMAIN, "Error: {error}");
        }
    }

    async fn send_done(&self, token: &HandleToken) {
        let request_id;
        {
            let mut map = self.map().write().unwrap();
            request_id = map.remove(token);
        }

        if request_id.is_none() {
            glib::g_critical!(LOG_DOMAIN, "Unknown handle: {token}");
            return;
        }

        let message = Message::done(request_id.unwrap());
        if let Err(error) = self.sender().send(message).await {
            glib::g_critical!(LOG_DOMAIN, "Error: {error}");
        }
    }

    async fn send_request<T: std::fmt::Debug + std::marker::Send>(
        &self,
        token: &HandleToken,
        request: Request,
        receiver: Receiver<Result<T>>,
    ) -> Result<T> {
        glib::g_debug!(LOG_DOMAIN, "Request: {request:#?}");

        let (request_id, message) = Message::request(request);

        if let Err(error) = self.sender().send(message).await {
            glib::g_critical!(LOG_DOMAIN, "Error: {error}");
            return Err(PortalError::Failed(String::from("Unknown error")));
        }

        {
            let mut map = self.map().write().unwrap();
            map.insert(token.clone(), request_id);
        }

        let result = match receiver.await {
            Ok(response) => {
                glib::g_debug!(LOG_DOMAIN, "Response: {response:#?}");
                response
            }
            Err(error) => {
                glib::g_critical!(LOG_DOMAIN, "Error: {error}");
                Err(PortalError::Failed(String::from("Unknown error")))
            }
        };

        result
    }

    async fn update_request<T: std::fmt::Debug + std::marker::Send>(
        &self,
        token: &HandleToken,
        request: Request,
        receiver: Receiver<Result<T>>,
    ) -> Result<T> {
        glib::g_debug!(LOG_DOMAIN, "Request: {request:#?}");

        let message;
        {
            let map = self.map().read().unwrap();
            message = if let Some(request_id) = map.get(token) {
                Message::Request {
                    request_id: *request_id,
                    request,
                }
            } else {
                glib::g_critical!(LOG_DOMAIN, "Unknown request");
                return Err(PortalError::Failed(String::from("Unknown error")));
            }
        }

        if let Err(error) = self.sender().send(message).await {
            glib::g_critical!(LOG_DOMAIN, "Error: {error}");
            return Err(PortalError::Failed(String::from("Unknown error")));
        }

        let result = match receiver.await {
            Ok(response) => {
                glib::g_debug!(LOG_DOMAIN, "Response: {response:#?}");
                response
            }
            Err(error) => {
                glib::g_critical!(LOG_DOMAIN, "Error: {error}");
                Err(PortalError::Failed(String::from("Unknown error")))
            }
        };

        return result;
    }
}
