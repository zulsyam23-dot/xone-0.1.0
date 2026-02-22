//! Module: src/core/fs/tree.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::CoreError;

use super::{FileEntry, FolderEntry, Node};

pub fn build_tree(root: &Path, depth: usize) -> Result<FolderEntry, CoreError> {
    let children = if depth == 0 {
        Vec::new()
    } else {
        list_children(root, depth)?
    };
    Ok(FolderEntry::new(root.to_path_buf(), children))
}

fn list_children(path: &Path, depth: usize) -> Result<Vec<Node>, CoreError> {
    let mut entries: Vec<(PathBuf, bool, String)> = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if should_skip(&entry_path) {
            continue;
        }
        let is_dir = entry.file_type()?.is_dir();
        let sort_name = entry.file_name().to_string_lossy().to_lowercase();
        entries.push((entry_path, is_dir, sort_name));
    }
    entries.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.2.cmp(&right.2)));
    let mut nodes = Vec::new();
    for (entry, is_dir, _) in entries {
        let metadata = fs::metadata(&entry)?;
        if is_dir {
            let children = if depth > 1 {
                list_children(&entry, depth - 1)?
            } else {
                Vec::new()
            };
            nodes.push(Node::Folder(FolderEntry::new(entry, children)));
        } else {
            nodes.push(Node::File(FileEntry::new(entry, metadata.len())));
        }
    }
    Ok(nodes)
}

fn should_skip(path: &Path) -> bool {
    let name = path.file_name().and_then(|value| value.to_str());
    matches!(name, Some("target") | Some(".git"))
}
