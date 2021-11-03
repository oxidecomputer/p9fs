// Copyright 2021 Oxide Computer Company

use ispf;
use ispf::WireSize;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::{self, Display, Formatter};
use std::mem::size_of;

#[derive(Debug, PartialEq)]
#[repr(u32)]
pub enum OpenFlags {
    RdOnly,
    WrOnly,
    RdWr,
}

#[derive(Debug, PartialEq)]
pub enum P9Version {
    V2000,
    V2000U,
    V2000L,
}

impl P9Version {
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(self) -> String {
        match self {
            Self::V2000 => "9P2000".into(),
            Self::V2000U => "9P2000.U".into(),
            Self::V2000L => "9P2000.L".into(),
        }
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
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
    Rgetatter,
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
        write!(f, "{:?}", self)
    }
}

pub trait Message {
    fn message_type() -> MessageType;
    fn instance_type(&self) -> MessageType;
}

#[derive(
    Debug,
    PartialEq,
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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
        write!(f, "{:?}", self)
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Qid {
    pub typ: QidType,
    pub version: u32,
    pub path: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Wname {
    #[serde(with = "ispf::str_lv16")]
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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
