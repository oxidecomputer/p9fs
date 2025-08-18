// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// Copyright 2022 Oxide Computer Company

use async_recursion::async_recursion;
use clap::{AppSettings, Parser};
use devinfo::{get_devices, DiPropValue};
use p9ds::proto::{
    OpenFlags, P9Version, QidType, Rattach, Rlopen, Rread, Rreaddir, Rwalk,
    Tattach, Tlopen, Tread, Treaddir, Twalk, Version, Wname, NO_AFID,
    NO_NUNAME,
};
use p9kp::{ChardevClient, Client, UnixClient};
use slog::{info, Drain, Logger};
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::marker::Send;
use std::path::PathBuf;

const HEADER_SPACE: u32 = 11;

#[derive(Parser)]
#[clap(
    version = "0.1.0",
    author = "Ryan Goodfellow <ryan.goodfellow@oxide.computer>"
)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,

    #[clap(short, long, default_value_t = 65536)]
    chunk_size: u32,
}

#[derive(Parser)]
enum SubCommand {
    Pull(Pull),
}

#[derive(Parser)]
#[clap(setting = AppSettings::InferSubcommands)]
struct Pull {
    /// Connect to a unix domain socket. If not specified the program will
    /// use the first virtio filesystem device it can find.
    conn_str: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_envlogger::new(drain).fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let log = Logger::root(drain, slog::o!());

    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Pull(ref p) => pull(&opts, p, &log).await,
    }
}

async fn pull(
    opts: &Opts,
    p: &Pull,
    log: &Logger,
) -> Result<(), Box<dyn Error>> {
    match p.conn_str {
        None => {
            let mut client = find_virtfs_dev(log).await?;
            run(opts, &mut client, log).await?;
        }
        Some(ref conn_str) => {
            let pb = PathBuf::from(conn_str);
            let mut client = UnixClient::new(pb, log.clone());
            run(opts, &mut client, log).await?;
        }
    };

    Ok(())
}

async fn find_virtfs_dev(
    log: &Logger,
) -> Result<ChardevClient, Box<dyn Error>> {
    let devices = get_devices(false)?;

    // look for libvirt/vritfs device
    let vendor_id = 0x1af4;
    let device_id = 0x1009;

    for (device_key, dev_info) in devices {
        let vendor_match = match dev_info.props.get("vendor-id") {
            Some(value) => value.matches_int(vendor_id),
            _ => false,
        };
        let dev_match = match dev_info.props.get("device-id") {
            Some(value) => value.matches_int(device_id),
            _ => false,
        };
        let unit_address = match dev_info.props.get("unit-address") {
            Some(DiPropValue::Strings(vs)) => {
                if vs.is_empty() {
                    continue;
                }
                vs[0].clone()
            }
            _ => continue,
        };
        if vendor_match && dev_match {
            let dev_path = format!(
                "/devices/pci@0,0/{}@{}:9p",
                device_key.node_name, unit_address,
            );
            info!(log, "trying path {} ...", dev_path);
            let pb = PathBuf::from(dev_path);
            let mut client = p9kp::ChardevClient::new(pb, 0x10000, log.clone());

            let mut ver = Version::new(P9Version::V2000L);
            ver.msize = 0x10000;
            let server_version =
                client.send::<Version, Version>(&ver).await.unwrap();
            if Some(P9Version::V2000L)
                == P9Version::from_str(&server_version.version)
            {
                info!(log, "compatible 9p device found");
                return Ok(client);
            } else {
                info!(
                    log,
                    "not a compatible 9p device: {}", server_version.version
                );
            }
            // keep looking ...
        }
    }
    Err("suitable 9pfs device not found".into())
}

async fn run<C: Client + Send>(
    opts: &Opts,
    client: &mut C,
    log: &Logger,
) -> Result<(), Box<dyn Error>> {
    let mut ver = Version::new(P9Version::V2000L);
    ver.msize = opts.chunk_size;
    client.send::<Version, Version>(&ver).await?;

    let attach = Tattach::new(
        1,
        NO_AFID,
        "root".into(),
        "/todo".into(), //TODO not really used
        NO_NUNAME,
    );
    client.send::<Tattach, Rattach>(&attach).await?;

    let walk = Twalk::new(1, 2, Vec::new());
    client.send::<Twalk, Rwalk>(&walk).await?;

    let open = Tlopen::new(2, OpenFlags::RdOnly as u32);
    client.send::<Tlopen, Rlopen>(&open).await?;

    let mut offset = 0;
    let fid = 2;
    let mut nextfid = 3;
    let max_msg_size = opts.chunk_size - HEADER_SPACE;
    loop {
        let readdir = Treaddir::new(2, offset, max_msg_size);
        let resp = client.send::<Treaddir, Rreaddir>(&readdir).await?;
        if resp.data.is_empty() {
            break;
        }
        offset += resp.data.len() as u64;

        let path = PathBuf::from(".");
        copydir(client, opts, resp, "".into(), fid, &mut nextfid, log, path)
            .await?;
        if readdir.size < max_msg_size {
            break;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[async_recursion]
async fn copydir<C>(
    client: &mut C,
    opts: &Opts,
    readdir: Rreaddir,
    indent: String,
    fid: u32,
    nextfid: &mut u32,
    log: &Logger,
    path: PathBuf,
) -> Result<(), Box<dyn Error>>
where
    C: Client + Send,
{
    for entry in readdir.data {
        let attrs = match entry.qid.typ {
            QidType::Dir => "d",
            _ => "-",
        };
        info!(log, "{}  {}{}", attrs, indent, entry.name);

        // QEMU only sets entry.typ to the real value and uses glibc extension
        // types (DT_*) to identify the entry type.
        if entry.qid.typ == QidType::Dir || entry.typ == libc::DT_DIR {
            if entry.name == "." || entry.name == ".." {
                continue;
            }

            let newfid = *nextfid;
            let w = Twalk::new(
                fid,
                newfid,
                vec![Wname {
                    value: entry.name.clone(),
                }],
            );
            *nextfid += 1;
            client.send::<Twalk, Rwalk>(&w).await?;

            let o = Tlopen::new(newfid, OpenFlags::RdOnly as u32);
            client.send::<Tlopen, Rlopen>(&o).await?;

            let mut offset = 0;
            loop {
                let chunk_size = opts.chunk_size - HEADER_SPACE;
                let readdir = Treaddir::new(newfid, offset, chunk_size);
                let d = client.send::<Treaddir, Rreaddir>(&readdir).await?;
                if d.data.is_empty() {
                    break;
                }
                offset += d.data.len() as u64;

                let mut fp = path.clone();
                fp.push(entry.name.clone());
                std::fs::create_dir_all(format!("{}", fp.display()))?;

                copydir(
                    client,
                    opts,
                    d,
                    format!("  {indent}"),
                    newfid,
                    nextfid,
                    log,
                    fp.clone(),
                )
                .await?;

                if readdir.size < chunk_size {
                    break;
                }
            }
        } else if entry.qid.typ == QidType::File {
            copyfile(
                entry.name.clone(),
                opts,
                client,
                fid,
                nextfid,
                log,
                path.clone(),
            )
            .await?;
        }
    }
    Ok(())
}

async fn copyfile<C: Client>(
    name: String,
    opts: &Opts,
    client: &mut C,
    fid: u32,
    nextfid: &mut u32,
    _log: &Logger,
    path: PathBuf,
) -> Result<(), Box<dyn Error>> {
    let newfid = *nextfid;
    let walk = Twalk::new(
        fid,
        newfid,
        vec![Wname {
            value: name.clone(),
        }],
    );
    *nextfid += 1;
    client.send::<Twalk, Rwalk>(&walk).await?;

    let open = Tlopen::new(newfid, OpenFlags::RdOnly as u32);
    client.send::<Tlopen, Rlopen>(&open).await?;

    let mut fp = path.clone();
    fp.push(name.clone());

    let mut file = OpenOptions::new().create(true).append(true).open(fp)?;

    file.set_len(0)?; //truncate any existing content

    let mut offset = 0;
    loop {
        let r = Tread::new(newfid, offset, opts.chunk_size - HEADER_SPACE);
        let f = client.send::<Tread, Rread>(&r).await?;
        if f.data.is_empty() {
            break;
        }
        offset += f.data.len() as u64;

        file.write_all(f.data.as_slice())?;
    }

    Ok(())
}
