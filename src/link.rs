use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStrategy {
    ReflinkOnly,
    ReflinkOrHardlink,
    ReflinkOrCopy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkResult {
    Reflink,
    Hardlink,
    Copy,
}

impl LinkStrategy {
    pub fn link(&self, source: &Path, dest: &Path) -> anyhow::Result<LinkResult> {
        // Ensure parent directory exists
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Try reflink first (most efficient)
        if reflink::reflink(source, dest).is_ok() {
            return Ok(LinkResult::Reflink);
        }

        // Handle fallback based on strategy
        match self {
            Self::ReflinkOnly => {
                anyhow::bail!("Reflink failed and no fallback allowed");
            }
            Self::ReflinkOrHardlink | Self::ReflinkOrCopy => {
                // Try hardlink
                if std::fs::hard_link(source, dest).is_ok() {
                    return Ok(LinkResult::Hardlink);
                }

                // Fall back to copy if allowed
                if *self == Self::ReflinkOrCopy {
                    std::fs::copy(source, dest)?;
                    return Ok(LinkResult::Copy);
                }

                anyhow::bail!("Reflink and hardlink failed, copy not allowed");
            }
        }
    }
}
