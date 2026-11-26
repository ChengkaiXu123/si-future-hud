//! Other systems: plain files; the folder's own permissions protect the key.
use std::{fs, io::Write, path::Path};

pub fn directory(path: &Path) -> std::io::Result<()> {
    fs::create_dir_all(path)
}

pub fn write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(bytes)
}
