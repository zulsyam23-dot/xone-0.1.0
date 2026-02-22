//! Module: src/core/mod.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

pub use error::CoreError;
pub use fs::{FolderEntry, Node};
pub use workspace::Workspace;

mod error;
mod fs;
mod path;
mod workspace;
