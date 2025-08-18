// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// Copyright 2022 Oxide Computer Company

use ispf;
use ispf::WireSize;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::{self, Display, Formatter};
use std::mem::size_of;

#[derive(Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum OpenFlags {
    RdOnly,
    WrOnly,
    RdWr,
}

#[derive(Debug, PartialEq, Eq)]
pub enum P9Version {
    V2000,
    V2000U,
    V2000L,
    V2000P4,
}

impl P9Version {
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(self) -> String {
        match self {
            Self::V2000 => "9P2000".into(),
            Self::V2000U => "9P2000.U".into(),
            Self::V2000L => "9P2000.L".into(),
            Self::V2000P4 => "9P2000.P4".into(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "9P2000" => Some(Self::V2000),
            "9P2000.U" => Some(Self::V2000U),
            "9P2000.L" => Some(Self::V2000L),
            "9P2000.P4" => Some(Self::V2000P4),
            _ => None,
        }
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    TryFromPrimitive,
    IntoPrimitive,
)]
#[repr(u8)]
pub enum MessageType {
    Unknown = 0,
    Tlerror = 6,
    Rlerror,
    Tstatfs = 8,
    Rstatfs,
    Tlopen = 12,
    Rlopen,
    Tlcreate = 14,
    Rlcreate,
    Tsymlink = 16,
    Rsymlink,
    Tmknod = 18,
    Rmknod,
    Trename = 20,
    Rrename,
    Treadlink = 22,
    Rreadlink,
    Tgetattr = 24,
    Rgetattr,
    Txattrwalk = 30,
    Rxattrwalk,
    Treaddir = 40,
    Rreaddir,
    Tfsync = 50,
    Rfsync,
    Tlock = 52,
    Rlock,
    Tgetlock = 54,
    Rgetlock,
    Tlink = 70,
    Rlink,
    Tmkdir = 72,
    Rmkdir,
    Trenameat = 74,
    Rrenameat,
    Tunlinkat = 76,
    Runlinkat,
    Tversion = 100,
    Rversion,
    Tauth = 102,
    Rauth,
    Tattach = 104,
    Rattach,
    Tflush = 108,
    Rflush,
    Twalk = 110,
    Rwalk,
    Tread = 116,
    Rread,
    Twrite = 118,
    Rwrite,
    Tclunk = 120,
    Rclunk,
    Tremove = 122,
    Rremove,
}

impl Display for MessageType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

pub trait Message {
    fn message_type() -> MessageType;
    fn instance_type(&self) -> MessageType;
}

#[derive(
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    TryFromPrimitive,
    IntoPrimitive,
)]
#[repr(u8)]
pub enum QidType {
    Dir = 0x80,
    Append = 0x40,
    Excl = 0x20,
    Mount = 0x10,
    Auth = 0x08,
    Tmp = 0x04,
    Link = 0x02,
    File = 0x00,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Partial {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
}

impl Message for Partial {
    fn instance_type(&self) -> MessageType {
        self.typ
    }
    fn message_type() -> MessageType {
        MessageType::Unknown
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rlerror {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub ecode: u32,
}

impl Rlerror {
    pub fn new(ecode: u32) -> Self {
        Rlerror {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>()  +
                // tag
                size_of::<u16>() +
                // ecode
                size_of::<u32>()
            ) as u32,
            typ: MessageType::Rlerror,
            tag: 0,
            ecode,
        }
    }
}

impl Display for Rlerror {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Message for Rlerror {
    fn instance_type(&self) -> MessageType {
        self.typ
    }
    fn message_type() -> MessageType {
        MessageType::Rlerror
    }
}

pub const NO_FID: u32 = !0u32;
pub const NO_AFID: u32 = !0u32;
pub const NO_NUNAME: u32 = !0u32;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Version {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub msize: u32,
    #[serde(with = "ispf::str_lv16")]
    pub version: String,
}

impl Message for Version {
    fn instance_type(&self) -> MessageType {
        self.typ
    }
    fn message_type() -> MessageType {
        MessageType::Rversion
    }
}

impl Version {
    pub fn new(v: P9Version) -> Self {
        let vs = v.to_string();
        Version {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>() +
                // tag
                size_of::<u16>() +
                // msize
                size_of::<u32>() +
                // version.size
                size_of::<u16>() +
                // version
                vs.len()
            ) as u32,
            typ: MessageType::Tversion,
            tag: 0,
            msize: 0x8000, //32 kB default
            version: vs,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tclunk {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub fid: u32,
}

impl Tclunk {
    pub fn new(fid: u32) -> Self {
        Tclunk {
            size: (
                //size
                size_of::<u32>() +
                // typ
                size_of::<u8>() +
                // tag
                size_of::<u16>() +
                // fid
                size_of::<u32>()
            ) as u32,
            typ: MessageType::Tclunk,
            tag: 0,
            fid,
        }
    }
}

impl Message for Tclunk {
    fn instance_type(&self) -> MessageType {
        self.typ
    }
    fn message_type() -> MessageType {
        MessageType::Tclunk
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rclunk {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
}

impl Rclunk {
    pub fn new() -> Self {
        Rclunk {
            size: (
                //size
                size_of::<u32>() +
                // typ
                size_of::<u8>() +
                // tag
                size_of::<u16>()
            ) as u32,
            typ: MessageType::Rclunk,
            tag: 0,
        }
    }
}

impl Message for Rclunk {
    fn instance_type(&self) -> MessageType {
        self.typ
    }
    fn message_type() -> MessageType {
        MessageType::Rclunk
    }
}

impl Default for Rclunk {
    fn default() -> Self {
        Self::new()
    }
}

/*
size[4] Tgetattr tag[2] fid[4] request_mask[8]
*/
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tgetattr {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub fid: u32,
    pub request_mask: u64,
}

impl Tgetattr {
    pub fn new(fid: u32, request_mask: u64) -> Self {
        Tgetattr {
            size: (
                //size
                size_of::<u32>() +
                // typ
                size_of::<u8>() +
                // tag
                size_of::<u16>() +
                // fid
                size_of::<u32>() +
                // mask
                size_of::<u64>()
            ) as u32,
            typ: MessageType::Tgetattr,
            tag: 0,
            fid,
            request_mask,
        }
    }
}

pub const P9_GETATTR_MODE: u64 = 0x00000001;
pub const P9_GETATTR_NLINK: u64 = 0x00000002;
pub const P9_GETATTR_UID: u64 = 0x00000004;
pub const P9_GETATTR_GID: u64 = 0x00000008;
pub const P9_GETATTR_RDEV: u64 = 0x00000010;
pub const P9_GETATTR_ATIME: u64 = 0x00000020;
pub const P9_GETATTR_MTIME: u64 = 0x00000040;
pub const P9_GETATTR_CTIME: u64 = 0x00000080;
pub const P9_GETATTR_INO: u64 = 0x00000100;
pub const P9_GETATTR_SIZE: u64 = 0x00000200;
pub const P9_GETATTR_BLOCKS: u64 = 0x00000400;

pub const P9_GETATTR_BTIME: u64 = 0x00000800;
pub const P9_GETATTR_GEN: u64 = 0x00001000;
pub const P9_GETATTR_DATA_VERSION: u64 = 0x00002000;

pub const P9_GETATTR_BASIC: u64 = 0x000007ff; /* Mask for fields up to BLOCKS */
pub const P9_GETATTR_ALL: u64 = 0x00003fff; /* Mask for All fields above */

/*
size[4] Rgetattr
    tag[2]
    valid[8]
    qid[13]
    mode[4]
    uid[4]
    gid[4]
    nlink[8]
    rdev[8]
    size[8]
    blksize[8]
    blocks[8]
    atime_sec[8]
    atime_nsec[8]
    mtime_sec[8]
    mtime_nsec[8]
    ctime_sec[8]
    ctime_nsec[8]
    btime_sec[8]
    btime_nsec[8]
    gen[8]
    data_version[8]
*/
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rgetattr {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub valid: u64,
    pub qid: Qid,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub nlink: u64,
    pub rdev: u64,
    pub attrsize: u64,
    pub blksize: u64,
    pub blocks: u64,
    pub atime_sec: u64,
    pub atime_nsec: u64,
    pub mtime_sec: u64,
    pub mtime_nsec: u64,
    pub ctime_sec: u64,
    pub ctime_nsec: u64,
    pub btime_sec: u64,
    pub btime_nsec: u64,
    pub gen: u64,
    pub data_version: u64,
}

impl Rgetattr {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        valid: u64,
        qid: Qid,
        mode: u32,
        uid: u32,
        gid: u32,
        nlink: u64,
        rdev: u64,
        attrsize: u64,
        blksize: u64,
        blocks: u64,
        atime_sec: u64,
        atime_nsec: u64,
        mtime_sec: u64,
        mtime_nsec: u64,
        ctime_sec: u64,
        ctime_nsec: u64,
        btime_sec: u64,
        btime_nsec: u64,
        gen: u64,
        data_version: u64,
    ) -> Self {
        Rgetattr {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>()  +
                // tag
                size_of::<u16>() +
                //valid
                size_of::<u64>() +
                // qid.typ
                size_of::<QidType>() +
                // qid.version
                size_of::<u32>() +
                // qid.path
                size_of::<u64>() +
                //  mode
                size_of::<u32>() +
                //  uid
                size_of::<u32>() +
                //  gid
                size_of::<u32>() +
                //  nlink
                size_of::<u64>() +
                //  rdev
                size_of::<u64>() +
                //  attrsize
                size_of::<u64>() +
                //  blksize
                size_of::<u64>() +
                //  blocks
                size_of::<u64>() +
                //  atime_sec
                size_of::<u64>() +
                //  atime_nsec
                size_of::<u64>() +
                //  mtime_sec
                size_of::<u64>() +
                //  mtime_nsec
                size_of::<u64>() +
                //  ctime_sec
                size_of::<u64>() +
                //  ctime_nsec
                size_of::<u64>() +
                //  btime_sec
                size_of::<u64>() +
                //  btime_nsec
                size_of::<u64>() +
                //  gen
                size_of::<u64>() +
                //  data_version
                size_of::<u64>()
            ) as u32,
            typ: MessageType::Rgetattr,
            tag: 0,
            valid,
            qid,
            mode,
            uid,
            gid,
            nlink,
            rdev,
            attrsize,
            blksize,
            blocks,
            atime_sec,
            atime_nsec,
            mtime_sec,
            mtime_nsec,
            ctime_sec,
            ctime_nsec,
            btime_sec,
            btime_nsec,
            gen,
            data_version,
        }
    }
}

/*
size[4] Tstatfs tag[2] fid[4]
*/
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tstatfs {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub fid: u32,
}

impl Tstatfs {
    pub fn new(fid: u32) -> Self {
        Tstatfs {
            size: (
                //size
                size_of::<u32>() +
                // typ
                size_of::<u8>() +
                // tag
                size_of::<u16>() +
                // fid
                size_of::<u32>()
            ) as u32,
            typ: MessageType::Tstatfs,
            tag: 0,
            fid,
        }
    }
}

/*
size[4] Rstatfs
    tag[2]
    type[4]
    bsize[4]
    blocks[8]
    bfree[8]
    bavail[8]
    files[8]
    ffree[8]
    fsid[8]
    namelen[4]
*/
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rstatfs {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub fstype: u32,
    pub bsize: u32,
    pub blocks: u64,
    pub bfree: u64,
    pub bavail: u64,
    pub files: u64,
    pub ffree: u64,
    pub fsid: u64,
    pub namelen: u32,
}

impl Rstatfs {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        fstype: u32,
        bsize: u32,
        blocks: u64,
        bfree: u64,
        bavail: u64,
        files: u64,
        ffree: u64,
        fsid: u64,
        namelen: u32,
    ) -> Self {
        Rstatfs {
            size: (
                //size
                size_of::<u32>() +
                // typ
                size_of::<u8>() +
                // tag
                size_of::<u16>() +
                // fstype
                size_of::<u32>() +
                // bsize
                size_of::<u32>() +
                // blocks
                size_of::<u64>() +
                // bfree
                size_of::<u64>() +
                // bavail
                size_of::<u64>() +
                // files
                size_of::<u64>() +
                // ffree
                size_of::<u64>() +
                // fsid
                size_of::<u64>() +
                // namelen
                size_of::<u32>()
            ) as u32,
            typ: MessageType::Rstatfs,
            tag: 0,
            fstype,
            bsize,
            blocks,
            bfree,
            bavail,
            files,
            ffree,
            fsid,
            namelen,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tattach {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub fid: u32,
    pub afid: u32,
    #[serde(with = "ispf::str_lv16")]
    pub uname: String,
    #[serde(with = "ispf::str_lv16")]
    pub aname: String,
    pub n_uname: u32,
}

impl Tattach {
    pub fn new(
        fid: u32,
        afid: u32,
        uname: String,
        aname: String,
        n_uname: u32,
    ) -> Self {
        Tattach {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>() +
                // tag
                size_of::<u16>() +
                // fid
                size_of::<u32>() +
                // afid
                size_of::<u32>() +
                // uname.size
                size_of::<u16>() +
                // uname
                uname.len() +
                // aname.size
                size_of::<u16>() +
                // aname
                aname.len() +
                // nuname
                size_of::<u32>()
            ) as u32,
            typ: MessageType::Tattach,
            tag: 0,
            fid,
            afid,
            uname,
            aname,
            n_uname,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rattach {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub qid: Qid,
}

impl Rattach {
    pub fn new(qid: Qid) -> Self {
        Rattach {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>()  +
                // tag
                size_of::<u16>() +
                // qid.typ
                size_of::<QidType>() +
                // qid.version
                size_of::<u32>() +
                // qid.path
                size_of::<u64>()
            ) as u32,
            typ: MessageType::Rattach,
            tag: 0,
            qid,
        }
    }
}

impl Message for Rattach {
    fn instance_type(&self) -> MessageType {
        self.typ
    }
    fn message_type() -> MessageType {
        MessageType::Rattach
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Qid {
    pub typ: QidType,
    pub version: u32,
    pub path: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Wname {
    #[serde(with = "ispf::str_lv16")]
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Twalk {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub fid: u32,
    pub newfid: u32,
    #[serde(with = "ispf::vec_lv16")]
    pub wname: Vec<Wname>,
}

impl Twalk {
    pub fn new(fid: u32, newfid: u32, wname: Vec<Wname>) -> Self {
        let mut wname_sz = 0usize;
        for x in &wname {
            // leading length u16 plus string
            wname_sz += size_of::<u16>() + x.value.len()
        }
        Twalk {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>()  +
                // tag
                size_of::<u16>() +
                // fid
                size_of::<u32>() +
                // newfid
                size_of::<u32>() +
                // wname.len
                size_of::<u16>() +
                wname_sz
            ) as u32,
            typ: MessageType::Twalk,
            tag: 0,
            fid,
            newfid,
            wname,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rwalk {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    #[serde(with = "ispf::vec_lv16")]
    pub wname: Vec<Qid>,
}

impl Rwalk {
    pub fn new(wname: Vec<Qid>) -> Self {
        let wname_sz = wname.len()
            * (size_of::<QidType>() + size_of::<u32>() + size_of::<u64>());
        Rwalk {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>()  +
                // tag
                size_of::<u16>() +
                // wname.len
                size_of::<u16>() +
                wname_sz
            ) as u32,
            typ: MessageType::Rwalk,
            tag: 0,
            wname,
        }
    }
}

impl Message for Rwalk {
    fn instance_type(&self) -> MessageType {
        self.typ
    }
    fn message_type() -> MessageType {
        MessageType::Rwalk
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tlopen {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub fid: u32,
    pub flags: u32,
}

impl Tlopen {
    pub fn new(fid: u32, flags: u32) -> Self {
        Tlopen {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>()  +
                // tag
                size_of::<u16>() +
                // fid
                size_of::<u32>() +
                // flags
                size_of::<u32>()
            ) as u32,
            typ: MessageType::Tlopen,
            tag: 0,
            fid,
            flags,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rlopen {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub qid: Qid,
    pub iounit: u32,
}

impl Rlopen {
    pub fn new(qid: Qid, iounit: u32) -> Self {
        Rlopen {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>()  +
                // tag
                size_of::<u16>() +
                // qid.typ
                size_of::<QidType>() +
                // qid.version
                size_of::<u32>() +
                // qid.path
                size_of::<u64>() +
                // iounit
                size_of::<u32>()
            ) as u32,
            typ: MessageType::Rlopen,
            tag: 0,
            qid,
            iounit,
        }
    }
}

impl Message for Rlopen {
    fn instance_type(&self) -> MessageType {
        self.typ
    }
    fn message_type() -> MessageType {
        MessageType::Rlopen
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Treaddir {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub fid: u32,
    pub offset: u64,
    pub count: u32,
}

impl Treaddir {
    pub fn new(fid: u32, offset: u64, count: u32) -> Self {
        Treaddir {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>()  +
                // tag
                size_of::<u16>() +
                // fid
                size_of::<u32>() +
                // offset
                size_of::<u64>() +
                // count
                size_of::<u32>()
            ) as u32,
            typ: MessageType::Treaddir,
            tag: 0,
            fid,
            offset,
            count,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rreaddir {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    #[serde(with = "ispf::vec_lv32b")]
    pub data: Vec<Dirent>,
}

impl Rreaddir {
    pub fn new(data: Vec<Dirent>) -> Self {
        let mut data_sz = 0usize;
        for x in &data {
            // leading length u16 plus string
            data_sz += x.wire_size();
        }

        Rreaddir {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>()  +
                // tag
                size_of::<u16>() +
                // data.len
                size_of::<u32>() +
                data_sz
            ) as u32,
            typ: MessageType::Rreaddir,
            tag: 0,
            data,
        }
    }
}

impl Message for Rreaddir {
    fn instance_type(&self) -> MessageType {
        self.typ
    }
    fn message_type() -> MessageType {
        MessageType::Rreaddir
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dirent {
    pub qid: Qid,
    pub offset: u64,
    pub typ: u8,
    #[serde(with = "ispf::str_lv16")]
    pub name: String,
}

impl ispf::WireSize for Dirent {
    fn wire_size(&self) -> usize {
        // qid.typ
        size_of::<QidType>() +
        // qid.version
        size_of::<u32>() +
        // qid.path
        size_of::<u64>() +
        // offset
        size_of::<u64>() +
        // typ
        size_of::<u8>() +
        // name.len TODO: awkward, user specifying
        //                serde inserted value
        size_of::<u16>() +
        // name
        self.name.len()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tread {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub fid: u32,
    pub offset: u64,
    pub count: u32,
}

impl Tread {
    pub fn new(fid: u32, offset: u64, count: u32) -> Self {
        Tread {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>()  +
                // tag
                size_of::<u16>() +
                // fid
                size_of::<u32>() +
                // offset
                size_of::<u64>() +
                // count
                size_of::<u32>()
            ) as u32,
            typ: MessageType::Tread,
            tag: 0,
            fid,
            offset,
            count,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rread {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    #[serde(with = "ispf::vec_lv32")]
    pub data: Vec<u8>,
}

impl Rread {
    pub fn new(data: Vec<u8>) -> Self {
        Rread {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>()  +
                // tag
                size_of::<u16>() +
                // data.count
                size_of::<u32>() +
                data.len()
            ) as u32,
            typ: MessageType::Rread,
            tag: 0,
            data,
        }
    }
}

impl Message for Rread {
    fn instance_type(&self) -> MessageType {
        self.typ
    }
    fn message_type() -> MessageType {
        MessageType::Rread
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Twrite {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub fid: u32,
    pub offset: u64,
    #[serde(with = "ispf::vec_lv32")]
    pub data: Vec<u8>,
}

impl Twrite {
    pub fn new(data: Vec<u8>, fid: u32, offset: u64) -> Self {
        Twrite {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>()  +
                // tag
                size_of::<u16>() +
                // fid
                size_of::<u32>() +
                // offset
                size_of::<u64>() +
                // data.count
                size_of::<u32>() +
                data.len()
            ) as u32,
            typ: MessageType::Twrite,
            tag: 0,
            fid,
            offset,
            data,
        }
    }
}

impl Message for Twrite {
    fn instance_type(&self) -> MessageType {
        self.typ
    }
    fn message_type() -> MessageType {
        MessageType::Twrite
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rwrite {
    pub size: u32,
    pub typ: MessageType,
    pub tag: u16,
    pub count: u32,
}

impl Rwrite {
    pub fn new(count: u32) -> Self {
        Rwrite {
            size: (
                // size
                size_of::<u32>() +
                // typ
                size_of::<u8>()  +
                // tag
                size_of::<u16>() +
                // fid
                size_of::<u32>()
            ) as u32,
            typ: MessageType::Rwrite,
            tag: 0,
            count,
        }
    }
}

impl Message for Rwrite {
    fn instance_type(&self) -> MessageType {
        self.typ
    }
    fn message_type() -> MessageType {
        MessageType::Rwrite
    }
}
