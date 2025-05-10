use std::{
    ffi::OsStr,
    io::Cursor,
    path::Path,
    sync::{Arc, Mutex},
};

use binrw::{BinRead, PosValue};
use thiserror::Error;

use crate::{chunks, squashed};

pub(crate) const HMAC_KEY: &'static [u8] = &[
    0x8e, 0x1f, 0x55, 0x43, 0xc2, 0xf5, 0x4a, 0x11, 0x67, 0x3a, 0x28, 0x2a, 0x2f, 0x87, 0xc0, 0x06,
];

#[derive(Error, Debug)]
pub enum OpenError {
    #[error("Failed to read file: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Failed to parse file: {0}")]
    Parse(#[from] binrw::Error),
    #[error("File name contained invalid UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Failed to read SquashFS file: {0}")]
    SquashFS(#[from] squashed::Error),
    #[error("Invalid file name: {0}")]
    GlobError(#[from] glob::PatternError),
    #[error("Unknown file type")]
    UnknownFileType,
    #[error("Directory does not appear to contain a split SPK file")]
    DirectoryDoesNotContainSplitSPK,
}

#[derive(Error, Debug)]
pub enum ReadError {
    #[error("Failed to read file: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Failed to parse file: {0}")]
    Parse(#[from] binrw::Error),
}

trait SeekableReader: std::io::Read + std::io::Seek + Send {}
impl<T> SeekableReader for T where T: std::io::Read + std::io::Seek + Send {}

pub struct SPKFile<'a> {
    pub packages: Vec<Package>,
    reader: Arc<Mutex<dyn SeekableReader + 'a>>,
}

impl<'a> std::fmt::Debug for SPKFile<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File")
            .field("packages", &self.packages)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub version: (u8, u8, u8),
    pub type_: chunks::PackageType,
    pub files: Vec<FileInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub(crate) offset: u64,
    pub(crate) data_size: u64,
    pub hmac: [u8; 20],
    pub md5: [u8; 16],
    pub mode: u16,
}

impl<'a> SPKFile<'a> {
    pub fn parse<R>(mut reader: R) -> Result<Self, OpenError>
    where
        R: std::io::Read + std::io::Seek + Send + 'a,
    {
        let spks = chunks::SPKS::read_le(&mut reader)?;

        let mut packages = Vec::new();
        for _ in 0..spks.chunk_count {
            let spk0 = PosValue::<chunks::SPK0>::read_le(&mut reader)?;
            let sidx = chunks::SIDX::read_le(&mut reader)?;

            // TODO: It's unclear what this is used for.
            let _ = chunks::SZ64::read_le(&mut reader);

            let strs = PosValue::<chunks::STRS>::read_le(&mut reader)?;
            let mut files = Vec::new();
            loop {
                let file_info =
                    PosValue::<chunks::FileInfo>::read_le_args(&mut reader, (strs.pos + 8,))?;
                if let chunks::FileInfo::FEND(_) = file_info.val {
                    break;
                }

                let file_info: chunks::FI64 = file_info.val.try_into().unwrap();
                files.push(FileInfo {
                    name: file_info.filename.to_string(),
                    size: file_info.file_size,
                    offset: file_info.data_offset as u64,
                    data_size: file_info.data_size as u64,
                    mode: file_info.mode,
                    hmac: file_info.data_hmac,
                    md5: file_info.data_md5,
                });
            }

            let sdat = PosValue::<chunks::SDAT>::read_le(&mut reader)?;
            for file in &mut files {
                file.offset += sdat.pos + sdat.header_size();
            }

            let package = Package {
                name: std::str::from_utf8(&sidx.package_name)?
                    .trim_end_matches('\0')
                    .to_string(),
                version: (sidx.major_version, sidx.minor_version, sidx.patch_version),
                type_: sidx.package_type,
                files,
            };
            packages.push(package);

            // The next SPK0 starts at `offset`.
            let offset = spk0.pos + spk0.offset_to_next();
            reader.seek(std::io::SeekFrom::Start(offset))?;
        }

        Ok(Self {
            packages,
            reader: Arc::new(Mutex::new(reader)),
        })
    }

    pub fn open(path: &Path) -> Result<Self, OpenError> {
        if std::fs::metadata(path)?.is_dir() {
            let paths = glob::glob(&format!("{}/*.000", path.display()))?;
            let paths: Vec<_> = paths.flat_map(|p| p.ok()).collect();
            if paths.len() != 1 {
                Err(OpenError::DirectoryDoesNotContainSplitSPK)?;
            }
            return Self::open_split_squashed(&paths[0]);
        }

        match path.extension().and_then(OsStr::to_str) {
            Some("spk") => Self::open_single_file(path),
            Some("000") => Self::open_split_squashed(path),
            None | Some(_) => Err(OpenError::UnknownFileType)?,
        }
    }

    pub fn open_single_file(path: &Path) -> Result<Self, OpenError> {
        let file = std::fs::File::open(path)?;
        let reader = Box::new(file);
        Self::parse(reader)
    }

    pub fn open_split_squashed(path: &Path) -> Result<Self, OpenError> {
        let spk_file_data = squashed::extract_spk_file(path)?;
        Self::parse(Cursor::new(spk_file_data))
    }

    pub fn read(&self, file: &FileInfo) -> Result<Vec<u8>, ReadError> {
        let mut buf = vec![0; file.data_size as usize];
        let mut reader = self.reader.lock().unwrap();
        reader.seek(std::io::SeekFrom::Start(file.offset))?;
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }
}
