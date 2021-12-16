// Copyright 2021 Oxide Computer Company

use async_recursion::async_recursion;
use clap::{AppSettings, Parser};
use client::{ChardevClient, Client, UnixClient};
use devinfo::{get_devices, DiPropValue};
use p9ds::proto::{
    OpenFlags, P9Version, QidType, Rattach, Rlopen, Rread, Rreaddir, Rwalk,
    Tattach, Tlopen, Tread, Treaddir, Twalk, Version, Wname, NO_AFID,
    NO_NUNAME,
};
use slog::{info, Drain, Logger};
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::marker::Send;
use std::path::PathBuf;
use std::process::Command;

pub mod client;

const CHUNK_SIZE: u32 = 8192;
const MAX_MSG_SIZE: u32 = CHUNK_SIZE - 11;

#[derive(Parser)]
#[clap(
    version = "0.1.0",
    author = "Ryan Goodfellow <ryan.goodfellow@oxide.computer>"
)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Parser)]
enum SubCommand {
    Pull(Pull),
    LoadDriver(LoadDriver),
}

#[derive(Parser)]
#[clap(setting = AppSettings::InferSubcommands)]
struct Pull {
    /// Connect to a unix domain socket. If not specified the program will
    /// use the first virtio filesystem device it can find.
    conn_str: Option<String>,
}

#[derive(Parser)]
#[clap(setting = AppSettings::InferSubcommands)]
struct LoadDriver {}

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
        SubCommand::LoadDriver(ref l) => load_driver(&opts, l, &log).await,
    }
}

async fn pull(
    _opts: &Opts,
    p: &Pull,
    log: &Logger,
) -> Result<(), Box<dyn Error>> {
    match p.conn_str {
        None => {
            let dev = find_virtfs_dev(log)?;
            //TODO handle case when it's not pci@0,0
            let dev_path = format!(
                "/devices/pci@0,0/{}@{}:9p",
                dev.device_name, dev.unit_address,
            );
            info!(log, "using path {}", dev_path);
            let pb = PathBuf::from(dev_path);
            let mut client = ChardevClient::new(pb, log.clone());
            run(&mut client, log).await?;
        }
        Some(ref conn_str) => {
            let pb = PathBuf::from(conn_str);
            let mut client = UnixClient::new(pb, log.clone());
            run(&mut client, log).await?;
        }
    };

    Ok(())
}

async fn load_driver(
    _opts: &Opts,
    _l: &LoadDriver,
    log: &Logger,
) -> Result<(), Box<dyn Error>> {
    let dev = find_virtfs_dev(log)?;
    do_load_driver(&dev, log)
}

struct Virtio9pDevice {
    device_name: String,
    unit_address: String,
}

fn find_virtfs_dev(_log: &Logger) -> Result<Virtio9pDevice, Box<dyn Error>> {
    let devices = get_devices(false)?;

    // look for libvirt/vritfs device
    let vendor_id = 0x1af4;
    let device_id = 0x1009;

    let mut found = None;
    for (device_name, dev_info) in devices {
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
            found = Some(Virtio9pDevice {
                device_name,
                unit_address,
            });
            break;
        }
    }

    match found {
        Some(dev) => Ok(dev),
        None => Err(Box::new(io::Error::new(
            io::ErrorKind::NotFound,
            "virtio filesystem device not found",
        ))),
    }
}

fn do_load_driver(
    dev: &Virtio9pDevice,
    log: &Logger,
) -> Result<(), Box<dyn Error>> {
    info!(log, "loading vio9p for {}", dev.device_name);

    let out = Command::new("rem_drv").args(["vio9p"]).output()?;

    if !out.status.success() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!("rem_drv: {:?}", out),
        )));
    }

    let out = Command::new("add_drv")
        .args(["-i", dev.device_name.as_str(), "-v", "vio9p"])
        .output()?;

    if !out.status.success() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!("rem_drv: {:?}", out),
        )));
    }

    Ok(())
}

async fn run<C: Client + Send>(
    client: &mut C,
    log: &Logger,
) -> Result<(), Box<dyn Error>> {
    let mut ver = Version::new(P9Version::V2000L);
    ver.msize = CHUNK_SIZE;
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
    loop {
        let readdir = Treaddir::new(2, offset, MAX_MSG_SIZE);
        let resp = client.send::<Treaddir, Rreaddir>(&readdir).await?;
        if resp.data.is_empty() {
            break;
        }
        offset += resp.data.len() as u64;

        let path = PathBuf::from(".");
        copydir(client, resp, "".into(), fid, &mut nextfid, log, path).await?;
        if readdir.size < MAX_MSG_SIZE {
            break;
        }
    }

    Ok(())
}

#[async_recursion]
async fn copydir<C: Client + Send>(
    client: &mut C,
    readdir: Rreaddir,
    indent: String,
    fid: u32,
    nextfid: &mut u32,
    log: &Logger,
    path: PathBuf,
) -> Result<(), Box<dyn Error>> {
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
                let chunk_size = 8192 - 11;
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
                    d,
                    format!("  {}", indent),
                    newfid,
                    nextfid,
                    log,
                    fp.clone(),
                )
                .await?;

                if readdir.size < MAX_MSG_SIZE {
                    break;
                }
            }
        } else if entry.qid.typ == QidType::File {
            copyfile(
                entry.name.clone(),
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
        let r = Tread::new(newfid, offset, 8192 - 11 /*mini chunks*/);
        let f = client.send::<Tread, Rread>(&r).await?;
        if f.data.is_empty() {
            break;
        }
        offset += f.data.len() as u64;

        file.write_all(f.data.as_slice())?;
    }

    Ok(())
}
