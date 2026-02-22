//! Module: src/app/terminal/mod.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

mod state;
mod vt100_imp;
mod widget;

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use vt100::Parser;

pub use self::widget::PseudoTerminal;

// CWD Marker constants - dipindahkan ke atas agar bisa digunakan di seluruh file
const CWD_MARKER_START: &str = "XONE_CWD:";
const CWD_MARKER_END: &str = ":XONE_CWD";
const TERMINAL_SCROLLBACK_ROWS: usize = 8000;

enum TerminalEvent {
    Output(Vec<u8>),
    Exited,
}

pub struct TerminalState {
    session: Option<TerminalSession>,
    parser: Parser,
    running: bool,
    rows: u16,
    cols: u16,
    prompt_configured: bool,
    cwd_buffer: String,
    cwd_update: Option<PathBuf>,
    scrollback: String,
    enhanced_prompt: bool,
}

struct TerminalSession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    _child: Box<dyn Child + Send>,
    rx: Receiver<TerminalEvent>,
}

impl TerminalState {
    pub fn new() -> Self {
        let rows = 24;
        let cols = 80;
        Self {
            session: None,
            parser: Parser::new(rows, cols, TERMINAL_SCROLLBACK_ROWS),
            running: false,
            rows,
            cols,
            prompt_configured: false,
            cwd_buffer: String::new(),
            cwd_update: None,
            scrollback: String::new(),
            enhanced_prompt: true,
        }
    }

    pub fn ensure_started(&mut self, cwd: &Path) -> io::Result<()> {
        if self.session.is_some() && self.running {
            return Ok(());
        }

        // Coba jalankan XOneShell dengan pseudo terminal
        let pty_system = native_pty_system();
        let pair = map_to_io(pty_system.openpty(PtySize {
            rows: self.rows,
            cols: self.cols,
            pixel_width: 0,
            pixel_height: 0,
        }))?;

        // Coba jalankan XOneShell terlebih dahulu
        let child = match spawn_xoneshell(&pair, cwd) {
            Ok(child) => child,
            Err(_) => {
                // Fallback ke PowerShell jika XOneShell gagal
                spawn_powershell(&pair, cwd)?
            }
        };

        let mut reader = map_to_io(pair.master.try_clone_reader())?;
        let writer = map_to_io(pair.master.take_writer())?;
        let master = pair.master;
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        let _ = tx.send(TerminalEvent::Exited);
                        break;
                    }
                    Ok(n) => {
                        let bytes = buffer[..n].to_vec();
                        let _ = tx.send(TerminalEvent::Output(bytes));
                    }
                    Err(_) => {
                        let _ = tx.send(TerminalEvent::Exited);
                        break;
                    }
                }
            }
        });

        let session = TerminalSession {
            master,
            writer,
            _child: child,
            rx,
        };

        self.session = Some(session);
        self.running = true;
        self.prompt_configured = false;
        self.configure_prompt()?;
        let _ = self.send("\r\n");
        Ok(())
    }

    pub fn send(&mut self, text: &str) -> io::Result<()> {
        let Some(session) = &mut self.session else {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Terminal session belum tersedia",
            ));
        };
        session.writer.write_all(text.as_bytes())?;
        session.writer.flush()
    }

    pub fn poll(&mut self) -> bool {
        let mut events = Vec::new();
        let mut has_output = false;
        let mut has_exit = false;

        if let Some(session) = &self.session {
            // Process events dengan batching yang lebih agresif
            for _ in 0..64 {
                // Tingkatkan batch size untuk throughput
                match session.rx.try_recv() {
                    Ok(event) => {
                        events.push(event);
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        has_exit = true;
                        break;
                    }
                }
            }
        }

        // Process semua event dalam satu batch
        for event in events {
            match event {
                TerminalEvent::Output(bytes) => {
                    // Pertahankan posisi scrollback user saat ada output baru masuk.
                    // Tanpa ini, beberapa stream output bisa menarik tampilan kembali ke bawah.
                    let prev_scrollback = self.parser.screen().scrollback();
                    self.parser.process(&bytes);
                    if prev_scrollback > 0 {
                        self.parser.screen_mut().set_scrollback(prev_scrollback);
                    }
                    self.capture_cwd_output(&bytes);
                    self.capture_scrollback(&bytes);
                    has_output = true;
                }
                TerminalEvent::Exited => has_exit = true,
            }
        }

        if has_exit {
            self.running = false;
        }

        // Return true jika ada output yang perlu dirender ulang
        has_output
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        if self.rows == rows && self.cols == cols {
            return;
        }
        self.rows = rows;
        self.cols = cols;
        self.parser.screen_mut().set_size(rows, cols);
        let Some(session) = &mut self.session else {
            return;
        };
        session.resize(rows, cols).ok();
    }

    pub fn take_cwd_update(&mut self) -> Option<PathBuf> {
        if let Some(path) = self.cwd_update.take() {
            return Some(path);
        }

        let mut processed_any = false;

        // Process buffer untuk mencari CWD marker
        loop {
            // Cari marker start
            let start = match self.cwd_buffer.find(CWD_MARKER_START) {
                Some(pos) => pos,
                None => break, // Tidak ada marker, keluar dari loop
            };

            // Cari marker end setelah start
            let start_pos = start + CWD_MARKER_START.len();
            if start_pos >= self.cwd_buffer.len() {
                // Marker belum lengkap, tunggu data berikutnya
                break;
            }

            let end_search = &self.cwd_buffer[start_pos..];
            let pos = match end_search.find(CWD_MARKER_END) {
                Some(pos) => pos,
                None => {
                    // Marker end belum ditemukan, tunggu data berikutnya
                    break;
                }
            };

            let path_str = sanitize_cwd_marker_text(&end_search[..pos]);

            // Validasi path string
            if path_str.is_empty() || path_str.len() < 3 {
                // Hapus marker yang invalid
                let processed_end = start + CWD_MARKER_START.len() + pos + CWD_MARKER_END.len();
                let remaining = self.cwd_buffer[processed_end..].to_string();
                self.cwd_buffer.clear();
                self.cwd_buffer.push_str(&remaining);
                processed_any = true;
                continue;
            }

            // Validasi dan canonicalize path
            match PathBuf::from(path_str.as_str()).canonicalize() {
                Ok(path) => {
                    // Pastikan path adalah direktori yang valid
                    if path.is_dir() {
                        // Hapus processed marker dari buffer
                        let processed_end =
                            start + CWD_MARKER_START.len() + pos + CWD_MARKER_END.len();
                        let remaining = self.cwd_buffer[processed_end..].to_string();
                        self.cwd_buffer.clear();
                        self.cwd_buffer.push_str(&remaining);
                        return Some(path);
                    } else {
                        // Hapus processed marker dari buffer
                        let processed_end =
                            start + CWD_MARKER_START.len() + pos + CWD_MARKER_END.len();
                        let remaining = self.cwd_buffer[processed_end..].to_string();
                        self.cwd_buffer.clear();
                        self.cwd_buffer.push_str(&remaining);
                        processed_any = true;
                    }
                }
                Err(_) => {
                    // Hapus processed marker dari buffer
                    let processed_end = start + CWD_MARKER_START.len() + pos + CWD_MARKER_END.len();
                    let remaining = self.cwd_buffer[processed_end..].to_string();
                    self.cwd_buffer.clear();
                    self.cwd_buffer.push_str(&remaining);
                    processed_any = true;
                }
            }
        }

        // Batasi ukuran buffer untuk menghindari memory bloat
        if !processed_any && self.cwd_buffer.len() > 4096 {
            self.cwd_buffer.clear();
        }

        None
    }

    pub fn search_scrollback(&self, query: &str) -> (usize, Option<String>) {
        let needle = query.trim();
        if needle.is_empty() {
            return (0, None);
        }
        let needle = needle.to_ascii_lowercase();
        let mut count = 0usize;
        let mut last_line = None;
        for line in self.scrollback.lines() {
            if line.to_ascii_lowercase().contains(&needle) {
                count += 1;
                last_line = Some(line.to_string());
            }
        }
        (count, last_line)
    }

    pub fn scrollback_up(&mut self, rows: usize) -> usize {
        if rows == 0 {
            return self.parser.screen().scrollback();
        }
        let current = self.parser.screen().scrollback();
        self.parser
            .screen_mut()
            .set_scrollback(current.saturating_add(rows));
        self.parser.screen().scrollback()
    }

    pub fn scrollback_down(&mut self, rows: usize) -> usize {
        if rows == 0 {
            return self.parser.screen().scrollback();
        }
        let current = self.parser.screen().scrollback();
        self.parser
            .screen_mut()
            .set_scrollback(current.saturating_sub(rows));
        self.parser.screen().scrollback()
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    pub fn set_enhanced_prompt(&mut self, enabled: bool) -> io::Result<()> {
        self.enhanced_prompt = enabled;
        if self.session.is_some() && self.running {
            self.prompt_configured = false;
            self.configure_prompt()?;
        }
        Ok(())
    }

    fn configure_prompt(&mut self) -> io::Result<()> {
        if self.prompt_configured {
            return Ok(());
        }

        // Strategy: Override semua command yang bisa mengubah direktori
        let cd_override = format!(
            "function cd {{ param($path); Set-Location $path; Write-Output \"{0}$($PWD.Path){1}\" }}\r\n",
            CWD_MARKER_START, CWD_MARKER_END
        );
        self.send(&cd_override)?;

        // Override juga pushd dan popd
        let pushd_override = format!(
            "function pushd {{ param($path); Push-Location $path; Write-Output \"{0}$($PWD.Path){1}\" }}\r\n",
            CWD_MARKER_START, CWD_MARKER_END
        );
        self.send(&pushd_override)?;

        let popd_override = format!(
            "function popd {{ $newPath = (Get-Location).Path; Pop-Location; Write-Output \"{0}$newPath{1}\" }}\r\n",
            CWD_MARKER_START, CWD_MARKER_END
        );
        self.send(&popd_override)?;

        // Override Set-Location juga
        let set_location_override = format!(
            "function Set-Location {{ param($path); Microsoft.PowerShell.Management\\Set-Location $path; Write-Output \"{0}$($PWD.Path){1}\" }}\r\n",
            CWD_MARKER_START, CWD_MARKER_END
        );
        self.send(&set_location_override)?;

        // Untuk inisialisasi awal, kirim current directory
        let init_cwd = format!(
            "Write-Output \"{0}$($PWD.Path){1}\"\r\n",
            CWD_MARKER_START, CWD_MARKER_END
        );
        self.send(&init_cwd)?;

        let prompt_fn = if self.enhanced_prompt {
            "function prompt {\r\n\
$p = (Get-Location).Path\r\n\
$leaf = Split-Path -Leaf $p\r\n\
\"`e[38;2;122;202;255m$leaf`e[0m `e[38;2;124;224;184m>`e[0m \"\r\n\
}\r\n\
Set-PSReadLineOption -Colors @{ Command = \"#7ACAFF\"; String = \"#EFCF88\"; Number = \"#88C3FF\"; Operator = \"#B6A3FF\"; Variable = \"#7DE8BA\"; Parameter = \"#FFB77A\" }\r\n"
        } else {
            "function prompt { \"PS \" + $(Get-Location) + \"> \" }\r\n"
        };
        self.send(prompt_fn)?;

        self.prompt_configured = true;

        Ok(())
    }

    fn capture_cwd_output(&mut self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        self.cwd_buffer.push_str(&text);

        // Proses buffer untuk menemukan semua marker CWD
        let mut processed_any = false;
        while let Some(start) = self.cwd_buffer.find(CWD_MARKER_START) {
            let end_search = &self.cwd_buffer[start + CWD_MARKER_START.len()..];
            let Some(pos) = end_search.find(CWD_MARKER_END) else {
                // Jika tidak menemukan end marker, simpan buffer untuk next iteration
                break;
            };

            let path_str = sanitize_cwd_marker_text(&end_search[..pos]);

            // Validasi path string
            if path_str.is_empty() || path_str.len() < 3 {
                // Hapus marker yang invalid
                let processed_end = start + CWD_MARKER_START.len() + pos + CWD_MARKER_END.len();
                let remaining = self.cwd_buffer[processed_end..].to_string();
                self.cwd_buffer.clear();
                self.cwd_buffer.push_str(&remaining);
                processed_any = true;
                continue;
            }

            // Validasi dan canonicalize path
            match PathBuf::from(path_str.as_str()).canonicalize() {
                Ok(path) => {
                    // Pastikan path adalah direktori yang valid
                    if path.is_dir() {
                        self.cwd_update = Some(path);
                    }
                }
                Err(_) => {}
            }

            // Hapus processed marker dari buffer
            let processed_end = start + CWD_MARKER_START.len() + pos + CWD_MARKER_END.len();
            let remaining = self.cwd_buffer[processed_end..].to_string();
            self.cwd_buffer.clear();
            self.cwd_buffer.push_str(&remaining);
            processed_any = true;
        }

        // Batasi ukuran buffer untuk menghindari memory bloat
        if !processed_any && self.cwd_buffer.len() > 4096 {
            self.cwd_buffer.clear();
        }
    }

    fn capture_scrollback(&mut self, bytes: &[u8]) {
        const MAX_SCROLLBACK_CHARS: usize = 220_000;
        let text = String::from_utf8_lossy(bytes);
        self.scrollback.push_str(&text);
        if self.scrollback.len() > MAX_SCROLLBACK_CHARS {
            let overflow = self.scrollback.len() - MAX_SCROLLBACK_CHARS;
            let mut cut = overflow;
            while !self.scrollback.is_char_boundary(cut) && cut < self.scrollback.len() {
                cut += 1;
            }
            self.scrollback.drain(..cut.min(self.scrollback.len()));
        }
    }
}

impl TerminalSession {
    fn resize(&mut self, rows: u16, cols: u16) -> io::Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}

fn spawn_xoneshell(pair: &portable_pty::PtyPair, cwd: &Path) -> io::Result<Box<dyn Child + Send>> {
    let xoneshell_path = find_xoneshell_binary();

    if let Some(path) = xoneshell_path {
        let mut cmd = CommandBuilder::new(path);
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if let Ok(child) = map_to_io(pair.slave.spawn_command(cmd)) {
            return Ok(child);
        }
    }

    // Fallback ke PowerShell jika XOneShell tidak ditemukan
    spawn_powershell(pair, cwd)
}

fn find_xoneshell_binary() -> Option<String> {
    // Cek di target/debug terlebih dahulu (dari current working directory)
    let debug_path = std::env::current_dir()
        .ok()?
        .join("target")
        .join("debug")
        .join("xoneshell.exe");

    if debug_path.exists() {
        return Some(debug_path.to_string_lossy().to_string());
    }

    // Cek di target/release
    let release_path = std::env::current_dir()
        .ok()?
        .join("target")
        .join("release")
        .join("xoneshell.exe");

    if release_path.exists() {
        return Some(release_path.to_string_lossy().to_string());
    }

    // Cek di PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for path_dir in path_var.split(';') {
            let xoneshell_path = std::path::Path::new(path_dir).join("xoneshell.exe");
            if xoneshell_path.exists() {
                return Some(xoneshell_path.to_string_lossy().to_string());
            }
        }
    }

    None
}

fn spawn_powershell(pair: &portable_pty::PtyPair, cwd: &Path) -> io::Result<Box<dyn Child + Send>> {
    let candidates = ["pwsh.exe", "pwsh", "powershell.exe", "powershell"];
    for candidate in candidates {
        let mut cmd = CommandBuilder::new(candidate);
        cmd.arg("-NoLogo");
        cmd.arg("-NoProfile");
        cmd.arg("-NoExit");
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if let Ok(child) = map_to_io(pair.slave.spawn_command(cmd)) {
            return Ok(child);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "PowerShell tidak ditemukan",
    ))
}

fn sanitize_cwd_marker_text(input: &str) -> String {
    // Keep visible non-control characters only, then trim whitespace.
    input
        .chars()
        .filter(|ch| !ch.is_control())
        .collect::<String>()
        .trim()
        .to_string()
}

#[derive(Debug)]
struct TerminalIoError(String);

impl std::fmt::Display for TerminalIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TerminalIoError {}

fn map_to_io<T, E>(result: Result<T, E>) -> io::Result<T>
where
    E: std::fmt::Display,
{
    result.map_err(|error| io::Error::other(TerminalIoError(error.to_string())))
}
