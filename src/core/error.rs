//! Module: src/core/error.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use std::path::PathBuf;

#[derive(Debug)]
pub enum CoreError {
    Io(std::io::Error),
    OutsideRoot(PathBuf),
    NonUtf8(PathBuf),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::Io(error) => write!(f, "{}", error),
            CoreError::OutsideRoot(path) => {
                write!(f, "Path di luar workspace: {}", path.display())
            }
            CoreError::NonUtf8(path) => {
                write!(
                    f,
                    "File bukan UTF-8 dan tidak bisa dibuka sebagai teks: {}",
                    path.display()
                )
            }
        }
    }
}

impl From<std::io::Error> for CoreError {
    fn from(value: std::io::Error) -> Self {
        CoreError::Io(value)
    }
}
