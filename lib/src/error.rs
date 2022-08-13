// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// Copyright 2022 Oxide Computer Company

use crate::proto::{MessageType, Rlerror};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum P9Error {
    #[error("expected {0} type found {1}")]
    UnexpectedReturnType(MessageType, MessageType),
    #[error("server error: {0}")]
    ServerError(Rlerror, String),
    #[error("error: {0}")]
    General(String),
}
