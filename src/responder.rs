/*
 * Copyright (C) 2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

use crate::Request;

/// A responder reacts to the portal request, gathers input from the user and returns the reply to
/// it. While processing, if the request gets cancelled, then [`Responder.cancel`](Responder.cancel)
/// will be called.
pub trait Responder {
    fn respond(&self, request: Request);
    fn cancel(&self);
}
