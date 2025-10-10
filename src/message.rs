/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::Request;

static REQUEST_ID: AtomicUsize = AtomicUsize::new(1);

/// A message to the `GLib` world from the ASHPD world.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Message {
    /// User has the cancelled the request of given ID.
    Cancel { request_id: usize },
    /// Received reply from a responder for the request of given ID, the responder can be closed
    /// now.
    Done { request_id: usize },
    /// A new request from user.
    Request { request_id: usize, request: Request },
}

impl Message {
    #[must_use]
    pub fn cancel(request_id: usize) -> Self {
        Self::Cancel { request_id }
    }

    #[must_use]
    pub fn done(request_id: usize) -> Self {
        Self::Done { request_id }
    }

    pub fn request(request: Request) -> (usize, Self) {
        let request_id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
        let message = Self::Request {
            request_id,
            request,
        };
        (request_id, message)
    }

    #[must_use]
    pub fn request_with_id(request_id: usize, request: Request) -> Self {
        Self::Request {
            request_id,
            request,
        }
    }
}
