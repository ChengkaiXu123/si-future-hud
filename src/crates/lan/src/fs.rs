//! Owner-only directories and atomic JSON writes for LAN credentials and cursors.
use super::Result;
use serde::Serialize;
use std::{fs, path::Path};

pub fn private_dir(path: &Path) -> Result<()> {
    crate::os::create_directory(path)?;
    crate::os::protect_directory(path)
}

pub fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    use std::io::Write;
    let parent = path.parent().ok_or("path")?;
    private_dir(parent)?;
    let temp = parent.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> std::io::Result<()> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        crate::os::private_file(&mut options);
        let mut file = options.open(&temp)?;
        let bytes = serde_json::to_vec(value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result.map_err(|e| e.to_string())
}
