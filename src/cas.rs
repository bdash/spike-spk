use std::path::PathBuf;

const CAS_VERSION: &str = "1";

pub struct ContentAddressableStore {
    root: PathBuf,
}

impl ContentAddressableStore {
    pub fn new(root: PathBuf) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&root)?;

        // Write version marker if it doesn't exist
        let version_file = root.join(".cas-version");
        if !version_file.exists() {
            std::fs::write(&version_file, CAS_VERSION)?;
        }

        let objects_dir = root.join("objects");
        std::fs::create_dir_all(&objects_dir)?;

        Ok(Self { root })
    }

    pub fn ensure_stored(&self, hash: &str, data: &[u8]) -> anyhow::Result<PathBuf> {
        let path = self.get_path(hash);

        // If file already exists, we're done
        if path.exists() {
            return Ok(path);
        }

        // Create parent directory
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write to temporary file first
        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, data)?;

        // Atomic rename
        std::fs::rename(&temp_path, &path)?;

        Ok(path)
    }

    #[must_use]
    pub fn get_path(&self, hash: &str) -> PathBuf {
        // Use first 2 characters for sharding
        let shard1 = &hash[0..2];
        let shard2 = &hash[2..4];

        self.root
            .join("objects")
            .join(shard1)
            .join(shard2)
            .join(hash)
    }
}
