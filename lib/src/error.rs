// Copyright 2021 Oxide Computer Company

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
