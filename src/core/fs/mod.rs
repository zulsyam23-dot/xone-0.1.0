//! Module: src/core/fs/mod.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

pub use file::FileEntry;
pub use folder::{FolderEntry, Node};

mod file;
mod folder;
mod tree;

use std::path::Path;

use crate::core::CoreError;

pub fn build_tree(root: &Path, depth: usize) -> Result<FolderEntry, CoreError> {
    tree::build_tree(root, depth)
}
