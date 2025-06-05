use std::{
    ffi::OsStr,
    io::{Cursor, Read as _},
    path::Path,
    result::Result,
};

use backhand::{FilesystemReader, InnerNode};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to read file: {0}")]
    IO(#[from] std::io::Error),
    #[error("Failed to load SquashFS file: {0}")]
    SquashFS(#[from] backhand::BackhandError),
    #[error("Invalid file name: {0}")]
    Glob(#[from] glob::PatternError),
    #[error("No files found within SquashFS file system")]
    NoFilesFound,
    #[error("SquashFS file system did not contain a single .spk file as expected")]
    SPKFileNotFound,
}

pub(crate) fn extract_spk_file(path: &Path) -> Result<Vec<u8>, Error> {
    let pattern = format!("{}.*", path.with_extension("").to_str().unwrap());

    let mut paths: Vec<_> = glob::glob(&pattern)?.filter_map(Result::ok).collect();
    paths.sort();

    let mut buffer: Vec<u8> = Vec::new();
    for path in paths {
        buffer.extend(std::fs::read(path)?);
    }
    let mut reader = Cursor::new(&*buffer);

    let filesystem = FilesystemReader::from_reader(&mut reader)?;
    let Some(
        spk_file_node @ backhand::Node {
            inner: InnerNode::File(spk_file, ..),
            ..
        },
    ) = filesystem
        .files()
        .find(|n| matches!(n.inner, backhand::InnerNode::File(_)))
    else {
        return Err(Error::NoFilesFound)?;
    };

    let Some("spk") = Path::new(&spk_file_node.fullpath)
        .extension()
        .and_then(OsStr::to_str)
    else {
        return Err(Error::SPKFileNotFound)?;
    };

    let mut spk_file_reader = filesystem.file(spk_file).reader();
    let mut spk_file_contents = vec![];
    spk_file_contents.reserve_exact(spk_file.file_len());
    spk_file_reader.read_to_end(&mut spk_file_contents)?;

    Ok(spk_file_contents)
}
