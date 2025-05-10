use std::{
    ffi::OsStr,
    fs::File,
    io::{Cursor, Read, SeekFrom, Write},
    path::Path,
};

use backhand::{FilesystemReader, InnerNode};
use binrw::{BinRead, FilePtr32, NullString, PosValue, binread};

use hmac::{self, Mac as _};
use md5::{Digest, digest::generic_array::GenericArray};
use sha1;

const HMAC_KEY: &'static [u8] = &[
    0x8e, 0x1f, 0x55, 0x43, 0xc2, 0xf5, 0x4a, 0x11, 0x67, 0x3a, 0x28, 0x2a, 0x2f, 0x87, 0xc0, 0x06,
];

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(repr(u8))]
enum PackageType {
    Spike1 = 1,
    Spike2 = 3,
    Game = 2,
}

impl PackageType {
    fn path_prefix(&self) -> &str {
        if self == &PackageType::Game {
            "/games/"
        } else {
            "/"
        }
    }
}

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(magic = b"SPKS")]
struct SPKS {
    byte_len: u32,
    chunk_count: u32,
}

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(magic = b"SPK0")]
struct SPK0 {
    byte_len: u32,
}

#[derive(BinRead, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(magic = b"SIDX")]
struct SIDX {
    byte_len: u32,
    package_name: [u8; 0x20],
    major_version: u8,
    minor_version: u8,
    patch_version: u8,
    package_type: PackageType,
    unknown_b: [u8; 0xc],
}

#[derive(BinRead, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(magic = b"STRS")]
struct STRS {
    byte_len: u32,
    #[br(count(byte_len))]
    string_data: Vec<u8>,
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
struct FINF {
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

#[derive(BinRead, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(magic = b"FEND")]
struct FEND {
    #[br(assert(byte_len == 0))]
    byte_len: u32,
}

#[derive(BinRead, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[br(magic = b"SDAT")]
struct SDAT {
    byte_len: u32,
}

#[derive(BinRead, Debug, Clone, PartialEq, Eq)]
#[br(import(strs_offset: u64))]
enum Chunk {
    SPKS(SPKS),
    SPK0(SPK0),
    SIDX(SIDX),
    STRS(STRS),
    FINF(#[br(args(strs_offset))] FINF),
    FEND(FEND),
}

fn verify<R>(mut reader: R) -> Result<(), Box<dyn std::error::Error>>
where
    R: std::io::Read + std::io::Seek,
{
    let spks = SPKS::read_le(&mut reader)?;

    for i in 0..spks.chunk_count {
        let spk0 = PosValue::<SPK0>::read_le(&mut reader)?;
        let sidx = SIDX::read_le(&mut reader)?;
        if i > 0 {
            println!("\n");
        }

        println!("Package: {}", std::str::from_utf8(&sidx.package_name)?);
        println!(
            "Version: {}.{}.{}",
            sidx.major_version, sidx.minor_version, sidx.patch_version
        );

        let strs = PosValue::<STRS>::read_le(&mut reader)?;
        let mut files = vec![];
        while let Chunk::FINF(finf) = Chunk::read_le_args(&mut reader, (strs.pos + 8,))? {
            files.push(finf);
        }

        let sdat = PosValue::<SDAT>::read_le(&mut reader)?;

        for file_info in files {
            print!(
                "{:165} offset={:10} size={:10}  ",
                format!("{}{}", sidx.package_type.path_prefix(), file_info.filename),
                file_info.data_offset,
                file_info.file_size
            );
            std::io::stdout().flush()?;

            let mut file_contents = vec![0; file_info.data_size as usize];
            let offset = sdat.pos + 8 + file_info.data_offset as u64;
            reader.seek(SeekFrom::Start(offset))?;
            reader.read_exact(&mut file_contents)?;

            let md5_digest = md5::Md5::digest(&file_contents);
            if md5_digest == file_info.data_md5.into() {
                print!("md5: ✔  ");
            } else {
                print!("md5: ✗  ");
            }
            std::io::stdout().flush()?;

            let mut sha1_hmac = hmac::Hmac::<sha1::Sha1>::new_from_slice(HMAC_KEY)?;
            sha1_hmac.update(&file_contents);
            let sha1_hmac_digest = sha1_hmac.finalize().into_bytes();
            if sha1_hmac_digest == file_info.data_hmac.into() {
                println!("hmac: ✔");
            } else {
                println!("hmac: ✗");
            }
        }

        // The next SPK0 starts at `offset`.
        let offset = spk0.pos + 8 + spk0.byte_len as u64;
        reader.seek(SeekFrom::Start(offset))?;
    }

    Ok(())
}

fn verify_squashed(file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let pattern = format!("{}.*", file_path.with_extension("").to_str().unwrap());
    print!("Loading SquashFS file system from {pattern}...");
    std::io::stdout().flush()?;

    let mut paths: Vec<_> = glob::glob(&pattern)?
        .flat_map(|p| p.map(|p| p.to_str().unwrap().to_owned()))
        .into_iter()
        .collect();
    paths.sort();

    let mut buffer: Vec<u8> = Vec::new();
    for path in paths {
        buffer.extend(std::fs::read(path.to_string())?);
    }
    let mut reader = Cursor::new(&*buffer);
    println!(" done!");

    let filesystem = FilesystemReader::from_reader(&mut reader)?;
    let Some(
        spk_file_node @ backhand::Node {
            inner: InnerNode::File(spk_file, ..),
            ..
        },
    ) = filesystem
        .files()
        .filter(|n| {
            if let backhand::InnerNode::File(_) = n.inner {
                true
            } else {
                false
            }
        })
        .next()
    else {
        return Err("No files found within SquashFS file system")?;
    };

    if Path::new(&spk_file_node.fullpath)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        != "spk"
    {
        return Err("SquashFS file system did not contain a single .spk file as expected")?;
    }

    print!(
        "Reading {} from SquashFS file system...",
        spk_file_node.fullpath.to_str().unwrap()
    );
    std::io::stdout().flush()?;

    let mut spk_file_reader = filesystem.file(&spk_file).reader();
    let mut spk_file_contents = vec![];
    spk_file_contents.reserve_exact(spk_file.file_len() as usize);
    spk_file_reader.read_to_end(&mut spk_file_contents)?;
    println!(" done!");
    println!();

    verify(Cursor::new(spk_file_contents))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        return Err(Box::from("usage: spike2-spk <path>"));
    }

    let file_name = &args[1];
    let path = Path::new(file_name);
    match path.extension().and_then(OsStr::to_str) {
        Some("spk") => {
            let mut file = File::open(file_name)?;
            verify(&mut file)
        }
        Some("000") => verify_squashed(path),
        None | Some(_) => Err("Unknown file type")?,
    }
}
