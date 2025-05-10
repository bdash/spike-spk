use std::{
    ffi::OsStr,
    io::{Cursor, Read, SeekFrom, Write},
    path::Path,
};

use backhand::{FilesystemReader, InnerNode};
use binrw::{BinRead, PosValue};
use hmac::{self, Mac as _};
use md5::Digest;
use sha1;

use crate::spk;

pub fn verify<R>(mut reader: R) -> Result<(), Box<dyn std::error::Error>>
where
    R: std::io::Read + std::io::Seek,
{
    let spks = spk::SPKS::read_le(&mut reader)?;

    for i in 0..spks.chunk_count {
        let spk0 = PosValue::<spk::SPK0>::read_le(&mut reader)?;
        let sidx = spk::SIDX::read_le(&mut reader)?;
        if i > 0 {
            println!("\n");
        }

        println!("Package: {}", std::str::from_utf8(&sidx.package_name)?);
        println!(
            "Version: {}.{}.{}",
            sidx.major_version, sidx.minor_version, sidx.patch_version
        );

        // TODO: It's unclear what this is used for.
        let _ = spk::SZ64::read_le(&mut reader);

        let strs = PosValue::<spk::STRS>::read_le(&mut reader)?;
        let mut files: Vec<spk::FI64> = vec![];
        loop {
            let file_info = PosValue::<spk::FileInfo>::read_le_args(&mut reader, (strs.pos + 8,))?;
            if let spk::FileInfo::FEND(_) = file_info.val {
                break;
            }

            files.push(file_info.val.try_into()?);
        }

        let sdat = PosValue::<spk::SDAT>::read_le(&mut reader)?;

        for file_info in files {
            print!(
                "{:165} offset={:10} size={:10}  ",
                format!("{}{}", sidx.package_type.path_prefix(), file_info.filename),
                file_info.data_offset,
                file_info.file_size
            );
            std::io::stdout().flush()?;

            let mut file_contents = vec![0; file_info.data_size as usize];
            let offset = sdat.pos + sdat.header_size() + file_info.data_offset as u64;
            reader.seek(SeekFrom::Start(offset))?;
            reader.read_exact(&mut file_contents)?;

            let md5_digest = md5::Md5::digest(&file_contents);
            if md5_digest == file_info.data_md5.into() {
                print!("md5: ✔  ");
            } else {
                print!("md5: ✗  ");
            }
            std::io::stdout().flush()?;

            let mut sha1_hmac = hmac::Hmac::<sha1::Sha1>::new_from_slice(spk::HMAC_KEY)?;
            sha1_hmac.update(&file_contents);
            let sha1_hmac_digest = sha1_hmac.finalize().into_bytes();
            if sha1_hmac_digest == file_info.data_hmac.into() {
                println!("hmac: ✔");
            } else {
                println!("hmac: ✗");
            }
        }

        // The next SPK0 starts at `offset`.
        let offset = spk0.pos + spk0.header_size() + spk0.byte_len();
        reader.seek(SeekFrom::Start(offset))?;
    }

    Ok(())
}

pub fn verify_squashed(file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
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
