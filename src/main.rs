/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use std::boxed::Box;
use std::collections::HashMap;
use std::process::ExitCode;

use ashpd::zbus::fdo::RequestNameFlags;
use futures_util::future::pending;
use gtk::glib;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use xdg_desktop_portal_phosh::utils::gettextf;
use xdg_desktop_portal_phosh::{requesters, responders, Message, Request, Requester, Responder};

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

const HELP: &str = "Usage:
  {} [OPTION…]

A backend implementation of XDG Desktop Portal for Phosh environment in Rust.

  -h, --help\t\tPrint this help and exit.
  -r, --replace\t\tReplace existing instance.
  -v, --verbose\t\tPrint debug information.
  --version\t\tPrint version information and exit.

XDG Desktop Portal allow Flatpak apps, and other desktop containment frameworks, to interact with
the system in a secure and well defined way.
{} provides D-Bus interfaces to be used by XDG Desktop Portal.
Please see https://flatpak.github.io/xdg-desktop-portal/docs/index.html for more details about
portals and their purpose.

Please report issues at https://gitlab.gnome.org/guidog/xdg-desktop-portal-phosh/issues.";

struct Options {
    pub replace: bool,
    pub verbose: bool,
}

impl Options {
    pub fn new() -> Self {
        Options {
            replace: false,
            verbose: false,
        }
    }
}

fn handle_cli() -> Result<Options, ExitCode> {
    let mut args = std::env::args().into_iter();

    let mut options = Options::new();

    let Some(name) = args.next() else {
        return Ok(options);
    };

    for arg in args {
        match &arg[..] {
            "-h" | "--help" => {
                let help = gettextf(HELP, &[&name, &name]);
                println!("{help}");
                return Err(ExitCode::SUCCESS);
            }
            "-r" | "--replace" => {
                options.replace = true;
            }
            "-v" | "--verbose" => {
                options.verbose = true;
            }
            "--version" => {
                println!(env!("CARGO_PKG_VERSION"));
                return Err(ExitCode::SUCCESS);
            }
            arg => {
                let error = gettextf("Unknown argument: {}", &[arg]);
                eprintln!("{error}");
                return Err(ExitCode::FAILURE);
            }
        }
    }

    Ok(options)
}

fn message_handler(domain: Option<&str>, level: glib::LogLevel, message: &str) {
    let mut new_level = level;

    if level == glib::LogLevel::Debug && domain.unwrap_or("").starts_with(LOG_DOMAIN) {
        new_level = glib::LogLevel::Message;
    }

    glib::log_default_handler(domain, new_level, Some(message));
}

fn main() -> ExitCode {
    xdg_desktop_portal_phosh::i18n_init();

    let options = match handle_cli() {
        Ok(options) => options,
        Err(code) => return code,
    };

    if options.verbose {
        glib::log_set_default_handler(message_handler);
    }

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
            let result = ashpd_main(&options, sender).await;
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
                        responder.cancel();
                    } else {
                        glib::g_critical!(LOG_DOMAIN, "No responder found for {request_id}");
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
                                glib::g_critical!(
                                    LOG_DOMAIN,
                                    "No responder found for {request_id}"
                                );
                            }
                            responder
                        }
                        Request::FileChooserOpenFile {
                            application: _,
                            title: _,
                            options: _,
                            sender: _,
                        }
                        | Request::FileChooserSaveFile {
                            application: _,
                            title: _,
                            options: _,
                            sender: _,
                        }
                        | Request::FileChooserSaveFiles {
                            application: _,
                            title: _,
                            options: _,
                            sender: _,
                        } => Some(Box::new(responders::FileChooser::new())),
                    };

                    if let Some(responder) = responder {
                        responder.respond(request);
                        map.insert(request_id, responder);
                    }
                }
            }
        }
    });

    glib::g_message!(LOG_DOMAIN, "Running main loop");

    main_loop.run();

    ExitCode::SUCCESS
}

async fn ashpd_main(options: &Options, sender: mpsc::Sender<Message>) -> ashpd::Result<()> {
    let mut builder = ashpd::backend::Builder::new(bin_config::DBUS_NAME)?;

    builder = if options.replace {
        glib::g_debug!(LOG_DOMAIN, "Replacing existing instance");
        builder.with_flags(RequestNameFlags::ReplaceExisting.into())
    } else {
        builder
    };

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

    builder = if bin_config::FILE_CHOOSER {
        glib::g_debug!(LOG_DOMAIN, "Add interface: FileChooser");
        builder.file_chooser(requesters::FileChooser::new(sender.clone()))
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
