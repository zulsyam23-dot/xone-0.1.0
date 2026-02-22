//! Module: src/app/mod.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use std::collections::{hash_map::DefaultHasher, BTreeSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::io::IsTerminal;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use crossterm::cursor::Show;
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use crate::core::{CoreError, Workspace};

mod control;
mod editor_clipboard;
mod explorer_controller;
mod helpers;
mod hooks;
mod input_router;
mod language;
mod mouse_router;
mod settings_panel;
mod shortcuts_panel;
mod style;
mod terminal;
mod terminal_controller;
#[cfg(test)]
mod tests;
mod ui;

use self::helpers::*;
use self::hooks::{load_hooks, CommandHook, HookAction};
use self::terminal::TerminalState;

#[cfg(windows)]
type ConsoleHandle = windows_sys::Win32::Foundation::HANDLE;
#[cfg(not(windows))]
type ConsoleHandle = usize;

#[derive(Debug)]
pub enum AppError {
    Core(CoreError),
    Io(io::Error),
}

#[derive(Default)]
struct DirtyFlags {
    ui: bool,
    explorer: bool,
    terminal: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct VisualColCacheKey {
    buffer_index: usize,
    row: usize,
    col: usize,
    line_hash: u64,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Core(error) => write!(f, "{}", error),
            AppError::Io(error) => write!(f, "{}", error),
        }
    }
}

impl From<CoreError> for AppError {
    fn from(value: CoreError) -> Self {
        AppError::Core(value)
    }
}

impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        AppError::Io(value)
    }
}

pub fn run(root: PathBuf) -> Result<(), AppError> {
    let workspace = Workspace::new(root)?;
    let mut app = App::new(workspace)?;
    app.run()
}

struct App {
    workspace: Workspace,
    explorer_root: PathBuf,
    tree_depth: usize,
    explorer: ExplorerState,
    ai_chat: AiChatState,
    editor: EditorState,
    focus: Focus,
    explorer_visible: bool,
    left_panel: LeftPanel,
    message: Option<String>,
    quit: bool,
    editor_view_height: usize,
    editor_view_width: usize,
    selection: Option<PanelSelection>,
    mouse_dragging: bool,
    clipboard: String,
    explorer_prompt: Option<ExplorerPromptState>,
    ui_regions: UiRegions,
    theme: style::Theme,
    settings: SettingsState,
    focus_before_settings: Focus,
    focus_before_about: Focus,
    focus_before_shortcuts: Focus,
    suggestion_index: usize,
    last_control_action: Option<(control::ControlAction, Instant)>,
    last_paste_fingerprint: Option<(u64, usize, Instant)>,
    last_editor_insert_key_at: Option<Instant>,
    last_explorer_refresh: Option<Instant>,
    dirty: DirtyFlags,
    editor_insert_key_streak: usize,
    editor_plain_paste_mode_until: Option<Instant>,
    pending_editor_paste: Option<PendingEditorPaste>,
    terminal: TerminalTabs,
    terminal_visible: bool,
    pending_terminal_cwd: Option<PathBuf>,
    terminal_command_buffer: String,
    terminal_command_tracking_valid: bool,
    terminal_search_mode: bool,
    terminal_search_query: String,
    workspace_search_mode: bool,
    workspace_search_query: String,
    workspace_search_results: Vec<WorkspaceSearchHit>,
    workspace_search_selected: usize,
    ai_rx: Option<Receiver<AiWorkerEvent>>,
    about_lines: Vec<String>,
    about_scroll: usize,
    command_hooks: Vec<CommandHook>,
    last_terminal_resize_redraw: Option<Instant>,
    visual_col_cache: std::collections::HashMap<VisualColCacheKey, usize>,
    shortcut_bindings: Vec<(control::ControlAction, control::ShortcutBinding)>,
    shortcuts_selected: usize,
    shortcuts_capture_mode: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Explorer,
    AiChat,
    Editor,
    Settings,
    Shortcuts,
    Terminal,
    About,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LeftPanel {
    Explorer,
    AiChat,
}

const EDITOR_PASTE_BURST_GAP: Duration = Duration::from_millis(12);
const EDITOR_PASTE_BURST_STREAK: usize = 6;
const EDITOR_PLAIN_MODE_TTL: Duration = Duration::from_millis(260);
const EDITOR_PENDING_PASTE_FLUSH_DELAY: Duration = Duration::from_millis(28);
const AUTO_SAVE_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone, Copy)]
struct PanelSelection {
    panel: Focus,
    anchor_row: usize,
    head_row: usize,
}

#[derive(Clone, Copy, Default)]
struct UiRegions {
    explorer: Option<Rect>,
    editor: Option<Rect>,
    terminal: Option<Rect>,
}

struct ExplorerState {
    items: Vec<ExplorerItem>,
    selected: usize,
}

struct AiChatState {
    messages: Vec<AiChatMessage>,
    input: String,
    cursor_col: usize,
    scroll: usize,
    config: AiConfig,
    api_key_present: bool,
    inflight: bool,
    read_only_mode: bool,
    pending_apply: Option<AiApplyPreview>,
}

#[derive(Clone)]
struct AiApplyPreview {
    content: String,
    replace_selection: bool,
    diff_lines: Vec<String>,
}

#[derive(Clone)]
struct WorkspaceSearchHit {
    path: PathBuf,
    line_no: usize,
    line: String,
}

struct AiChatMessage {
    role: AiRole,
    content: String,
}

#[derive(Clone, Copy)]
enum AiRole {
    User,
    Assistant,
}

#[derive(Clone)]
struct AiConfig {
    provider: String,
    base_url: String,
    model: String,
    api_key: String,
}

enum AiWorkerEvent {
    Chunk(String),
    Done,
    Error(String),
}

struct ExplorerItem {
    path: PathBuf,
    name: String,
    depth: usize,
    is_dir: bool,
}

struct EditorState {
    buffers: Vec<EditorBuffer>,
    active: usize,
}

struct PendingEditorPaste {
    content: String,
    last_update: Instant,
}

struct EditorBuffer {
    path: PathBuf,
    language: language::Language,
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    scroll: usize,
    hscroll: usize,
    dirty: bool,
    bookmarks: BTreeSet<usize>,
    line_ending: LineEnding,
    undo_stack: Vec<EditorSnapshot>,
    redo_stack: Vec<EditorSnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExplorerPromptKind {
    File,
    Folder,
}

struct ExplorerPromptState {
    kind: ExplorerPromptKind,
    value: String,
    cursor_col: usize,
}

struct SettingsState {
    selected: usize,
    syntax_highlight: bool,
    code_suggestions: bool,
    shortcut_page: usize,
    appearance: style::AppearancePreset,
    accent: style::AccentPreset,
    tab_theme: style::TabThemePreset,
    syntax_theme: style::SyntaxThemePreset,
    density: style::UiDensity,
    terminal_fx: bool,
    terminal_command_sync: bool,
    hard_mode: bool,
    adaptive_terminal_height: bool,
    ai_profile: AiProfile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AiProfile {
    Groq,
    Together,
    OpenRouter,
}

impl AiProfile {
    fn label(self) -> &'static str {
        match self {
            AiProfile::Groq => "groq",
            AiProfile::Together => "together",
            AiProfile::OpenRouter => "openrouter",
        }
    }

    fn next(self) -> Self {
        match self {
            AiProfile::Groq => AiProfile::Together,
            AiProfile::Together => AiProfile::OpenRouter,
            AiProfile::OpenRouter => AiProfile::Groq,
        }
    }
}

struct TerminalTabs {
    tabs: Vec<TerminalState>,
    active: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
        }
    }
}

#[derive(Clone, Debug)]
struct EditorSnapshot {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    scroll: usize,
    hscroll: usize,
    dirty: bool,
    bookmarks: BTreeSet<usize>,
}

impl EditorBuffer {
    fn snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            lines: self.lines.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
            scroll: self.scroll,
            hscroll: self.hscroll,
            dirty: self.dirty,
            bookmarks: self.bookmarks.clone(),
        }
    }

    fn apply_snapshot(&mut self, snapshot: EditorSnapshot) {
        self.lines = snapshot.lines;
        self.cursor_row = snapshot.cursor_row;
        self.cursor_col = snapshot.cursor_col;
        self.scroll = snapshot.scroll;
        self.hscroll = snapshot.hscroll;
        self.dirty = snapshot.dirty;
        self.bookmarks = snapshot.bookmarks;
    }

    fn push_undo(&mut self) {
        const UNDO_LIMIT: usize = 256;
        self.undo_stack.push(self.snapshot());
        if self.undo_stack.len() > UNDO_LIMIT {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo_stack.pop() {
            self.redo_stack.push(self.snapshot());
            self.apply_snapshot(previous);
            return true;
        }
        false
    }

    fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.snapshot());
            self.apply_snapshot(next);
            return true;
        }
        false
    }

    fn toggle_bookmark(&mut self) -> bool {
        if self.bookmarks.remove(&self.cursor_row) {
            false
        } else {
            self.bookmarks.insert(self.cursor_row);
            true
        }
    }

    fn next_bookmark_row(&self) -> Option<usize> {
        if self.bookmarks.is_empty() {
            return None;
        }
        self.bookmarks
            .range((self.cursor_row + 1)..)
            .next()
            .copied()
            .or_else(|| self.bookmarks.first().copied())
    }

    fn prev_bookmark_row(&self) -> Option<usize> {
        if self.bookmarks.is_empty() {
            return None;
        }
        self.bookmarks
            .range(..self.cursor_row)
            .next_back()
            .copied()
            .or_else(|| self.bookmarks.last().copied())
    }
}

impl EditorState {
    fn current(&self) -> Option<&EditorBuffer> {
        self.buffers.get(self.active)
    }

    fn current_mut(&mut self) -> Option<&mut EditorBuffer> {
        self.buffers.get_mut(self.active)
    }

    fn has_open_buffer(&self) -> bool {
        !self.buffers.is_empty()
    }

    fn has_dirty_buffer(&self) -> bool {
        self.buffers.iter().any(|buffer| buffer.dirty)
    }

    fn activate(&mut self, index: usize) {
        if index < self.buffers.len() {
            self.active = index;
        }
    }

    fn next(&mut self) {
        if self.buffers.is_empty() {
            return;
        }
        self.active = (self.active + 1) % self.buffers.len();
    }

    fn prev(&mut self) {
        if self.buffers.is_empty() {
            return;
        }
        self.active = if self.active == 0 {
            self.buffers.len() - 1
        } else {
            self.active - 1
        };
    }
}

impl TerminalTabs {
    fn new() -> Self {
        Self {
            tabs: vec![TerminalState::new()],
            active: 0,
        }
    }

    fn active_index(&self) -> usize {
        self.active
    }

    fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    fn current(&self) -> &TerminalState {
        &self.tabs[self.active]
    }

    fn current_mut(&mut self) -> &mut TerminalState {
        &mut self.tabs[self.active]
    }

    fn ensure_started(&mut self, cwd: &Path) -> io::Result<()> {
        self.current_mut().ensure_started(cwd)
    }

    fn is_running(&self) -> bool {
        self.current().is_running()
    }

    fn send(&mut self, text: &str) -> io::Result<()> {
        self.current_mut().send(text)
    }

    fn resize(&mut self, rows: u16, cols: u16) {
        self.current_mut().resize(rows, cols);
    }

    fn screen(&self) -> &vt100::Screen {
        self.current().screen()
    }

    fn poll(&mut self) -> bool {
        let mut active_output = false;
        for (index, tab) in self.tabs.iter_mut().enumerate() {
            let has_output = tab.poll();
            if index == self.active && has_output {
                active_output = true;
            }
        }
        active_output
    }

    fn take_cwd_update(&mut self) -> Option<(usize, PathBuf)> {
        if let Some(path) = self.tabs[self.active].take_cwd_update() {
            return Some((self.active, path));
        }
        for (index, tab) in self.tabs.iter_mut().enumerate() {
            if index == self.active {
                continue;
            }
            if let Some(path) = tab.take_cwd_update() {
                return Some((index, path));
            }
        }
        None
    }

    fn new_tab(&mut self, cwd: &Path) -> io::Result<usize> {
        let mut tab = TerminalState::new();
        tab.ensure_started(cwd)?;
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        Ok(self.active)
    }

    fn next_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.active = (self.active + 1) % self.tabs.len();
    }

    fn prev_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.active = if self.active == 0 {
            self.tabs.len() - 1
        } else {
            self.active - 1
        };
    }

    fn search_scrollback(&self, query: &str) -> (usize, Option<String>) {
        self.current().search_scrollback(query)
    }

    fn scrollback_up(&mut self, rows: usize) -> usize {
        self.current_mut().scrollback_up(rows)
    }

    fn scrollback_down(&mut self, rows: usize) -> usize {
        self.current_mut().scrollback_down(rows)
    }

    fn set_enhanced_prompt(&mut self, enabled: bool) {
        for tab in &mut self.tabs {
            let _ = tab.set_enhanced_prompt(enabled);
        }
    }
}

impl App {
    fn new(workspace: Workspace) -> Result<Self, AppError> {
        let explorer_root = workspace.root().to_path_buf();
        let (
            appearance,
            accent,
            tab_theme,
            syntax_theme,
            density,
            terminal_fx,
            terminal_command_sync,
            hard_mode,
            adaptive_terminal_height,
        ) =
            load_ui_preferences(workspace.root());
        let ai_config = load_ai_config(workspace.root());
        let ai_profile = infer_ai_profile(&ai_config);
        let about_lines = load_about_lines(workspace.root());
        let shortcut_bindings = load_shortcut_bindings(workspace.root());
        let api_key_present = if ai_config.provider.eq_ignore_ascii_case("ollama") {
            true
        } else {
            !ai_config.api_key.trim().is_empty()
        };
        let mut app = Self {
            workspace,
            explorer_root,
            tree_depth: 6,
            explorer: ExplorerState {
                items: Vec::new(),
                selected: 0,
            },
            ai_chat: AiChatState {
                messages: vec![AiChatMessage {
                    role: AiRole::Assistant,
                    content:
                        "x1.AI siap. Tekan Enter untuk kirim. Tekan F6 untuk fokus panel ini."
                            .to_string(),
                }],
                input: String::new(),
                cursor_col: 0,
                scroll: 0,
                config: ai_config,
                api_key_present,
                inflight: false,
                read_only_mode: true,
                pending_apply: None,
            },
            editor: EditorState {
                buffers: Vec::new(),
                active: 0,
            },
            focus: Focus::Explorer,
            explorer_visible: true,
            left_panel: LeftPanel::Explorer,
            message: Some(
                "Xone siap. Ctrl+E Explorer, Ctrl+I Editor, Ctrl+O/F2 Settings, Ctrl+T Terminal"
                    .to_string(),
            ),
            quit: false,
            editor_view_height: 1,
            editor_view_width: 1,
            selection: None,
            mouse_dragging: false,
            clipboard: String::new(),
            explorer_prompt: None,
            ui_regions: UiRegions::default(),
            theme: style::Theme::with_options(appearance, accent, tab_theme, syntax_theme),
            settings: SettingsState {
                selected: 0,
                syntax_highlight: true,
                code_suggestions: true,
                shortcut_page: 0,
                appearance,
                accent,
                tab_theme,
                syntax_theme,
                density,
                terminal_fx,
                terminal_command_sync,
                hard_mode,
                adaptive_terminal_height,
                ai_profile,
            },
            focus_before_settings: Focus::Explorer,
            focus_before_about: Focus::Explorer,
            focus_before_shortcuts: Focus::Explorer,
            suggestion_index: 0,
            last_control_action: None,
            last_paste_fingerprint: None,
            last_editor_insert_key_at: None,
            last_explorer_refresh: None,
            editor_insert_key_streak: 0,
            editor_plain_paste_mode_until: None,
            pending_editor_paste: None,
            terminal: TerminalTabs::new(),
            terminal_visible: false,
            pending_terminal_cwd: None,
            terminal_command_buffer: String::new(),
            terminal_command_tracking_valid: terminal_command_sync,
            terminal_search_mode: false,
            terminal_search_query: String::new(),
            workspace_search_mode: false,
            workspace_search_query: String::new(),
            workspace_search_results: Vec::new(),
            workspace_search_selected: 0,
            ai_rx: None,
            about_lines,
            about_scroll: 0,
            command_hooks: Vec::new(),
            last_terminal_resize_redraw: None,
            visual_col_cache: std::collections::HashMap::new(),
            shortcut_bindings,
            shortcuts_selected: 0,
            shortcuts_capture_mode: false,
            dirty: DirtyFlags::default(),
        };
        app.terminal.set_enhanced_prompt(terminal_fx);
        app.command_hooks = load_hooks(app.workspace.root());
        app.refresh_explorer()?;

        // Otomatis buka terminal XOneShell saat startup
        if let Err(error) = app.terminal.ensure_started(&app.explorer_root) {
            app.message = Some(format!("Gagal membuka XOneShell: {}", error));
        } else {
            app.terminal_visible = true;
            app.set_focus(Focus::Terminal);
            app.message = Some("XOneShell siap. Gunakan Ctrl+T untuk toggle terminal.".to_string());
        }

        Ok(app)
    }

    fn run(&mut self) -> Result<(), AppError> {
        if !is_interactive_terminal() {
            println!(
                "xone: mode non-interaktif terdeteksi; preview TUI dilewati. \
jalankan di PowerShell/CMD interaktif untuk UI penuh."
            );
            return Ok(());
        }

        enable_raw_mode()?;
        let windows_mode = match configure_windows_input_mode() {
            Ok(mode) => mode,
            Err(error) => {
                disable_raw_mode().ok();
                return Err(error.into());
            }
        };
        let mut entered_alt_screen = false;
        let mut mouse_capture_enabled = false;
        let mut bracketed_paste_enabled = false;

        let result = (|| -> Result<(), AppError> {
            io::stdout().execute(EnterAlternateScreen)?;
            entered_alt_screen = true;

            let backend = CrosstermBackend::new(io::stdout());
            let mut terminal = Terminal::new(backend)?;
            terminal.backend_mut().execute(EnableMouseCapture)?;
            mouse_capture_enabled = true;
            terminal.backend_mut().execute(EnableBracketedPaste)?;
            bracketed_paste_enabled = true;
            self.run_loop(&mut terminal)
        })();

        if bracketed_paste_enabled {
            io::stdout().execute(DisableBracketedPaste).ok();
        }
        if mouse_capture_enabled {
            io::stdout().execute(DisableMouseCapture).ok();
        }
        if entered_alt_screen {
            io::stdout().execute(LeaveAlternateScreen).ok();
        }
        io::stdout().execute(Show).ok();
        disable_raw_mode().ok();
        restore_windows_input_mode(windows_mode);
        result
    }

    fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), AppError> {
        let mut needs_render = true;
        let mut last_terminal_poll = Instant::now();
        let mut last_cwd_sync = Instant::now();
        let mut last_auto_save = Instant::now();
        let mut pending_resize_redraw = false;
        let mut last_resize_event = Instant::now();

        const TERMINAL_POLL_INTERVAL: Duration = Duration::from_millis(4); // 250 Hz
        const CWD_SYNC_INTERVAL: Duration = Duration::from_millis(10); // 100 Hz - dipercepat untuk response lebih cepat
        const RESIZE_REDRAW_DEBOUNCE: Duration = Duration::from_millis(40);
        const MAX_EVENT_WAIT: Duration = Duration::from_millis(8);

        while !self.quit {
            self.flush_pending_editor_paste_if_ready(false);
            if self.poll_ai_worker_events() {
                self.dirty.ui = true;
                needs_render = true;
            }

            // Poll terminal dengan frekuensi tinggi tapi terpisah
            if last_terminal_poll.elapsed() >= TERMINAL_POLL_INTERVAL {
                let had_output = self.terminal.poll();
                if had_output {
                    self.dirty.ui = true; // Mark UI dirty jika ada output terminal
                    needs_render = true;
                }
                last_terminal_poll = Instant::now();
            }

            // Sync CWD dengan interval yang lebih jarang
            if last_cwd_sync.elapsed() >= CWD_SYNC_INTERVAL {
                let had_cwd_change = self.sync_terminal_cwd();
                if had_cwd_change {
                    self.dirty.explorer = true; // Mark explorer dirty jika ada perubahan CWD
                    needs_render = true;
                }
                last_cwd_sync = Instant::now();
            }

            if last_auto_save.elapsed() >= AUTO_SAVE_INTERVAL {
                let saved = self.autosave_dirty_buffers()?;
                if saved > 0 {
                    self.message = Some(format!("Auto-save: {} file", saved));
                    self.dirty.ui = true;
                    self.dirty.explorer = true;
                    needs_render = true;
                }
                last_auto_save = Instant::now();
            }

            // Resize ditangani lewat debounce agar tidak bikin terminal redraw liar saat drag-resize.
            if pending_resize_redraw && last_resize_event.elapsed() >= RESIZE_REDRAW_DEBOUNCE {
                self.redraw_terminal_after_resize();
                pending_resize_redraw = false;
                needs_render = true;
            }

            // Render event-driven: tetap render saat resize burst agar area terminal
            // tidak meninggalkan artefak frame lama ketika window diperkecil.
            let has_dirty = self.dirty.ui || self.dirty.explorer || self.dirty.terminal;
            if needs_render || has_dirty {
                terminal.draw(|frame| self.draw(frame))?;
                needs_render = false;
                self.dirty = DirtyFlags::default();
            }

            // Poll timeout dinamis supaya tetap responsif tanpa busy-spin 1ms terus.
            let next_terminal_due = TERMINAL_POLL_INTERVAL.saturating_sub(last_terminal_poll.elapsed());
            let next_cwd_due = CWD_SYNC_INTERVAL.saturating_sub(last_cwd_sync.elapsed());
            let mut wait = next_terminal_due.min(next_cwd_due);
            if pending_resize_redraw {
                let next_resize_due =
                    RESIZE_REDRAW_DEBOUNCE.saturating_sub(last_resize_event.elapsed());
                wait = wait.min(next_resize_due);
            }
            if wait > MAX_EVENT_WAIT {
                wait = MAX_EVENT_WAIT;
            }

            if event::poll(wait)? {
                match event::read()? {
                    Event::Key(key) => {
                        self.handle_key(key)?;
                        self.dirty.ui = true; // Mark UI dirty saat ada input
                        needs_render = true;
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse(mouse);
                        self.dirty.ui = true; // Mark UI dirty saat ada mouse event
                        needs_render = true;
                    }
                    Event::Paste(text) => {
                        self.handle_paste_event(text);
                        self.dirty.ui = true; // Mark UI dirty saat ada paste
                        needs_render = true;
                    }
                    Event::Resize(_, _) => {
                        self.dirty.ui = true;
                        self.dirty.terminal = true;
                        self.dirty.explorer = true;
                        pending_resize_redraw = true;
                        last_resize_event = Instant::now();
                        needs_render = true;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        ui::draw(self, frame);
    }

    fn cached_visual_width_for_active_cursor(&mut self) -> usize {
        const MAX_VISUAL_COL_CACHE: usize = 8192;
        let Some(buffer) = self.editor.current() else {
            return 0;
        };
        if buffer.cursor_row >= buffer.lines.len() {
            return 0;
        }
        let line = &buffer.lines[buffer.cursor_row];
        let mut hasher = DefaultHasher::new();
        line.hash(&mut hasher);
        let key = VisualColCacheKey {
            buffer_index: self.editor.active,
            row: buffer.cursor_row,
            col: buffer.cursor_col,
            line_hash: hasher.finish(),
        };
        if let Some(width) = self.visual_col_cache.get(&key) {
            return *width;
        }
        let width = visual_width_until(line, buffer.cursor_col);
        if self.visual_col_cache.len() >= MAX_VISUAL_COL_CACHE {
            self.visual_col_cache.clear();
        }
        self.visual_col_cache.insert(key, width);
        width
    }

    fn handle_paste_event(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        self.flush_pending_editor_paste_if_ready(true);
        self.reset_editor_insert_burst_state();
        if self.explorer_prompt.is_some() {
            self.paste_into_explorer_prompt(&text);
            return;
        }
        if self.workspace_search_mode {
            self.workspace_search_query.push_str(&normalize_single_line_text(&text));
            return;
        }

        match self.focus {
            Focus::Editor => self.paste_editor_text(&text),
            Focus::Explorer => {
                if self.editor.has_open_buffer() {
                    self.set_focus(Focus::Editor);
                    self.paste_editor_text(&text);
                }
            }
            Focus::AiChat => {
                self.insert_ai_input_text(&text);
            }
            Focus::Terminal => {
                self.paste_terminal_text(&text);
            }
            Focus::Settings | Focus::Shortcuts | Focus::About => {}
        }
    }

    fn set_focus(&mut self, focus: Focus) {
        if self.focus != focus {
            if matches!(self.focus, Focus::Editor) {
                self.flush_pending_editor_paste_if_ready(true);
            }
            if !matches!(focus, Focus::Terminal) {
                self.terminal_search_mode = false;
                self.terminal_search_query.clear();
            }
            self.workspace_search_mode = false;
            let preserve_editor_selection = matches!(
                (self.focus, focus),
                (Focus::Editor, Focus::AiChat) | (Focus::AiChat, Focus::Editor)
            );
            if !preserve_editor_selection {
                self.selection = None;
            }
            self.mouse_dragging = false;
            self.suggestion_index = 0;
            self.reset_editor_insert_burst_state();
        }
        self.focus = focus;
        if matches!(focus, Focus::Terminal) {
            if let Some(path) = self.pending_terminal_cwd.take() {
                self.explorer_root = path;
                let _ = self.refresh_explorer();
            }
        }
        if !matches!(focus, Focus::Settings) {
            self.focus_before_settings = focus;
        }
        if !matches!(focus, Focus::About) {
            self.focus_before_about = focus;
        }
        if !matches!(focus, Focus::Shortcuts) {
            self.focus_before_shortcuts = focus;
        }
    }

    fn reset_editor_insert_burst_state(&mut self) {
        self.last_editor_insert_key_at = None;
        self.editor_insert_key_streak = 0;
        self.editor_plain_paste_mode_until = None;
        self.pending_editor_paste = None;
    }

    fn queue_editor_paste_key(&mut self, key: KeyEvent) {
        let chunk = match key.code {
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => c.to_string(),
            KeyCode::Enter => "\n".to_string(),
            KeyCode::Tab if !key.modifiers.contains(KeyModifiers::SHIFT) => "\t".to_string(),
            _ => return,
        };
        let now = Instant::now();
        if let Some(pending) = self.pending_editor_paste.as_mut() {
            pending.content.push_str(&chunk);
            pending.last_update = now;
        } else {
            self.pending_editor_paste = Some(PendingEditorPaste {
                content: chunk,
                last_update: now,
            });
        }
    }

    fn flush_pending_editor_paste_if_ready(&mut self, force: bool) {
        let Some(pending) = self.pending_editor_paste.as_ref() else {
            return;
        };
        if !force && pending.last_update.elapsed() < EDITOR_PENDING_PASTE_FLUSH_DELAY {
            return;
        }
        let pending = self.pending_editor_paste.take().expect("pending exists");
        let normalized = self.normalize_editor_paste_for_active_language(&pending.content);
        if normalized.is_empty() {
            return;
        }
        self.apply_standardized_editor_paste(&normalized, false);
    }

    fn pending_editor_paste_loading_text(&self) -> Option<String> {
        let pending = self.pending_editor_paste.as_ref()?;
        let chars = pending.content.chars().count();
        let spinner = spinner_frame_from_elapsed(pending.last_update.elapsed());
        Some(format!("paste {} rewrite xone ({} chars)", spinner, chars))
    }

    fn normalize_editor_paste_for_active_language(&self, content: &str) -> String {
        let language = self
            .editor
            .current()
            .map(|buffer| buffer.language)
            .unwrap_or(language::Language::PlainText);
        rewrite_editor_paste(content, language)
    }

    fn apply_standardized_editor_paste(&mut self, normalized: &str, dedupe: bool) {
        if dedupe && self.should_skip_duplicate_paste(normalized) {
            return;
        }
        if self.selection_range(Focus::Editor).is_some() {
            self.delete_editor_selection();
        }
        let view_height = self.editor_view_height;
        let line_count = normalized.split('\n').count();
        let Some(buffer) = self.editor.current_mut() else {
            self.message = Some("Tidak ada buffer".to_string());
            return;
        };
        buffer.push_undo();
        insert_text(buffer, normalized);
        buffer.dirty = true;
        ensure_cursor_visible(buffer, view_height);
        self.selection = None;
        self.suggestion_index = 0;
        self.message = Some(format!("Paste {} baris (distandardkan Xone)", line_count));
        self.remember_paste(normalized);
    }

    fn should_use_plain_editor_insert(&mut self, key: &KeyEvent) -> bool {
        let now = Instant::now();
        let mode_active = self
            .editor_plain_paste_mode_until
            .map(|until| now <= until)
            .unwrap_or(false);

        if !is_editor_text_input_key(key) {
            self.last_editor_insert_key_at = None;
            self.editor_insert_key_streak = 0;
            if !mode_active {
                self.editor_plain_paste_mode_until = None;
            }
            return false;
        }

        if matches!(key.kind, KeyEventKind::Repeat) {            return mode_active;
        }

        if let Some(previous) = self.last_editor_insert_key_at {
            if now.duration_since(previous) <= EDITOR_PASTE_BURST_GAP {
                self.editor_insert_key_streak += 1;
            } else {
                self.editor_insert_key_streak = 1;
            }
        } else {
            self.editor_insert_key_streak = 1;
        }
        self.last_editor_insert_key_at = Some(now);

        if self.editor_insert_key_streak >= EDITOR_PASTE_BURST_STREAK {
            self.editor_plain_paste_mode_until = Some(now + EDITOR_PLAIN_MODE_TTL);
            return true;
        }

        if mode_active {
            self.editor_plain_paste_mode_until = Some(now + EDITOR_PLAIN_MODE_TTL);
            return true;
        }

        false
    }

    fn should_skip_control_action(&self, key: &KeyEvent, action: control::ControlAction) -> bool {
        if matches!(key.kind, KeyEventKind::Repeat) {
            return true;
        }
        if let Some((last_action, at)) = self.last_control_action {
            if last_action == action
                && at.elapsed() <= Duration::from_millis(160)
                && (is_debounced_control_action(action) || is_raw_control_fallback_event(key))
            {
                return true;
            }
        }

        if is_raw_control_fallback_event(key) {
            if let Some((last_action, at)) = self.last_control_action {
                if last_action == action && at.elapsed() <= Duration::from_millis(220) {
                    return true;
                }
            }
        }
        false
    }

    fn mark_control_action(&mut self, action: control::ControlAction) {
        self.last_control_action = Some((action, Instant::now()));
    }

    fn should_skip_duplicate_paste(&self, payload: &str) -> bool {
        let Some((last_hash, last_len, at)) = self.last_paste_fingerprint else {
            return false;
        };
        if at.elapsed() > Duration::from_millis(140) {
            return false;
        }
        let (hash, len) = paste_fingerprint(payload);
        hash == last_hash && len == last_len
    }

    fn remember_paste(&mut self, payload: &str) {
        let (hash, len) = paste_fingerprint(payload);
        self.last_paste_fingerprint = Some((hash, len, Instant::now()));
    }

    fn paste_into_explorer_prompt(&mut self, text: &str) {
        let text = normalize_single_line_text(text);
        if text.is_empty() {
            self.message = Some("Clipboard tidak berisi teks".to_string());
            return;
        }
        if self.should_skip_duplicate_paste(&text) {
            return;
        }
        if let Some(prompt) = self.explorer_prompt.as_mut() {
            let byte = char_to_byte_index(&prompt.value, prompt.cursor_col);
            prompt.value.insert_str(byte, &text);
            prompt.cursor_col += line_len(&text);
            self.message = Some("Paste nama/path".to_string());
            self.remember_paste(&text);
        }
    }

    fn paste_editor_text(&mut self, content: &str) {
        self.flush_pending_editor_paste_if_ready(true);
        let normalized = self.normalize_editor_paste_for_active_language(content);
        if normalized.is_empty() {
            self.message = Some("Paste kosong".to_string());
            return;
        }
        self.reset_editor_insert_burst_state();
        self.apply_standardized_editor_paste(&normalized, true);
    }

    fn paste_terminal_text(&mut self, content: &str) {
        terminal_controller::paste_terminal_text(self, content);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<(), AppError> {
        input_router::handle_key(self, key)
    }

    fn apply_command_hook(&mut self, key: &KeyEvent) -> Result<bool, AppError> {
        let Some(hook) = self
            .command_hooks
            .iter()
            .find(|hook| hook.binding.matches(key))
            .cloned()
        else {
            return Ok(false);
        };

        match hook.action {
            HookAction::Terminal(command) => {
                if !self.terminal.is_running() {
                    if let Err(error) = self.terminal.ensure_started(&self.explorer_root) {
                        self.message = Some(format!("Gagal membuka terminal: {}", error));
                        return Ok(true);
                    }
                }
                let mut payload = command;
                if !payload.ends_with('\n') && !payload.ends_with('\r') {
                    payload.push_str("\r\n");
                }
                let _ = self.terminal.send(&payload);
                self.terminal_visible = true;
                self.set_focus(Focus::Terminal);
                self.message = Some("Hook: command dikirim ke terminal".to_string());
            }
            HookAction::Message(text) => {
                self.message = Some(text);
            }
            HookAction::Open(path) => {
                let target = if path.is_absolute() {
                    path
                } else {
                    self.workspace.root().join(path)
                };
                self.open_editor_path(&target)?;
                self.set_focus(Focus::Editor);
                self.message = Some("Hook: file dibuka".to_string());
            }
        }
        Ok(true)
    }

    fn handle_terminal_key(&mut self, key: KeyEvent) {
        terminal_controller::handle_terminal_key(self, key);
    }

    fn redraw_terminal_after_resize(&mut self) {
        terminal_controller::redraw_terminal_after_resize(self);
    }

    fn open_new_terminal_tab(&mut self) {
        terminal_controller::open_new_terminal_tab(self);
    }

    fn handle_terminal_search_key(&mut self, key: KeyEvent) -> bool {
        terminal_controller::handle_terminal_search_key(self, key)
    }

    fn open_workspace_search(&mut self) {
        self.workspace_search_mode = true;
        self.workspace_search_query.clear();
        self.workspace_search_results.clear();
        self.workspace_search_selected = 0;
        self.message = Some("Workspace Search aktif. Ketik query lalu Enter.".to_string());
    }

    fn open_workspace_search_selected(&mut self) {
        let Some(hit) = self
            .workspace_search_results
            .get(self.workspace_search_selected)
            .cloned()
        else {
            return;
        };
        if self.open_editor_path(&hit.path).is_ok() {
            self.set_focus(Focus::Editor);
            if let Some(buffer) = self.editor.current_mut() {
                let row = hit.line_no.saturating_sub(1).min(buffer.lines.len().saturating_sub(1));
                buffer.cursor_row = row;
                buffer.cursor_col = 0;
                ensure_cursor_visible(buffer, self.editor_view_height);
            }
            self.message = Some(format!(
                "Search hit: {}:{} | {}",
                self.workspace.relative(&hit.path).to_string_lossy(),
                hit.line_no,
                clip_text_end(&hit.line, 72)
            ));
        }
    }

    fn run_workspace_search(&mut self) {
        let query = self.workspace_search_query.trim().to_string();
        if query.is_empty() {
            self.message = Some("Query search kosong".to_string());
            return;
        }
        self.workspace_search_results = self.workspace_search_in_files(&query, 120);
        self.workspace_search_selected = 0;
        if self.workspace_search_results.is_empty() {
            self.message = Some(format!("Tidak ada hasil untuk '{}'", query));
            return;
        }
        self.open_workspace_search_selected();
        self.message = Some(format!(
            "Workspace Search: {} hasil untuk '{}'",
            self.workspace_search_results.len(),
            query
        ));
    }

    fn workspace_search_in_files(&self, query: &str, max_hits: usize) -> Vec<WorkspaceSearchHit> {
        let mut hits = Vec::new();
        let mut stack = vec![self.workspace.root().to_path_buf()];
        let needle = query.to_ascii_lowercase();
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if path.file_name().and_then(|v| v.to_str()) == Some("target")
                        || path.file_name().and_then(|v| v.to_str()) == Some(".git")
                    {
                        continue;
                    }
                    stack.push(path);
                    continue;
                }
                let Ok(metadata) = entry.metadata() else {
                    continue;
                };
                if metadata.len() > 1_500_000 {
                    continue;
                }
                let Ok(content) = fs::read_to_string(&path) else {
                    continue;
                };
                for (idx, line) in content.lines().enumerate() {
                    if line.to_ascii_lowercase().contains(&needle) {
                        hits.push(WorkspaceSearchHit {
                            path: path.clone(),
                            line_no: idx + 1,
                            line: line.trim().to_string(),
                        });
                        if hits.len() >= max_hits {
                            return hits;
                        }
                    }
                }
            }
        }
        hits
    }

    fn handle_workspace_search_key(&mut self, key: KeyEvent) -> bool {
        if !self.workspace_search_mode {
            return false;
        }
        match key.code {
            KeyCode::Esc => {
                self.workspace_search_mode = false;
                self.message = Some("Workspace Search dibatalkan".to_string());
            }
            KeyCode::Enter => {
                self.run_workspace_search();
            }
            KeyCode::Up => {
                self.workspace_search_selected = self.workspace_search_selected.saturating_sub(1);
                self.open_workspace_search_selected();
            }
            KeyCode::Down => {
                if !self.workspace_search_results.is_empty() {
                    self.workspace_search_selected = (self.workspace_search_selected + 1)
                        .min(self.workspace_search_results.len() - 1);
                    self.open_workspace_search_selected();
                }
            }
            KeyCode::Backspace => {
                self.workspace_search_query.pop();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.workspace_search_query.push(c);
            }
            _ => {}
        }
        true
    }

    fn open_about_panel(&mut self) {
        if !matches!(self.focus, Focus::About) {
            self.focus_before_about = self.focus;
        }
        self.about_scroll = 0;
        self.set_focus(Focus::About);
        self.message = Some("About dibuka (Up/Down scroll, Esc/F1 tutup)".to_string());
    }

    fn close_about_panel(&mut self) {
        if !matches!(self.focus, Focus::About) {
            return;
        }
        let target = if self.editor.has_open_buffer() {
            Focus::Editor
        } else {
            Focus::Explorer
        };
        self.set_focus(target);
        self.message = Some("Kembali dari About".to_string());
    }

    fn toggle_about_panel(&mut self) {
        if matches!(self.focus, Focus::About) {
            self.close_about_panel();
        } else {
            self.open_about_panel();
        }
    }

    fn handle_about_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::F(1) => self.close_about_panel(),
            KeyCode::Up => self.about_scroll = self.about_scroll.saturating_sub(1),
            KeyCode::Down => self.about_scroll = self.about_scroll.saturating_add(1),
            KeyCode::PageUp => self.about_scroll = self.about_scroll.saturating_sub(8),
            KeyCode::PageDown => self.about_scroll = self.about_scroll.saturating_add(8),
            KeyCode::Home => self.about_scroll = 0,
            _ => {}
        }
    }

    fn focus_ai_chat(&mut self) {
        self.explorer_visible = true;
        self.left_panel = LeftPanel::AiChat;
        self.set_focus(Focus::AiChat);
        self.message = Some("x1.AI aktif".to_string());
    }

    fn cycle_keyboard_focus(&mut self, backward: bool) {
        let mut order: Vec<Focus> = Vec::new();
        if self.explorer_visible {
            order.push(Focus::Explorer);
        }
        if self.editor.has_open_buffer() {
            order.push(Focus::Editor);
        }
        if self.terminal_visible {
            order.push(Focus::Terminal);
        }
        order.push(Focus::AiChat);

        if order.is_empty() {
            return;
        }

        let current_index = order.iter().position(|f| *f == self.focus).unwrap_or(0);
        let next_index = if backward {
            if current_index == 0 {
                order.len() - 1
            } else {
                current_index - 1
            }
        } else {
            (current_index + 1) % order.len()
        };

        match order[next_index] {
            Focus::Explorer => {
                self.explorer_visible = true;
                self.left_panel = LeftPanel::Explorer;
                self.set_focus(Focus::Explorer);
                self.message = Some("Focus: Explorer".to_string());
            }
            Focus::Editor => {
                self.set_focus(Focus::Editor);
                self.message = Some("Focus: Editor".to_string());
            }
            Focus::Terminal => {
                self.set_focus(Focus::Terminal);
                self.message = Some("Focus: Terminal".to_string());
            }
            Focus::AiChat => {
                self.focus_ai_chat();
            }
            Focus::Settings | Focus::Shortcuts | Focus::About => {}
        }
    }

    fn insert_ai_input_text(&mut self, text: &str) {
        let sanitized = normalize_single_line_text(text);
        if sanitized.is_empty() {
            return;
        }
        let byte = char_to_byte_index(&self.ai_chat.input, self.ai_chat.cursor_col);
        self.ai_chat.input.insert_str(byte, &sanitized);
        self.ai_chat.cursor_col += line_len(&sanitized);
    }

    fn submit_ai_input(&mut self) {
        let prompt = self.ai_chat.input.trim().to_string();
        if prompt.is_empty() {
            self.message = Some("Prompt AI kosong".to_string());
            return;
        }
        if provider_requires_api_key(&self.ai_chat.config.provider)
            && self.ai_chat.config.api_key.trim().is_empty()
        {
            self.ai_chat.messages.push(AiChatMessage {
                role: AiRole::Assistant,
                content: format!(
                    "API key belum diisi untuk provider '{}'. Daftar lalu isi key di Settings > AI API Key (Enter dari clipboard).\nOpenAI: https://platform.openai.com/api-keys\nGroq: https://console.groq.com/keys\nTogether: https://api.together.xyz/settings/api-keys\nOpenRouter: https://openrouter.ai/keys\nMistral: https://console.mistral.ai/api-keys",
                    self.ai_chat.config.provider
                ),
            });
            self.message = Some("API key belum ada. Lihat link daftar provider di chat/settings.".to_string());
            return;
        }
        if self.ai_chat.inflight {
            self.message = Some("AI masih memproses request sebelumnya".to_string());
            return;
        }
        self.ai_chat.messages.push(AiChatMessage {
            role: AiRole::User,
            content: prompt,
        });
        self.ai_chat.messages.push(AiChatMessage {
            role: AiRole::Assistant,
            content: String::new(),
        });
        let prompt = self
            .ai_chat
            .messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, AiRole::User))
            .map(|m| self.build_ai_prompt_with_context(&m.content))
            .unwrap_or_default();
        let config = self.ai_chat.config.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            run_ai_worker(&config, &prompt, &tx);
        });
        self.ai_rx = Some(rx);
        self.ai_chat.inflight = true;
        self.ai_chat.input.clear();
        self.ai_chat.cursor_col = 0;
        self.ai_chat.scroll = 0;
        self.message = Some("Prompt AI dikirim".to_string());
    }

    fn handle_ai_chat_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && key.modifiers.contains(KeyModifiers::SHIFT)
            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
        {
            self.copy_latest_ai_code_block();
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('r') | KeyCode::Char('R')) {
            self.ai_chat.read_only_mode = !self.ai_chat.read_only_mode;
            self.message = Some(if self.ai_chat.read_only_mode {
                "x1.AI mode: Review (read-only)".to_string()
            } else {
                "x1.AI mode: Direct apply".to_string()
            });
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
            self.confirm_ai_review_apply();
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Enter) {
            self.request_ai_apply_to_editor(false);
            return;
        }
        if key.modifiers.contains(KeyModifiers::ALT) && matches!(key.code, KeyCode::Enter) {
            self.request_ai_apply_to_editor(true);
            return;
        }
        match key.code {
            KeyCode::Esc => {
                if self.ai_chat.pending_apply.take().is_some() {
                    self.message = Some("Review apply dibatalkan".to_string());
                    return;
                }
                if self.editor.has_open_buffer() {
                    self.set_focus(Focus::Editor);
                } else {
                    self.left_panel = LeftPanel::Explorer;
                    self.set_focus(Focus::Explorer);
                }
            }
            KeyCode::Enter => self.submit_ai_input(),
            KeyCode::Left => {
                self.ai_chat.cursor_col = self.ai_chat.cursor_col.saturating_sub(1);
            }
            KeyCode::Right => {
                let len = line_len(&self.ai_chat.input);
                self.ai_chat.cursor_col = (self.ai_chat.cursor_col + 1).min(len);
            }
            KeyCode::Home => self.ai_chat.cursor_col = 0,
            KeyCode::End => self.ai_chat.cursor_col = line_len(&self.ai_chat.input),
            KeyCode::Up => {
                self.ai_chat.scroll = self.ai_chat.scroll.saturating_add(1);
            }
            KeyCode::Down => {
                self.ai_chat.scroll = self.ai_chat.scroll.saturating_sub(1);
            }
            KeyCode::PageUp => {
                self.ai_chat.scroll = self.ai_chat.scroll.saturating_add(6);
            }
            KeyCode::PageDown => {
                self.ai_chat.scroll = self.ai_chat.scroll.saturating_sub(6);
            }
            KeyCode::Backspace => {
                if self.ai_chat.cursor_col > 0 {
                    let end = char_to_byte_index(&self.ai_chat.input, self.ai_chat.cursor_col);
                    let start =
                        char_to_byte_index(&self.ai_chat.input, self.ai_chat.cursor_col - 1);
                    self.ai_chat.input.replace_range(start..end, "");
                    self.ai_chat.cursor_col -= 1;
                }
            }
            KeyCode::Delete => {
                let len = line_len(&self.ai_chat.input);
                if self.ai_chat.cursor_col < len {
                    let start = char_to_byte_index(&self.ai_chat.input, self.ai_chat.cursor_col);
                    let end = char_to_byte_index(&self.ai_chat.input, self.ai_chat.cursor_col + 1);
                    self.ai_chat.input.replace_range(start..end, "");
                }
            }
            KeyCode::Char('l') | KeyCode::Char('L')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.ai_chat.messages.clear();
                self.ai_chat.scroll = 0;
                self.message = Some("Riwayat AI dibersihkan".to_string());
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let byte = char_to_byte_index(&self.ai_chat.input, self.ai_chat.cursor_col);
                self.ai_chat.input.insert(byte, c);
                self.ai_chat.cursor_col += 1;
            }
            _ => {}
        }
    }

    fn poll_ai_worker_events(&mut self) -> bool {
        let Some(rx) = &self.ai_rx else {
            return false;
        };
        let mut changed = false;
        while let Ok(event) = rx.try_recv() {
            match event {
                AiWorkerEvent::Chunk(text) => {
                    if let Some(last) = self.ai_chat.messages.last_mut() {
                        if matches!(last.role, AiRole::Assistant) {
                            last.content.push_str(&text);
                        }
                    }
                    changed = true;
                }
                AiWorkerEvent::Done => {
                    self.ai_chat.inflight = false;
                    self.ai_rx = None;
                    self.message = Some("AI response diterima".to_string());
                    changed = true;
                    break;
                }
                AiWorkerEvent::Error(error) => {
                    if let Some(last) = self.ai_chat.messages.last_mut() {
                        if matches!(last.role, AiRole::Assistant) {
                            last.content = format!("Error: {}", error);
                        }
                    } else {
                        self.ai_chat.messages.push(AiChatMessage {
                            role: AiRole::Assistant,
                            content: format!("Error: {}", error),
                        });
                    }
                    self.ai_chat.inflight = false;
                    self.ai_rx = None;
                    self.message = Some(format!("AI error: {}", error));
                    changed = true;
                    break;
                }
            }
        }
        changed
    }

    fn build_ai_prompt_with_context(&self, user_prompt: &str) -> String {
        let mut parts = vec![format!("User prompt:\n{}", user_prompt)];
        if let Some(buffer) = self.editor.current() {
            let rel = self.workspace.relative(&buffer.path).to_string_lossy().to_string();
            parts.push(format!(
                "Context file: {} | language: {}",
                rel,
                language::language_label(buffer.language)
            ));
            if let Some((start, end)) = self.selection_range(Focus::Editor) {
                let last = buffer.lines.len().saturating_sub(1);
                let start = start.min(last);
                let end = end.min(last);
                let selected = buffer.lines[start..=end].join("\n");
                parts.push(format!(
                    "Selected code (lines {}..{}):\n{}",
                    start + 1,
                    end + 1,
                    selected
                ));
            }
        }
        parts.join("\n\n")
    }

    fn latest_assistant_reply(&self) -> Option<&str> {
        self.ai_chat
            .messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, AiRole::Assistant) && !m.content.trim().is_empty())
            .map(|m| m.content.as_str())
    }

    fn copy_latest_ai_code_block(&mut self) {
        let Some(reply) = self.latest_assistant_reply() else {
            self.message = Some("Belum ada jawaban AI".to_string());
            return;
        };
        let Some(code) = extract_latest_fenced_code_block(reply) else {
            self.message = Some("Tidak ada code block untuk disalin".to_string());
            return;
        };
        self.store_clipboard_text(code);
        self.message = Some("Code block terakhir tersalin".to_string());
    }

    fn request_ai_apply_to_editor(&mut self, replace_selection: bool) {
        let Some(reply) = self.latest_assistant_reply().map(|v| v.to_string()) else {
            self.message = Some("Belum ada jawaban AI untuk dimasukkan".to_string());
            return;
        };
        if self.ai_chat.read_only_mode {
            let diff_lines = self.build_ai_apply_diff_preview(&reply, replace_selection);
            self.ai_chat.pending_apply = Some(AiApplyPreview {
                content: reply,
                replace_selection,
                diff_lines,
            });
            self.message = Some("Review siap. Ctrl+Y untuk apply, Esc untuk batal".to_string());
            return;
        }
        self.apply_ai_to_editor(reply, replace_selection);
    }

    fn build_ai_apply_diff_preview(&self, reply: &str, replace_selection: bool) -> Vec<String> {
        let mut lines = Vec::new();
        let before = if replace_selection {
            if let Some(buffer) = self.editor.current() {
                if let Some((start, end)) = self.selection_range(Focus::Editor) {
                    let last = buffer.lines.len().saturating_sub(1);
                    let s = start.min(last);
                    let e = end.min(last);
                    buffer.lines[s..=e].join("\n")
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        lines.push(format!(
            "@ mode={} before={} chars after={} chars",
            if replace_selection { "replace" } else { "insert" },
            before.chars().count(),
            reply.chars().count()
        ));
        for line in before.lines().take(3) {
            lines.push(format!("- {}", clip_text_end(line, 90)));
        }
        for line in reply.lines().take(6) {
            lines.push(format!("+ {}", clip_text_end(line, 90)));
        }
        if reply.lines().count() > 6 {
            lines.push("...".to_string());
        }
        lines
    }

    fn confirm_ai_review_apply(&mut self) {
        let Some(preview) = self.ai_chat.pending_apply.take() else {
            self.message = Some("Tidak ada review apply aktif".to_string());
            return;
        };
        self.apply_ai_to_editor(preview.content, preview.replace_selection);
    }

    fn apply_ai_to_editor(&mut self, reply: String, replace_selection: bool) {
        if self.run_editor_shortcut(true, |app| {
            if replace_selection && app.selection_range(Focus::Editor).is_some() {
                app.delete_editor_selection();
            }
            let view_height = app.editor_view_height;
            if let Some(buffer) = app.editor.current_mut() {
                buffer.push_undo();
                insert_text(buffer, &reply);
                buffer.dirty = true;
                ensure_cursor_visible(buffer, view_height);
            }
        }).is_ok() {
            self.message = Some(if replace_selection {
                "Jawaban AI menggantikan seleksi editor".to_string()
            } else {
                "Jawaban AI disisipkan ke editor".to_string()
            });
        }
    }

    fn handle_escape_action(&mut self) {
        if matches!(self.focus, Focus::Settings) {
            self.close_settings_panel();
            return;
        }
        if matches!(self.focus, Focus::Shortcuts) {
            self.close_shortcuts_panel();
            return;
        }
        if matches!(self.focus, Focus::About) {
            self.close_about_panel();
            return;
        }
        if matches!(self.focus, Focus::AiChat) {
            if self.editor.has_open_buffer() {
                self.set_focus(Focus::Editor);
            } else {
                self.left_panel = LeftPanel::Explorer;
                self.set_focus(Focus::Explorer);
            }
            return;
        }
        let mut cancelled = false;
        if self.selection.take().is_some() {
            cancelled = true;
        }
        self.mouse_dragging = false;

        if cancelled {
            self.message = Some("Batal".to_string());
        }
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        mouse_router::handle_mouse(self, mouse);
    }

    fn start_selection(&mut self, panel: Focus, row: usize) {
        self.selection = Some(PanelSelection {
            panel,
            anchor_row: row,
            head_row: row,
        });
    }

    fn update_selection(&mut self, panel: Focus, row: usize) {
        if let Some(selection) = self.selection.as_mut() {
            if selection.panel == panel {
                selection.head_row = row;
            }
        }
    }

    fn selection_range(&self, panel: Focus) -> Option<(usize, usize)> {
        let selection = self.selection?;
        if selection.panel != panel {
            return None;
        }
        let start = selection.anchor_row.min(selection.head_row);
        let end = selection.anchor_row.max(selection.head_row);
        Some((start, end))
    }

    fn apply_control(&mut self, action: control::ControlAction) -> Result<(), AppError> {
        match action {
            control::ControlAction::FocusExplorer => {
                self.explorer_visible = true;
                self.left_panel = LeftPanel::Explorer;
                self.set_focus(Focus::Explorer);
            }
            control::ControlAction::FocusEditor => {
                if !self.editor.has_open_buffer() {
                    let selected_file = self
                        .explorer
                        .items
                        .get(self.explorer.selected)
                        .filter(|item| !item.is_dir)
                        .map(|item| item.path.clone());
                    if let Some(path) = selected_file {
                        self.open_editor_path(&path)?;
                    }
                }
                if self.editor.has_open_buffer() {
                    self.set_focus(Focus::Editor);
                } else {
                    self.message = Some("Pilih file di Explorer lalu tekan Enter".to_string());
                }
            }
            control::ControlAction::OpenSettings => {
                self.toggle_settings_panel();
            }
            control::ControlAction::ToggleExplorer => {
                self.explorer_visible = !self.explorer_visible;
                if !self.explorer_visible {
                    if matches!(self.focus, Focus::Explorer | Focus::AiChat)
                        && self.editor.has_open_buffer()
                    {
                        self.set_focus(Focus::Editor);
                    }
                }
            }
            control::ControlAction::ToggleTerminalPanel => {
                self.toggle_terminal_panel();
            }
            control::ControlAction::Save => self.save_editor()?,
            control::ControlAction::CloseBuffer => self.close_active_buffer(),
            control::ControlAction::Quit => self.request_quit(false),
            control::ControlAction::Undo => self.undo_editor(),
            control::ControlAction::Redo => self.redo_editor(),
            control::ControlAction::ToggleBookmark => self.toggle_bookmark(),
            control::ControlAction::NextBookmark => self.goto_next_bookmark(),
            control::ControlAction::PrevBookmark => self.goto_prev_bookmark(),
            control::ControlAction::NextBuffer => {
                self.editor.next();
                self.suggestion_index = 0;
            }
            control::ControlAction::PrevBuffer => {
                self.editor.prev();
                self.suggestion_index = 0;
            }
            control::ControlAction::EditorSelectAll => self.run_editor_shortcut(false, |app| {
                app.select_all_editor();
            })?,
            control::ControlAction::EditorCopy => self.run_editor_shortcut(false, |app| {
                app.copy_editor_selection();
            })?,
            control::ControlAction::EditorPaste => self.run_editor_shortcut(false, |app| {
                app.paste_editor_clipboard();
            })?,
            control::ControlAction::EditorCut => self.run_editor_shortcut(false, |app| {
                app.copy_editor_selection();
                app.delete_editor_selection();
            })?,
            control::ControlAction::EditorDeleteSelection => {
                self.run_editor_shortcut(false, |app| {
                    app.delete_editor_selection();
                })?
            }
            control::ControlAction::GlobalEditorSelectAll => {
                self.run_editor_shortcut(true, |app| {
                    app.select_all_editor();
                })?
            }
            control::ControlAction::GlobalEditorCopy => self.run_editor_shortcut(true, |app| {
                app.copy_editor_selection();
            })?,
            control::ControlAction::GlobalEditorPaste => self.run_editor_shortcut(true, |app| {
                app.paste_editor_clipboard();
            })?,
            control::ControlAction::GlobalEditorCut => self.run_editor_shortcut(true, |app| {
                app.copy_editor_selection();
                app.delete_editor_selection();
            })?,
            control::ControlAction::GlobalEditorDeleteSelection => {
                self.run_editor_shortcut(true, |app| {
                    app.delete_editor_selection();
                })?
            }
            control::ControlAction::AcceptSuggestion => self.run_editor_shortcut(false, |app| {
                app.apply_editor_suggestion();
            })?,
            control::ControlAction::NextSuggestion => self.run_editor_shortcut(false, |app| {
                app.next_editor_suggestion();
            })?,
            control::ControlAction::CreateFile => self.start_create_file_prompt(),
            control::ControlAction::CreateFolder => self.start_create_folder_prompt(),
        }
        Ok(())
    }

    fn toggle_terminal_panel(&mut self) {
        terminal_controller::toggle_terminal_panel(self);
    }

    fn handle_explorer_key(&mut self, key: KeyEvent) -> Result<(), AppError> {
        explorer_controller::handle_explorer_key(self, key)
    }

    fn handle_explorer_prompt_key(&mut self, key: KeyEvent) -> Result<(), AppError> {
        explorer_controller::handle_explorer_prompt_key(self, key)
    }

    fn start_create_file_prompt(&mut self) {
        explorer_controller::start_explorer_prompt(self, ExplorerPromptKind::File);
    }

    fn start_create_folder_prompt(&mut self) {
        explorer_controller::start_explorer_prompt(self, ExplorerPromptKind::Folder);
    }

    fn handle_editor_key(&mut self, key: KeyEvent) -> Result<(), AppError> {
        let plain_insert_mode = self.should_use_plain_editor_insert(&key);
        if plain_insert_mode && is_editor_text_input_key(&key) {
            self.queue_editor_paste_key(key);
            return Ok(());
        }
        self.flush_pending_editor_paste_if_ready(true);
        if key.code == KeyCode::Delete && self.selection_range(Focus::Editor).is_some() {
            self.delete_editor_selection();
            return Ok(());
        }
        let view_height = self.editor_view_height;
        let mut context_changed = false;
        let Some(buffer) = self.editor.current_mut() else {
            self.message = Some("Belum ada file terbuka di editor".to_string());
            return Ok(());
        };
        match key.code {
            KeyCode::Up => {
                context_changed = true;
                if buffer.cursor_row > 0 {
                    buffer.cursor_row -= 1;
                    let len = line_len(&buffer.lines[buffer.cursor_row]);
                    buffer.cursor_col = buffer.cursor_col.min(len);
                }
            }
            KeyCode::Down => {
                context_changed = true;
                if buffer.cursor_row + 1 < buffer.lines.len() {
                    buffer.cursor_row += 1;
                    let len = line_len(&buffer.lines[buffer.cursor_row]);
                    buffer.cursor_col = buffer.cursor_col.min(len);
                }
            }
            KeyCode::Left => {
                context_changed = true;
                if buffer.cursor_col > 0 {
                    buffer.cursor_col -= 1;
                } else if buffer.cursor_row > 0 {
                    buffer.cursor_row -= 1;
                    buffer.cursor_col = line_len(&buffer.lines[buffer.cursor_row]);
                }
            }
            KeyCode::Right => {
                context_changed = true;
                let len = line_len(&buffer.lines[buffer.cursor_row]);
                if buffer.cursor_col < len {
                    buffer.cursor_col += 1;
                } else if buffer.cursor_row + 1 < buffer.lines.len() {
                    buffer.cursor_row += 1;
                    buffer.cursor_col = 0;
                }
            }
            KeyCode::Home => {
                context_changed = true;
                buffer.cursor_col = 0;
            }
            KeyCode::End => {
                context_changed = true;
                buffer.cursor_col = line_len(&buffer.lines[buffer.cursor_row]);
            }
            KeyCode::PageUp => {
                context_changed = true;
                buffer.cursor_row = buffer.cursor_row.saturating_sub(10);
                buffer.cursor_col = buffer
                    .cursor_col
                    .min(line_len(&buffer.lines[buffer.cursor_row]));
            }
            KeyCode::PageDown => {
                context_changed = true;
                buffer.cursor_row =
                    (buffer.cursor_row + 10).min(buffer.lines.len().saturating_sub(1));
                buffer.cursor_col = buffer
                    .cursor_col
                    .min(line_len(&buffer.lines[buffer.cursor_row]));
            }
            KeyCode::Enter => {
                context_changed = true;
                if plain_insert_mode {
                    insert_newline_plain(buffer);
                } else {
                    insert_newline(buffer);
                }
            }
            KeyCode::Backspace => {
                context_changed = true;
                backspace(buffer);
            }
            KeyCode::Delete => {
                context_changed = true;
                delete_char(buffer);
            }
            KeyCode::Tab => {
                context_changed = true;
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    outdent_current_line(buffer);
                } else {
                    insert_indent(buffer);
                }
            }
            KeyCode::BackTab => {
                context_changed = true;
                outdent_current_line(buffer);
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    context_changed = true;
                    if plain_insert_mode {
                        insert_char(buffer, c);
                    } else if try_insert_auto_pair(buffer, c) || try_skip_existing_closer(buffer, c)
                    {
                        // handled by helper
                    } else {
                        insert_char(buffer, c);
                    }
                }
            }
            _ => {}
        }
        ensure_cursor_visible(buffer, view_height);
        if context_changed {
            self.suggestion_index = 0;
        }
        Ok(())
    }

    fn refresh_explorer(&mut self) -> Result<(), AppError> {
        // Debouncing: skip jika terlalu sering (dipercepat dari 50ms ke 20ms)
        if let Some(last_refresh) = self.last_explorer_refresh {
            if last_refresh.elapsed() < Duration::from_millis(20) {
                return Ok(());
            }
        }

        let tree = self
            .workspace
            .list_tree_at(&self.explorer_root, self.tree_depth)?;
        self.explorer.items = flatten_tree(&tree, 0);
        if self.explorer.selected >= self.explorer.items.len() {
            self.explorer.selected = 0;
        }

        // Update timestamp untuk debouncing
        self.last_explorer_refresh = Some(Instant::now());
        Ok(())
    }

    fn sync_terminal_cwd(&mut self) -> bool {
        terminal_controller::sync_terminal_cwd(self)
    }

    fn update_terminal_cwd(&mut self, path: &Path) {
        terminal_controller::update_terminal_cwd(self, path);
    }

    fn open_editor_path(&mut self, path: &Path) -> Result<(), AppError> {
        let resolved = self.workspace.resolve(path)?;
        if let Some(index) = self
            .editor
            .buffers
            .iter()
            .position(|buffer| buffer.path == resolved)
        {
            self.editor.activate(index);
            self.message = Some("Buffer sudah terbuka".to_string());
            return Ok(());
        }

        let content = self.workspace.read_file(&resolved)?;
        let line_ending = detect_line_ending(&content);
        let mut lines: Vec<String> = content
            .lines()
            .map(|line| line.replace('\t', INDENT))
            .collect();
        if content.ends_with('\n') {
            lines.push(String::new());
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        let detected_language = language::detect_language(&resolved);

        self.editor.buffers.push(EditorBuffer {
            path: resolved,
            language: detected_language,
            lines,
            cursor_row: 0,
            cursor_col: 0,
            scroll: 0,
            hscroll: 0,
            dirty: false,
            bookmarks: BTreeSet::new(),
            line_ending,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        });
        self.editor.active = self.editor.buffers.len() - 1;
        Ok(())
    }

    fn save_editor(&mut self) -> Result<(), AppError> {
        let Some(index) = self
            .editor
            .buffers
            .get(self.editor.active)
            .map(|_| self.editor.active)
        else {
            self.message = Some("Tidak ada file".to_string());
            return Ok(());
        };

        match self.save_buffer_by_index(index, true)? {
            true => self.refresh_explorer()?,
            false => {}
        }
        Ok(())
    }

    fn autosave_dirty_buffers(&mut self) -> Result<usize, AppError> {
        let dirty_indices: Vec<usize> = self
            .editor
            .buffers
            .iter()
            .enumerate()
            .filter_map(|(idx, buffer)| if buffer.dirty { Some(idx) } else { None })
            .collect();
        if dirty_indices.is_empty() {
            return Ok(0);
        }

        let mut saved = 0usize;
        for index in dirty_indices {
            if self.save_buffer_by_index(index, false)? {
                saved += 1;
            }
        }
        if saved > 0 {
            self.refresh_explorer()?;
        }
        Ok(saved)
    }

    fn save_buffer_by_index(&mut self, index: usize, report_message: bool) -> Result<bool, AppError> {
        let Some(buffer_ref) = self.editor.buffers.get(index) else {
            return Ok(false);
        };
        let path = buffer_ref.path.clone();
        let content = buffer_ref.lines.join(buffer_ref.line_ending.as_str());

        let mut saved_outside_workspace = false;
        let save_result = match self.workspace.write_file(&path, &content) {
            Ok(_) => Ok(()),
            Err(CoreError::OutsideRoot(_)) => {
                saved_outside_workspace = true;
                save_external_file(&path, &content).map_err(CoreError::from)
            }
            Err(error) => Err(error),
        };

        match save_result {
            Ok(_) => {
                if let Some(buffer) = self.editor.buffers.get_mut(index) {
                    buffer.dirty = false;
                }
                if report_message {
                    self.message = Some(if saved_outside_workspace {
                        "Tersimpan (tab luar workspace)".to_string()
                    } else {
                        "Tersimpan".to_string()
                    });
                }
                Ok(true)
            }
            Err(error) => {
                self.message = Some(format!("Gagal menyimpan: {}", error));
                Ok(false)
            }
        }
    }

    fn undo_editor(&mut self) {
        let Some(buffer) = self.editor.current_mut() else {
            self.message = Some("Tidak ada buffer".to_string());
            return;
        };
        if buffer.undo() {
            ensure_cursor_visible(buffer, self.editor_view_height);
        } else {
            self.message = Some("Tidak ada riwayat undo".to_string());
        }
    }

    fn redo_editor(&mut self) {
        let Some(buffer) = self.editor.current_mut() else {
            self.message = Some("Tidak ada buffer".to_string());
            return;
        };
        if buffer.redo() {
            ensure_cursor_visible(buffer, self.editor_view_height);
        } else {
            self.message = Some("Tidak ada riwayat redo".to_string());
        }
    }

    fn toggle_bookmark(&mut self) {
        let Some(buffer) = self.editor.current_mut() else {
            self.message = Some("Tidak ada buffer".to_string());
            return;
        };
        if buffer.toggle_bookmark() {
            self.message = Some(format!(
                "Bookmark ditambah di baris {}",
                buffer.cursor_row + 1
            ));
        } else {
            self.message = Some(format!(
                "Bookmark dihapus dari baris {}",
                buffer.cursor_row + 1
            ));
        }
    }

    fn goto_next_bookmark(&mut self) {
        let height = self.editor_view_height;
        let Some(buffer) = self.editor.current_mut() else {
            self.message = Some("Tidak ada buffer".to_string());
            return;
        };
        if let Some(row) = buffer.next_bookmark_row() {
            buffer.cursor_row = row;
            buffer.cursor_col = buffer.cursor_col.min(line_len(&buffer.lines[row]));
            ensure_cursor_visible(buffer, height);
            self.message = Some(format!("Loncat ke bookmark baris {}", row + 1));
        } else {
            self.message = Some("Tidak ada bookmark".to_string());
        }
    }

    fn goto_prev_bookmark(&mut self) {
        let height = self.editor_view_height;
        let Some(buffer) = self.editor.current_mut() else {
            self.message = Some("Tidak ada buffer".to_string());
            return;
        };
        if let Some(row) = buffer.prev_bookmark_row() {
            buffer.cursor_row = row;
            buffer.cursor_col = buffer.cursor_col.min(line_len(&buffer.lines[row]));
            ensure_cursor_visible(buffer, height);
            self.message = Some(format!("Loncat ke bookmark baris {}", row + 1));
        } else {
            self.message = Some("Tidak ada bookmark".to_string());
        }
    }

    fn close_active_buffer(&mut self) {
        let Some(buffer) = self.editor.current() else {
            self.message = Some("Tidak ada buffer".to_string());
            return;
        };

        if buffer.dirty {
            self.message = Some("Buffer belum disimpan. Simpan dulu dengan Ctrl+S".to_string());
            return;
        }

        self.editor.buffers.remove(self.editor.active);
        if self.editor.active >= self.editor.buffers.len() && !self.editor.buffers.is_empty() {
            self.editor.active = self.editor.buffers.len() - 1;
        }
        if self.editor.buffers.is_empty() {
            self.editor.active = 0;
            self.set_focus(Focus::Explorer);
        }
        self.suggestion_index = 0;
    }

    fn request_quit(&mut self, force: bool) {
        if self.editor.has_dirty_buffer() && !force {
            self.message =
                Some("Ada perubahan belum disimpan. Simpan dulu dengan Ctrl+S".to_string());
            return;
        }
        self.quit = true;
    }
}

fn is_interactive_terminal() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

fn load_shortcut_bindings(root: &Path) -> Vec<(control::ControlAction, control::ShortcutBinding)> {
    let mut bindings: Vec<(control::ControlAction, control::ShortcutBinding)> = control::customizable_actions()
        .iter()
        .map(|action| (*action, control::default_binding(*action)))
        .collect();
    let path = root.join(".xone").join("shortcuts.conf");
    let Ok(content) = fs::read_to_string(path) else {
        return bindings;
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let Some(action) = control::action_from_config_key(k.trim()) else {
            continue;
        };
        let Some(binding) = control::parse_binding(v.trim()) else {
            continue;
        };
        if let Some((_, current)) = bindings.iter_mut().find(|(a, _)| *a == action) {
            *current = binding;
        }
    }
    bindings
}

fn save_shortcut_bindings(
    root: &Path,
    bindings: &[(control::ControlAction, control::ShortcutBinding)],
) {
    let dir = root.join(".xone");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join("shortcuts.conf");
    let mut out = String::from("# action=keybind\n");
    for action in control::customizable_actions() {
        let binding = bindings
            .iter()
            .find(|(a, _)| a == action)
            .map(|(_, b)| *b)
            .unwrap_or_else(|| control::default_binding(*action));
        out.push_str(control::action_config_key(*action));
        out.push('=');
        out.push_str(&control::binding_to_text(binding));
        out.push('\n');
    }
    let _ = fs::write(path, out);
}

fn load_ui_preferences(
    root: &Path,
) -> (
    style::AppearancePreset,
    style::AccentPreset,
    style::TabThemePreset,
    style::SyntaxThemePreset,
    style::UiDensity,
    bool,
    bool,
    bool,
    bool,
) {
    let path = root.join(".xone").join("ui.conf");
    let Ok(content) = fs::read_to_string(path) else {
        return (
            style::AppearancePreset::Classic,
            style::AccentPreset::Blue,
            style::TabThemePreset::Soft,
            style::SyntaxThemePreset::Soft,
            style::UiDensity::Comfortable,
            true,
            true,
            false,
            true,
        );
    };
    let mut appearance = style::AppearancePreset::Classic;
    let mut accent = style::AccentPreset::Blue;
    let mut tab_theme = style::TabThemePreset::Soft;
    let mut syntax_theme = style::SyntaxThemePreset::Soft;
    let mut density = style::UiDensity::Comfortable;
    let mut terminal_fx = true;
    let mut terminal_command_sync = true;
    let mut hard_mode = false;
    let mut adaptive_terminal_height = true;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case("appearance") {
            if let Some(preset) = style::AppearancePreset::from_str(value.trim()) {
                appearance = preset;
            }
        }
        if key.trim().eq_ignore_ascii_case("accent") {
            if let Some(v) = style::AccentPreset::from_str(value.trim()) {
                accent = v;
            }
        }
        if key.trim().eq_ignore_ascii_case("density") {
            if let Some(v) = style::UiDensity::from_str(value.trim()) {
                density = v;
            }
        }
        if key.trim().eq_ignore_ascii_case("tab_theme") {
            if let Some(v) = style::TabThemePreset::from_str(value.trim()) {
                tab_theme = v;
            }
        }
        if key.trim().eq_ignore_ascii_case("syntax_theme") {
            if let Some(v) = style::SyntaxThemePreset::from_str(value.trim()) {
                syntax_theme = v;
            }
        }
        if key.trim().eq_ignore_ascii_case("terminal_fx") {
            terminal_fx = matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes");
        }
        if key.trim().eq_ignore_ascii_case("hard_mode") {
            hard_mode = matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes");
        }
        if key.trim().eq_ignore_ascii_case("terminal_command_sync") {
            terminal_command_sync =
                matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes");
        }
        if key.trim().eq_ignore_ascii_case("adaptive_terminal_height") {
            adaptive_terminal_height =
                matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes");
        }
    }
    (
        appearance,
        accent,
        tab_theme,
        syntax_theme,
        density,
        terminal_fx,
        terminal_command_sync,
        hard_mode,
        adaptive_terminal_height,
    )
}

fn load_ai_config(root: &Path) -> AiConfig {
    let mut config = AiConfig {
        provider: "openai_compatible".to_string(),
        base_url: "https://api.groq.com/openai".to_string(),
        model: "llama-3.1-8b-instant".to_string(),
        api_key: String::new(),
    };
    let candidates = [
        root.join(".xone").join("ai").join("ai.conf"),
        root.join(".xone").join("ai.conf"),
    ];
    for path in candidates {
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            match key.as_str() {
                "provider" if config.provider == "openai_compatible" => config.provider = value,
                "base_url" if config.base_url == "https://api.groq.com/openai" => {
                    config.base_url = value
                }
                "model" if config.model == "llama-3.1-8b-instant" => config.model = value,
                "api_key" if config.api_key.is_empty() => config.api_key = value,
                _ => {}
            }
        }
        break;
    }
    config
}

fn load_about_lines(root: &Path) -> Vec<String> {
    let path = root.join("about").join("xone-profile.md");
    let Ok(content) = fs::read_to_string(path) else {
        return vec![
            "About belum tersedia.".to_string(),
            "Buat file about/xone-profile.md untuk menampilkan profil aplikasi.".to_string(),
        ];
    };
    let mut lines: Vec<String> = content.lines().map(|v| v.to_string()).collect();
    if lines.is_empty() {
        lines.push("About kosong.".to_string());
    }
    lines
}

fn infer_ai_profile(config: &AiConfig) -> AiProfile {
    let base = config.base_url.trim().to_ascii_lowercase();
    if base.contains("api.groq.com") {
        return AiProfile::Groq;
    }
    if base.contains("api.together.xyz") {
        return AiProfile::Together;
    }
    if base.contains("openrouter.ai") {
        return AiProfile::OpenRouter;
    }
    AiProfile::Groq
}

fn config_for_profile(profile: AiProfile, existing_api_key: &str) -> AiConfig {
    match profile {
        AiProfile::Groq => AiConfig {
            provider: "openai_compatible".to_string(),
            base_url: "https://api.groq.com/openai".to_string(),
            model: "llama-3.1-8b-instant".to_string(),
            api_key: existing_api_key.to_string(),
        },
        AiProfile::Together => AiConfig {
            provider: "openai_compatible".to_string(),
            base_url: "https://api.together.xyz".to_string(),
            model: "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo".to_string(),
            api_key: existing_api_key.to_string(),
        },
        AiProfile::OpenRouter => AiConfig {
            provider: "openai_compatible".to_string(),
            base_url: "https://openrouter.ai/api".to_string(),
            model: "openai/gpt-4o-mini".to_string(),
            api_key: existing_api_key.to_string(),
        },
    }
}

fn request_ai_completion(config: &AiConfig, prompt: &str) -> Result<String, String> {
    if prompt.trim().is_empty() {
        return Err("prompt kosong".to_string());
    }
    let provider = config.provider.trim().to_ascii_lowercase();
    if provider == "ollama" {
        request_ollama_completion(config, prompt)
    } else {
        request_openai_compatible_completion(config, prompt)
    }
}

fn provider_requires_api_key(provider: &str) -> bool {
    !provider.trim().eq_ignore_ascii_case("ollama")
}

fn run_ai_worker(config: &AiConfig, prompt: &str, tx: &mpsc::Sender<AiWorkerEvent>) {
    match config.provider.trim().to_ascii_lowercase().as_str() {
        "ollama" => {
            if let Err(error) = request_ollama_streaming(config, prompt, tx) {
                let _ = tx.send(AiWorkerEvent::Error(error));
                return;
            }
            let _ = tx.send(AiWorkerEvent::Done);
        }
        _ => {
            match request_ai_completion(config, prompt) {
                Ok(text) => {
                    let _ = tx.send(AiWorkerEvent::Chunk(text));
                    let _ = tx.send(AiWorkerEvent::Done);
                }
                Err(error) => {
                    let _ = tx.send(AiWorkerEvent::Error(error));
                }
            }
        }
    }
}

fn request_ollama_completion(config: &AiConfig, prompt: &str) -> Result<String, String> {
    let base = config.base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err("base_url kosong di ai.conf".to_string());
    }
    let model = config.model.trim();
    if model.is_empty() {
        return Err("model kosong di ai.conf".to_string());
    }
    let url = format!("{}/api/generate", base);
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false
    });

    let mut request = ureq::post(&url).set("Content-Type", "application/json");
    if !config.api_key.trim().is_empty() {
        request = request.set("Authorization", &format!("Bearer {}", config.api_key.trim()));
    }
    let response = request
        .send_json(body)
        .map_err(|error| map_provider_request_error("ollama", &url, error))?;

    let value: serde_json::Value = response
        .into_json()
        .map_err(|error| format!("response Ollama tidak valid: {}", error))?;
    let text = value
        .get("response")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if text.is_empty() {
        return Err("response Ollama kosong".to_string());
    }
    Ok(text)
}

fn request_ollama_streaming(
    config: &AiConfig,
    prompt: &str,
    tx: &mpsc::Sender<AiWorkerEvent>,
) -> Result<(), String> {
    let base = config.base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err("base_url kosong di ai.conf".to_string());
    }
    let model = config.model.trim();
    if model.is_empty() {
        return Err("model kosong di ai.conf".to_string());
    }
    let url = format!("{}/api/generate", base);
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": true
    });

    let mut request = ureq::post(&url).set("Content-Type", "application/json");
    if !config.api_key.trim().is_empty() {
        request = request.set("Authorization", &format!("Bearer {}", config.api_key.trim()));
    }
    let response = request
        .send_json(body)
        .map_err(|error| map_provider_request_error("ollama", &url, error))?;
    let reader = response.into_reader();
    let mut lines = BufReader::new(reader).lines();
    while let Some(Ok(line)) = lines.next() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(chunk) = value.get("response").and_then(|v| v.as_str()) {
            if !chunk.is_empty() {
                let _ = tx.send(AiWorkerEvent::Chunk(chunk.to_string()));
            }
        }
        if value.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
            break;
        }
    }
    Ok(())
}

fn request_openai_compatible_completion(config: &AiConfig, prompt: &str) -> Result<String, String> {
    let base = config.base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err("base_url kosong di ai.conf".to_string());
    }
    let model = config.model.trim();
    if model.is_empty() {
        return Err("model kosong di ai.conf".to_string());
    }
    let api_key = config.api_key.trim();
    if api_key.is_empty() {
        return Err("api_key kosong untuk provider openai-compatible".to_string());
    }

    let url = if base.ends_with("/chat/completions") {
        base.to_string()
    } else if base.ends_with("/v1") {
        format!("{}/chat/completions", base)
    } else {
        format!("{}/v1/chat/completions", base)
    };
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.7
    });

    let response = ureq::post(&url)
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("Bearer {}", api_key))
        .send_json(body)
        .map_err(|error| map_provider_request_error("openai-compatible", &url, error))?;

    let value: serde_json::Value = response
        .into_json()
        .map_err(|error| format!("response provider tidak valid: {}", error))?;

    let text = value
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("message"))
        .and_then(|msg| msg.get("content"))
        .and_then(|content| content.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if text.is_empty() {
        return Err("response provider kosong".to_string());
    }
    Ok(text)
}

fn map_provider_request_error(provider: &str, url: &str, error: ureq::Error) -> String {
    match error {
        ureq::Error::Status(code, resp) => {
            let status_text = resp.status_text().to_string();
            let raw = resp.into_string().unwrap_or_default();
            let detail = extract_provider_error_detail(&raw);
            let hint = match code {
                400 => "Request tidak valid. Cek model, format endpoint, atau parameter request.",
                401 => "API key tidak valid atau belum diisi.",
                403 => "Akses ditolak. Akun/key tidak punya izin ke model ini.",
                404 => "Endpoint/model tidak ditemukan. Cek base_url dan nama model.",
                408 => "Request timeout. Cek koneksi atau coba ulang.",
                429 => "Rate limit/quota tercapai. Tunggu sebentar atau cek billing/usage provider.",
                500 | 502 | 503 | 504 => "Server provider sedang bermasalah. Coba ulang beberapa saat lagi.",
                _ => "Request ke provider gagal.",
            };
            let clipped = if detail.is_empty() {
                String::new()
            } else {
                format!(" | detail={}", clip_text_end(&detail, 240))
            };
            format!(
                "HTTP {} {} [{}] | {} | url={}{}",
                code, status_text, provider, hint, url, clipped
            )
        }
        ureq::Error::Transport(e) => {
            format!("gagal konek ke {}: {} | url={}", provider, e, url)
        }
    }
}

fn extract_provider_error_detail(raw: &str) -> String {
    if raw.trim().is_empty() {
        return String::new();
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(text) = value
            .get("error")
            .and_then(|v| v.get("message"))
            .and_then(|v| v.as_str())
        {
            return text.trim().to_string();
        }
        if let Some(text) = value.get("message").and_then(|v| v.as_str()) {
            return text.trim().to_string();
        }
    }
    raw.trim().to_string()
}

fn clip_text_end(input: &str, max_chars: usize) -> String {
    let max_chars = max_chars.max(1);
    let chars: Vec<char> = input.chars().collect();
    if chars.len() <= max_chars {
        return input.to_string();
    }
    let keep = max_chars.saturating_sub(3).max(1);
    let mut out: String = chars.into_iter().take(keep).collect();
    out.push_str("...");
    out
}

fn save_ui_preferences(
    root: &Path,
    appearance: style::AppearancePreset,
    accent: style::AccentPreset,
    tab_theme: style::TabThemePreset,
    syntax_theme: style::SyntaxThemePreset,
    density: style::UiDensity,
    terminal_fx: bool,
    terminal_command_sync: bool,
    hard_mode: bool,
    adaptive_terminal_height: bool,
) -> io::Result<()> {
    let dir = root.join(".xone");
    fs::create_dir_all(&dir)?;
    let path = dir.join("ui.conf");
    let body = format!(
        "appearance={}\naccent={}\ntab_theme={}\nsyntax_theme={}\ndensity={}\nterminal_fx={}\nterminal_command_sync={}\nhard_mode={}\nadaptive_terminal_height={}\n",
        appearance.as_str(),
        accent.as_str(),
        tab_theme.as_str(),
        syntax_theme.as_str(),
        density.as_str(),
        if terminal_fx { "true" } else { "false" },
        if terminal_command_sync { "true" } else { "false" },
        if hard_mode { "true" } else { "false" },
        if adaptive_terminal_height { "true" } else { "false" }
    );
    fs::write(path, body)
}

fn save_ai_config(root: &Path, config: &AiConfig) -> io::Result<()> {
    let dir = root.join(".xone").join("ai");
    fs::create_dir_all(&dir)?;
    let path = dir.join("ai.conf");
    let body = format!(
        "provider={}\nbase_url={}\nmodel={}\napi_key={}\n",
        config.provider.trim(),
        config.base_url.trim(),
        config.model.trim(),
        config.api_key.trim()
    );
    fs::write(path, body)
}

fn extract_latest_fenced_code_block(input: &str) -> Option<String> {
    let mut blocks: Vec<String> = Vec::new();
    let mut in_code = false;
    let mut current = String::new();
    for line in input.lines() {
        if line.trim_start().starts_with("```") {
            if in_code {
                blocks.push(current.trim_end_matches('\n').to_string());
                current.clear();
                in_code = false;
            } else {
                in_code = true;
            }
            continue;
        }
        if in_code {
            current.push_str(line);
            current.push('\n');
        }
    }
    blocks
        .into_iter()
        .rev()
        .find(|block| !block.trim().is_empty())
}

fn spinner_frame_from_elapsed(elapsed: Duration) -> &'static str {
    const FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
    let index = ((elapsed.as_millis() / 120) % FRAMES.len() as u128) as usize;
    FRAMES[index]
}

fn is_editor_text_input_key(key: &KeyEvent) -> bool {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return false;
    }
    match key.code {
        KeyCode::Char(_) | KeyCode::Enter => true,
        KeyCode::Tab => !key.modifiers.contains(KeyModifiers::SHIFT),
        _ => false,
    }
}

fn rewrite_editor_paste(content: &str, language: language::Language) -> String {
    if content.is_empty() {
        return String::new();
    }

    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<String> = normalized
        .split('\n')
        .map(|line| {
            let (leading_width, rest) = split_leading_whitespace_columns(line);
            let standardized = if should_apply_xone_indent_standard(language) {
                standardize_indent_width(leading_width)
            } else {
                leading_width
            };
            let body = rest.trim_end_matches(char::is_whitespace);
            format!("{}{}", " ".repeat(standardized), body)
        })
        .collect();

    while lines.len() > 1 {
        let should_drop = lines.last().map(|line| line.is_empty()).unwrap_or(false);
        if !should_drop {
            break;
        }
        lines.pop();
    }

    lines.join("\n")
}

fn should_apply_xone_indent_standard(language: language::Language) -> bool {
    !matches!(
        language,
        language::Language::PlainText
            | language::Language::Markdown
            | language::Language::Python
            | language::Language::Yaml
    )
}

fn split_leading_whitespace_columns(line: &str) -> (usize, &str) {
    let mut columns = 0usize;
    let mut split_byte = 0usize;
    for (idx, ch) in line.char_indices() {
        if ch == ' ' {
            columns += 1;
            split_byte = idx + ch.len_utf8();
        } else if ch == '\t' {
            columns += INDENT_WIDTH;
            split_byte = idx + ch.len_utf8();
        } else if ch.is_whitespace() {
            columns += 1;
            split_byte = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    (columns, &line[split_byte..])
}

fn standardize_indent_width(width: usize) -> usize {
    if width == 0 {
        return 0;
    }
    width.div_ceil(INDENT_WIDTH) * INDENT_WIDTH
}

fn paste_fingerprint(payload: &str) -> (u64, usize) {
    let mut hasher = DefaultHasher::new();
    payload.hash(&mut hasher);
    (hasher.finish(), payload.len())
}

fn is_debounced_control_action(action: control::ControlAction) -> bool {
    matches!(
        action,
        control::ControlAction::EditorPaste
            | control::ControlAction::GlobalEditorPaste
            | control::ControlAction::EditorCopy
            | control::ControlAction::GlobalEditorCopy
            | control::ControlAction::EditorCut
            | control::ControlAction::GlobalEditorCut
            | control::ControlAction::EditorSelectAll
            | control::ControlAction::GlobalEditorSelectAll
            | control::ControlAction::AcceptSuggestion
            | control::ControlAction::NextSuggestion
    )
}

fn is_raw_control_fallback_event(key: &KeyEvent) -> bool {
    if !key.modifiers.is_empty() {
        return false;
    }
    let KeyCode::Char(ch) = key.code else {
        return false;
    };
    let value = ch as u32;
    (1..=26).contains(&value)
}
