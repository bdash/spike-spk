use std::path::Path;

use crate::{
    cas::ContentAddressableStore,
    link::{LinkResult, LinkStrategy},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteResult {
    Direct,
    Cas { cached: bool, method: LinkResult },
}

pub trait FileSystem {
    fn write_file(
        &mut self,
        path: &Path,
        data: &[u8],
        mode: u32,
        hmac: &[u8; 20],
    ) -> anyhow::Result<WriteResult>;
    fn create_dir_all(&mut self, path: &Path) -> anyhow::Result<()>;
}

pub struct DirectFileSystem;

impl FileSystem for DirectFileSystem {
    fn write_file(
        &mut self,
        path: &Path,
        data: &[u8],
        mode: u32,
        _hmac: &[u8; 20],
    ) -> anyhow::Result<WriteResult> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, data)?;
        std::fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(mode))?;
        Ok(WriteResult::Direct)
    }

    fn create_dir_all(&mut self, path: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(path)?;
        Ok(())
    }
}

pub struct CasFileSystem {
    cas: ContentAddressableStore,
    link_strategy: LinkStrategy,
}

impl CasFileSystem {
    #[must_use]
    pub fn new(cas: ContentAddressableStore, link_strategy: LinkStrategy) -> Self {
        Self { cas, link_strategy }
    }
}

impl FileSystem for CasFileSystem {
    fn write_file(
        &mut self,
        path: &Path,
        data: &[u8],
        mode: u32,
        hmac: &[u8; 20],
    ) -> anyhow::Result<WriteResult> {
        // Use HMAC as the content hash (already validated)
        let hash = hex::encode(hmac);

        // Check if already in CAS
        let cas_path = self.cas.get_path(&hash);
        let cached = cas_path.exists();

        // Store in CAS if needed
        let stored_path = self.cas.ensure_stored(&hash, data)?;

        // Link to destination
        let method = self.link_strategy.link(&stored_path, path)?;

        // Set permissions on destination
        std::fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(mode))?;

        Ok(WriteResult::Cas { cached, method })
    }

    fn create_dir_all(&mut self, path: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(path)?;
        Ok(())
    }
}
