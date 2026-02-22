//! Module: src/core/fs/folder.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use std::path::PathBuf;

use super::FileEntry;

#[derive(Clone, Debug)]
pub struct FolderEntry {
    pub path: PathBuf,
    pub name: String,
    pub children: Vec<Node>,
}

impl FolderEntry {
    // Constructor folder: kasih nama manusiawi dari path, bukan kode rahasia.
    pub fn new(path: PathBuf, children: Vec<Node>) -> Self {
        let name = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        Self {
            path,
            name,
            children,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Node {
    File(FileEntry),
    Folder(FolderEntry),
}

impl Node {
    // Helper universal: mau File atau Folder, keluarin namanya tanpa drama.
    pub fn name(&self) -> String {
        match self {
            Node::File(file) => file.name.clone(),
            Node::Folder(folder) => folder.name.clone(),
        }
    }
}
