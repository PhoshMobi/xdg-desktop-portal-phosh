/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use futures_util::future::pending;
use gtk::glib;
use std::boxed::Box;
use std::collections::HashMap;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use xdg_desktop_portal_phosh::requesters;
use xdg_desktop_portal_phosh::responders;
use xdg_desktop_portal_phosh::{Message, Request, Requester, Responder};

mod bin_config;

/*
 * The entry-point to the backend server application.
 *
 * The portal backend contains three components: GLib world, main and ASHPD world. The ASHPD world
 * contains requesters. They handle the portal requests from outside and pass it to the main. Each
 * request has a `sender` channel through which the reply must be sent. The main launches the
 * appropriate responder in the GLib world who can handle the request. The responder gets the
 * required information from user and passes the reply through `sender`. Once the requester gets the
 * reply, it then hands it over to the original portal request. Once done, it sends a `done` message
 * to main, so it can close the respective responder. Similarly, if the user cancels the request,
 * then requester sends a `cancel` message so that main can cancel the responder.
 */

const LOG_DOMAIN: &str = "xdpp";

fn main() {
    xdg_desktop_portal_phosh::init();

    let main_loop = glib::MainLoop::new(None, false);

    let (sender, mut receiver) = mpsc::channel(bin_config::MPSC_BUFFER);

    let runtime = Runtime::new().unwrap();
    runtime.spawn(glib::clone!(
        #[strong]
        sender,
        #[strong]
        main_loop,
        async move {
            let result = ashpd_main(sender).await;
            if let Err(error) = result {
                glib::g_critical!(LOG_DOMAIN, "ashpd server failed: {error}");
                main_loop.quit();
            }
        }
    ));

    let mut map: HashMap<usize, Box<dyn Responder>> = HashMap::new();
    glib::spawn_future_local(async move {
        while let Some(message) = receiver.recv().await {
            glib::g_debug!(LOG_DOMAIN, "New message: {message:#?}");
            match message {
                Message::Cancel { request_id } => {
                    if let Some(responder) = map.remove(&request_id) {
                        responder.cancel()
                    } else {
                        glib::g_critical!(LOG_DOMAIN, "No responder found for {request_id}")
                    }
                }
                Message::Done { request_id } => {
                    map.remove(&request_id);
                }
                Message::Request {
                    request_id,
                    request,
                } => {
                    let responder: Option<Box<dyn Responder>> = match request {
                        Request::AccountGetUserInformation {
                            application: _,
                            options: _,
                            sender: _,
                        } => Some(Box::new(responders::AccountWindow::new())),
                        Request::AppChooserChooseApplication {
                            application: _,
                            choices: _,
                            options: _,
                            sender: _,
                        } => Some(Box::new(responders::AppChooserWindow::new())),
                        Request::AppChooserUpdateChoices {
                            choices: _,
                            sender: _,
                        } => {
                            let responder = map.remove(&request_id);
                            if responder.is_none() {
                                glib::g_critical!(LOG_DOMAIN, "No responder found for {request_id}")
                            }
                            responder
                        }
                    };

                    if let Some(responder) = responder {
                        responder.respond(request);
                        map.insert(request_id, responder);
                    };
                }
            };
        }
    });

    glib::g_message!(LOG_DOMAIN, "Running main loop");

    main_loop.run();
}

async fn ashpd_main(sender: mpsc::Sender<Message>) -> ashpd::Result<()> {
    let mut builder = ashpd::backend::Builder::new(bin_config::DBUS_NAME)?;

    builder = if bin_config::ACCOUNT {
        glib::g_debug!(LOG_DOMAIN, "Adding interface: Account");
        builder.account(requesters::Account::new(sender.clone()))
    } else {
        builder
    };

    builder = if bin_config::APP_CHOOSER {
        glib::g_debug!(LOG_DOMAIN, "Adding interface: AppChooser");
        builder.app_chooser(requesters::AppChooser::new(sender.clone()))
    } else {
        builder
    };

    builder.build().await?;

    glib::g_message!(
        LOG_DOMAIN,
        "Running ashpd loop under {}",
        bin_config::DBUS_NAME
    );

    loop {
        pending::<()>().await;
    }
}
