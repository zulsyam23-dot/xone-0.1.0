//! Module: src/core/fs/file.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

impl FileEntry {
    // Constructor simpel: ambil nama file dari path, fallback ke path full kalau namanya misterius.
    pub fn new(path: PathBuf, size: u64) -> Self {
        let name = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        Self { path, name, size }
    }
}
