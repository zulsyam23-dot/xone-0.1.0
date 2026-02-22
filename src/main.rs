//! Module: src/main.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use std::env;
use std::path::PathBuf;

mod app;
mod core;

fn main() {
    let mut args = env::args().skip(1);
    let first = args.next();
    let root = if matches!(first.as_deref(), Some("run")) {
        args.next().map(PathBuf::from)
    } else {
        first.map(PathBuf::from)
    }
    .unwrap_or_else(resolve_start_dir);
    if let Err(error) = crate::app::run(root) {
        eprintln!("{}", error);
        std::process::exit(1);
    }
}

fn resolve_start_dir() -> PathBuf {
    if let Ok(cur) = env::current_dir() {
        let candidates = [
            cur.join("src").join("xone"),
            cur.join("src").join("xone.02"),
            cur.join("xone"),
            cur.join("xone.02"),
        ];
        for candidate in candidates {
            if candidate.is_dir() {
                return candidate;
            }
        }
    }
    if let Ok(v) = env::var("KARNELTERM_HOME") {
        let p = PathBuf::from(v);
        if p.is_dir() {
            return p;
        }
    }
    if let Ok(v) = env::var("XONE_HOME") {
        let p = PathBuf::from(v);
        if p.is_dir() {
            return p;
        }
    }
    if let Ok(home) = env::var("USERPROFILE") {
        let docs = PathBuf::from(home).join("Documents");
        let candidates = [docs.join("xone"), docs.join("xone.02")];
        for candidate in candidates {
            if candidate.is_dir() {
                return candidate;
            }
        }
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
