// Copyright 2021 Oxide Computer Company

use async_trait::async_trait;
use ispf::{from_bytes_le, to_bytes_le};
use libc;
use p9ds::error::P9Error;
use p9ds::proto::{Message, Partial, Rlerror};
use slog::{debug, trace, Logger};
use std::error::Error;
use std::io;
use std::marker::Sync;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::{
    fs::{File, OpenOptions},
    net::UnixStream,
};

#[async_trait]
pub trait Client {
    async fn connect(&mut self) -> Result<(), Box<dyn Error>>;
    async fn send<T, R>(&mut self, t: &T) -> Result<R, Box<dyn Error>>
    where
        T: std::fmt::Debug + serde::Serialize + Sync,
        R: std::fmt::Debug + serde::de::DeserializeOwned + Message;
}

fn read_msg<R>(data: &[u8]) -> Result<R, Box<dyn Error>>
where
    R: std::fmt::Debug + serde::de::DeserializeOwned + Message,
{
    // TODO: inefficient, this means we parse the first part of each message
    // up to 3 times
    let p: Partial = from_bytes_le(data)?;
    if p.instance_type() != R::message_type() {
        if p.instance_type() == Rlerror::message_type() {
            let e: Rlerror = from_bytes_le(data)?;
            let c_msg = unsafe { libc::strerror(e.ecode as i32) };
            let c_str = unsafe { std::ffi::CStr::from_ptr(c_msg) };
            let str_slice = c_str.to_str()?;
            let msg = str_slice.to_owned();

            return Err(Box::new(P9Error::ServerError(e, msg)));
        }
        return Err(Box::new(P9Error::UnexpectedReturnType(
            R::message_type(),
            p.instance_type(),
        )));
    }

    let r: R = from_bytes_le(data)?;
    Ok(r)
}

// Unix client ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

pub struct UnixClient {
    pub unix_sock: PathBuf,
    pub log: Logger,
    connection: Option<UnixStream>,
}

impl UnixClient {
    pub fn new(unix_sock: PathBuf, log: Logger) -> Self {
        UnixClient {
            unix_sock,
            log,
            connection: None,
        }
    }
}

#[async_trait]
impl Client for UnixClient {
    async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        self.connection = Some(UnixStream::connect(&self.unix_sock).await?);
        Ok(())
    }

    async fn send<T, R>(&mut self, t: &T) -> Result<R, Box<dyn Error>>
    where
        T: std::fmt::Debug + serde::Serialize + Sync,
        R: std::fmt::Debug + serde::de::DeserializeOwned + Message,
    {
        debug!(self.log, "→ {:#?}", t);

        let stream = match &self.connection {
            Some(s) => s,
            None => {
                self.connect().await?;
                self.connection.as_ref().unwrap()
            }
        };

        loop {
            stream.writable().await?;
            let out = to_bytes_le(t)?;
            match stream.try_write(out.as_slice()) {
                Ok(n) => {
                    debug!(self.log, "wrote {}", n);
                    break;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        let mut msg = Vec::new();
        loop {
            let mut buf = [0; 1024];

            stream.readable().await?;
            match stream.try_read(&mut buf) {
                Ok(0) => {
                    debug!(self.log, "eof");
                    break;
                }
                Ok(n) => {
                    debug!(self.log, "read {}", n);
                    msg.extend_from_slice(&buf[0..n]);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => return Err(e.into()),
            }
        }

        let r: R = match read_msg(msg.as_slice()) {
            Ok(r) => r,
            Err(e) => {
                trace!(self.log, "{:?}", msg.as_slice());
                return Err(e);
            }
        };
        debug!(self.log, "← {:?}", r);
        Ok(r)
    }
}

// Chardev client ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

pub struct ChardevClient {
    pub dev: PathBuf,
    pub log: Logger,
    file: Option<File>,
}

impl ChardevClient {
    pub fn new(dev: PathBuf, log: Logger) -> Self {
        ChardevClient {
            dev,
            log,
            file: None,
        }
    }
}

#[async_trait]
impl Client for ChardevClient {
    async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        self.file = Some(
            OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_EXCL)
                .open(&self.dev)
                .await?,
        );
        Ok(())
    }

    async fn send<T, R>(&mut self, t: &T) -> Result<R, Box<dyn Error>>
    where
        T: std::fmt::Debug + serde::Serialize + Sync,
        R: std::fmt::Debug + serde::de::DeserializeOwned + Message,
    {
        debug!(self.log, "→ {:#?}", t);

        let file = match &mut self.file {
            Some(ref mut f) => f,
            None => {
                self.connect().await?;
                self.file.as_mut().unwrap()
            }
        };

        let out = to_bytes_le(t)?;
        file.write_all(out.as_slice()).await?;

        trace!(self.log, "message sent");

        let mut msg = Vec::new();

        loop {
            // maximum message size of 8 kB
            let mut buf = [0u8; 8192];
            debug!(self.log, "waiting for data");
            let n = file.read(&mut buf).await?;
            debug!(self.log, "read {} bytes", n);
            msg.extend_from_slice(&buf[0..n]);
            if n < 0xF000 {
                break;
            }
        }

        let r: R = match read_msg(msg.as_slice()) {
            Ok(r) => r,
            Err(e) => {
                trace!(self.log, "{:?}", msg.as_slice());
                return Err(e);
            }
        };
        debug!(self.log, "← {:?}", r);
        Ok(r)
    }
}
