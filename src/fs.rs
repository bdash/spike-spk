use std::path::Path;

pub trait FileSystem {
    fn write_file(&mut self, path: &Path, data: &[u8], mode: u32) -> anyhow::Result<()>;
    fn create_dir_all(&mut self, path: &Path) -> anyhow::Result<()>;
}

pub struct DirectFileSystem;

impl FileSystem for DirectFileSystem {
    fn write_file(&mut self, path: &Path, data: &[u8], mode: u32) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, data)?;
        std::fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(mode))?;
        Ok(())
    }

    fn create_dir_all(&mut self, path: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(path)?;
        Ok(())
    }
}
