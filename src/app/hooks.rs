//! Module: src/app/hooks.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use std::fs;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Debug)]
pub struct CommandHook {
    pub binding: HookBinding,
    pub action: HookAction,
}

#[derive(Clone, Debug)]
pub enum HookAction {
    Terminal(String),
    Message(String),
    Open(PathBuf),
}

#[derive(Clone, Debug)]
pub struct HookBinding {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: HookKey,
}

#[derive(Clone, Debug)]
pub enum HookKey {
    Char(char),
    Enter,
    Tab,
    F(u8),
}

impl HookBinding {
    pub fn matches(&self, event: &KeyEvent) -> bool {
        if self.ctrl != event.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }
        if self.alt != event.modifiers.contains(KeyModifiers::ALT) {
            return false;
        }
        if self.shift != event.modifiers.contains(KeyModifiers::SHIFT) {
            return false;
        }
        match (&self.key, event.code) {
            (HookKey::Char(a), KeyCode::Char(b)) => a.eq_ignore_ascii_case(&b),
            (HookKey::Enter, KeyCode::Enter) => true,
            (HookKey::Tab, KeyCode::Tab) => true,
            (HookKey::F(a), KeyCode::F(b)) => *a == b,
            _ => false,
        }
    }
}

pub fn load_hooks(root: &Path) -> Vec<CommandHook> {
    let path = root.join(".xone").join("hooks.conf");
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut hooks = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(hook) = parse_line(line) {
            hooks.push(hook);
        }
    }
    hooks
}

fn parse_line(line: &str) -> Option<CommandHook> {
    let mut parts = line.splitn(3, '|');
    let bind_raw = parts.next()?.trim();
    let kind = parts.next()?.trim().to_ascii_lowercase();
    let payload = parts.next()?.trim();
    let binding = parse_binding(bind_raw)?;
    let action = match kind.as_str() {
        "terminal" => HookAction::Terminal(payload.to_string()),
        "message" => HookAction::Message(payload.to_string()),
        "open" => HookAction::Open(PathBuf::from(payload)),
        _ => return None,
    };
    Some(CommandHook { binding, action })
}

fn parse_binding(input: &str) -> Option<HookBinding> {
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut key = None;
    for part in input.split('+').map(|v| v.trim().to_ascii_lowercase()) {
        match part.as_str() {
            "ctrl" | "control" => ctrl = true,
            "alt" => alt = true,
            "shift" => shift = true,
            "enter" => key = Some(HookKey::Enter),
            "tab" => key = Some(HookKey::Tab),
            _ if part.starts_with('f') && part.len() <= 3 => {
                let value = part[1..].parse::<u8>().ok()?;
                key = Some(HookKey::F(value));
            }
            _ if part.len() == 1 => {
                key = Some(HookKey::Char(part.chars().next()?));
            }
            _ => return None,
        }
    }
    Some(HookBinding {
        ctrl,
        alt,
        shift,
        key: key?,
    })
}
