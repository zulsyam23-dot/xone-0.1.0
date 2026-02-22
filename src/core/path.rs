//! Module: src/core/path.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use std::path::{Path, PathBuf};

use super::CoreError;

pub fn ensure_within_root(root: &Path, target: &Path) -> Result<PathBuf, CoreError> {
    let root = root.canonicalize()?;
    let absolute = if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    };
    let mut current = absolute.clone();
    while !current.exists() {
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }
    let base = current.canonicalize()?;
    if base.starts_with(&root) {
        Ok(absolute)
    } else {
        Err(CoreError::OutsideRoot(absolute))
    }
}

pub fn to_relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
