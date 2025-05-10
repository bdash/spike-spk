use std::{
    ffi::OsStr,
    io::{Cursor, Read as _},
    path::Path,
};

use backhand::{FilesystemReader, InnerNode};

pub(crate) fn extract_spk_file(
    path: &Path,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let pattern = format!("{}.*", path.with_extension("").to_str().unwrap());

    let mut paths: Vec<_> = glob::glob(&pattern)?
        .flat_map(|p| p.ok())
        .into_iter()
        .collect();
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

    let Some("spk") = Path::new(&spk_file_node.fullpath)
        .extension()
        .and_then(OsStr::to_str)
    else {
        return Err("SquashFS file system did not contain a single .spk file as expected")?;
    };

    let mut spk_file_reader = filesystem.file(&spk_file).reader();
    let mut spk_file_contents = vec![];
    spk_file_contents.reserve_exact(spk_file.file_len() as usize);
    spk_file_reader.read_to_end(&mut spk_file_contents)?;

    Ok(spk_file_contents)
}
