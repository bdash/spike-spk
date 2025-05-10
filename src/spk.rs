
use binrw::{BinRead, FilePtr32, FilePtr64, NullString, binread};
use md5::digest::generic_array::GenericArray;

pub(crate) const HMAC_KEY: &'static [u8] = &[
    0x8e, 0x1f, 0x55, 0x43, 0xc2, 0xf5, 0x4a, 0x11, 0x67, 0x3a, 0x28, 0x2a, 0x2f, 0x87, 0xc0, 0x06,
];

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(repr(u8))]
pub(crate) enum PackageType {
    Spike1 = 1,
    Spike2 = 3,
    Game = 2,
}

impl PackageType {
    pub(crate) fn path_prefix(&self) -> &str {
        if self == &PackageType::Game {
            "/games/"
        } else {
            "/"
        }
    }
}

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ByteLen {
    #[br(magic = 0xffff_ffffu32)]
    New(u64),
    Old(u32),
}

impl ByteLen {
    pub(crate) fn byte_len(&self) -> u64 {
        match self {
            ByteLen::New(byte_len) => *byte_len,
            ByteLen::Old(byte_len) => *byte_len as u64,
        }
    }

    pub(crate) fn header_size(&self) -> u64 {
        match self {
            ByteLen::New(_) => 16,
            ByteLen::Old(_) => 4,
        }
    }
}

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(magic = b"SPKS")]
pub(crate) struct SPKS {
    byte_length: ByteLen,
    pub chunk_count: u32,
}

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(magic = b"SPK0")]
pub(crate) struct SPK0 {
    byte_len: ByteLen,
}

impl SPK0 {
    pub(crate) fn byte_len(&self) -> u64 {
        self.byte_len.byte_len()
    }

    pub(crate) fn header_size(&self) -> u64 {
        self.byte_len.header_size()
    }
}

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(magic = b"SIDX")]
pub(crate) struct SIDX {
    pub byte_len: ByteLen,
    pub package_name: [u8; 0x20],
    pub major_version: u8,
    pub minor_version: u8,
    pub patch_version: u8,
    pub package_type: PackageType,
    unknown_b: [u8; 0xc],
}

#[derive(BinRead, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(magic = b"STRS")]
pub(crate) struct STRS {
    byte_len: u32,
    #[br(count(byte_len))]
    pub string_data: Vec<u8>,
}

impl std::fmt::Debug for STRS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("STRS")
            .field("byte_len", &self.byte_len)
            .field("string_data", &"...")
            .finish()
    }
}

#[binread]
#[derive(Clone, PartialEq, Eq)]
#[br(magic = b"FINF", import(strs_offset: u64))]
pub(crate) struct FINF {
    byte_len: u32,
    #[br(offset(strs_offset), parse_with = FilePtr32::parse, restore_position)]
    filename: NullString,

    #[br(temp)]
    _filename: u32,

    file_size: u32,

    // Relative to SDAT.
    data_offset: u32,
    data_size: u32,

    // TODO: What're these?
    unknown: [u8; 2],

    #[br(pad_before(3))]
    data_hmac: [u8; 20],
    #[br(pad_after(3))]
    data_md5: [u8; 16],
}

impl std::fmt::Debug for FINF {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FINF")
            .field("byte_len", &self.byte_len)
            .field("filename", &self.filename)
            .field("file_size", &self.file_size)
            .field("data_offset", &self.data_offset)
            .field("data_size", &self.data_size)
            .field("unknown", &self.unknown)
            .field(
                "data_hmac",
                &format_args!("{:02x}", GenericArray::from(self.data_hmac)),
            )
            .field(
                "data_md5",
                &format_args!("{:02x}", GenericArray::from(self.data_md5)),
            )
            .finish()
    }
}

#[binread]
#[derive(Clone, PartialEq, Eq)]
#[br(magic = b"FI64", import(strs_offset: u64))]
pub(crate) struct FI64 {
    byte_len: u32,

    #[br(offset(strs_offset), parse_with = FilePtr64::parse, restore_position)]
    pub filename: NullString,

    #[br(temp)]
    _filename: u64,

    pub file_size: u64,

    // Relative to SDAT.
    pub data_offset: u64,
    pub data_size: u64,

    // TODO: What're these?
    unknown: [u8; 2],

    #[br(pad_before(3))]
    pub data_hmac: [u8; 20],
    #[br(pad_after(7))]
    pub data_md5: [u8; 16],
}

impl std::fmt::Debug for FI64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FINF")
            .field("byte_len", &self.byte_len)
            .field("filename", &self.filename)
            .field("file_size", &self.file_size)
            .field("data_offset", &self.data_offset)
            .field("data_size", &self.data_size)
            .field("unknown", &self.unknown)
            .field(
                "data_hmac",
                &format_args!("{:02x}", GenericArray::from(self.data_hmac)),
            )
            .field(
                "data_md5",
                &format_args!("{:02x}", GenericArray::from(self.data_md5)),
            )
            .finish()
    }
}

#[derive(BinRead, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(magic = b"FEND")]
pub(crate) struct FEND {
    #[br(assert(byte_len == 0))]
    byte_len: u32,
}

#[derive(BinRead, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(magic = b"SDAT")]
pub(crate) struct SDAT {
    byte_len: ByteLen,
}

impl SDAT {
    #[allow(unused)]
    pub(crate) fn byte_len(&self) -> u64 {
        self.byte_len.byte_len()
    }

    pub(crate) fn header_size(&self) -> u64 {
        self.byte_len.header_size()
    }
}

#[derive(BinRead, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(magic = b"SZ64")]
pub(crate) struct SZ64 {
    byte_len: u32,
    unknown: u64,
}

#[derive(BinRead, Debug, Clone, PartialEq, Eq)]
#[br(import(strs_offset: u64))]
pub(crate) enum FileInfo {
    FINF(#[br(args(strs_offset))] FINF),
    FI64(#[br(args(strs_offset))] FI64),
    FEND(FEND),
}

impl TryFrom<FileInfo> for FI64 {
    type Error = Box<dyn std::error::Error>;

    fn try_from(file_info: FileInfo) -> Result<Self, Self::Error> {
        match file_info {
            FileInfo::FINF(finf) => Ok(FI64 {
                byte_len: finf.byte_len,
                filename: finf.filename,
                file_size: finf.file_size as u64,
                data_offset: finf.data_offset as u64,
                data_size: finf.data_size as u64,
                unknown: finf.unknown,
                data_hmac: finf.data_hmac,
                data_md5: finf.data_md5,
            }),
            FileInfo::FI64(fi64) => Ok(fi64),
            FileInfo::FEND(_) => Err("FEND".into()),
        }
    }
}
