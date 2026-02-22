//! Module: src/app/style.rs
//! Catatan: file ini bagian dari mesin Xone; ubah logika dengan hati-hati, kopi tetap pahit.

use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppearancePreset {
    Classic,
    Compact,
    HighContrast,
    Minimal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccentPreset {
    Blue,
    Emerald,
    Amber,
    Mono,
}

impl AccentPreset {
    // Serialisasi ke config: biar preset bisa balik lagi setelah app dibuka ulang.
    pub fn as_str(self) -> &'static str {
        match self {
            AccentPreset::Blue => "blue",
            AccentPreset::Emerald => "emerald",
            AccentPreset::Amber => "amber",
            AccentPreset::Mono => "mono",
        }
    }

    // Label ramah manusia untuk ditampilkan di Settings.
    pub fn label(self) -> &'static str {
        match self {
            AccentPreset::Blue => "Blue",
            AccentPreset::Emerald => "Emerald",
            AccentPreset::Amber => "Amber",
            AccentPreset::Mono => "Mono",
        }
    }

    // Parser toleran: beberapa alias tetap diterima biar user gak perlu hafal mantra.
    pub fn from_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "blue" => Some(AccentPreset::Blue),
            "emerald" | "green" => Some(AccentPreset::Emerald),
            "amber" | "orange" => Some(AccentPreset::Amber),
            "mono" | "gray" | "grey" => Some(AccentPreset::Mono),
            _ => None,
        }
    }

    // Putar preset ke mode berikutnya (kayak muter playlist, tapi buat aksen warna).
    pub fn next(self) -> Self {
        match self {
            AccentPreset::Blue => AccentPreset::Emerald,
            AccentPreset::Emerald => AccentPreset::Amber,
            AccentPreset::Amber => AccentPreset::Mono,
            AccentPreset::Mono => AccentPreset::Blue,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiDensity {
    Comfortable,
    Compact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabThemePreset {
    Soft,
    Balanced,
    Vivid,
}

impl TabThemePreset {
    pub fn as_str(self) -> &'static str {
        match self {
            TabThemePreset::Soft => "soft",
            TabThemePreset::Balanced => "balanced",
            TabThemePreset::Vivid => "vivid",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TabThemePreset::Soft => "Soft",
            TabThemePreset::Balanced => "Balanced",
            TabThemePreset::Vivid => "Vivid",
        }
    }

    pub fn from_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "soft" => Some(TabThemePreset::Soft),
            "balanced" => Some(TabThemePreset::Balanced),
            "vivid" => Some(TabThemePreset::Vivid),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        match self {
            TabThemePreset::Soft => TabThemePreset::Balanced,
            TabThemePreset::Balanced => TabThemePreset::Vivid,
            TabThemePreset::Vivid => TabThemePreset::Soft,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntaxThemePreset {
    Soft,
    Neon,
    Mono,
    Aurora,
}

impl SyntaxThemePreset {
    pub fn as_str(self) -> &'static str {
        match self {
            SyntaxThemePreset::Soft => "soft",
            SyntaxThemePreset::Neon => "neon",
            SyntaxThemePreset::Mono => "mono",
            SyntaxThemePreset::Aurora => "aurora",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SyntaxThemePreset::Soft => "Soft",
            SyntaxThemePreset::Neon => "Neon",
            SyntaxThemePreset::Mono => "Mono",
            SyntaxThemePreset::Aurora => "Aurora",
        }
    }

    pub fn from_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "soft" => Some(SyntaxThemePreset::Soft),
            "neon" => Some(SyntaxThemePreset::Neon),
            "mono" | "gray" | "grey" => Some(SyntaxThemePreset::Mono),
            "aurora" | "vivid" => Some(SyntaxThemePreset::Aurora),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        match self {
            SyntaxThemePreset::Soft => SyntaxThemePreset::Neon,
            SyntaxThemePreset::Neon => SyntaxThemePreset::Mono,
            SyntaxThemePreset::Mono => SyntaxThemePreset::Aurora,
            SyntaxThemePreset::Aurora => SyntaxThemePreset::Soft,
        }
    }
}

impl UiDensity {
    // Simpan mode density ke string config.
    pub fn as_str(self) -> &'static str {
        match self {
            UiDensity::Comfortable => "comfortable",
            UiDensity::Compact => "compact",
        }
    }

    // Nama cantik untuk UI Settings.
    pub fn label(self) -> &'static str {
        match self {
            UiDensity::Comfortable => "Comfortable",
            UiDensity::Compact => "Compact",
        }
    }

    // Baca nilai density dari config user.
    pub fn from_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "comfortable" => Some(UiDensity::Comfortable),
            "compact" => Some(UiDensity::Compact),
            _ => None,
        }
    }

    // Toggle density berikutnya.
    pub fn next(self) -> Self {
        match self {
            UiDensity::Comfortable => UiDensity::Compact,
            UiDensity::Compact => UiDensity::Comfortable,
        }
    }
}

impl AppearancePreset {
    // Kunci serialisasi preset appearance.
    pub fn as_str(self) -> &'static str {
        match self {
            AppearancePreset::Classic => "classic",
            AppearancePreset::Compact => "compact",
            AppearancePreset::HighContrast => "high-contrast",
            AppearancePreset::Minimal => "minimal",
        }
    }

    // Label preset untuk manusia, bukan untuk compiler doang.
    pub fn label(self) -> &'static str {
        match self {
            AppearancePreset::Classic => "Classic",
            AppearancePreset::Compact => "Compact",
            AppearancePreset::HighContrast => "High Contrast",
            AppearancePreset::Minimal => "Minimal",
        }
    }

    // Parse preset appearance dari file config.
    pub fn from_str(raw: &str) -> Option<Self> {
        let value = raw.trim().to_ascii_lowercase();
        match value.as_str() {
            "classic" => Some(AppearancePreset::Classic),
            "compact" => Some(AppearancePreset::Compact),
            "high-contrast" | "high_contrast" | "highcontrast" => {
                Some(AppearancePreset::HighContrast)
            }
            "minimal" => Some(AppearancePreset::Minimal),
            _ => None,
        }
    }

    // Siklus preset biar ganti tema tinggal sekali klik.
    pub fn next(self) -> Self {
        match self {
            AppearancePreset::Classic => AppearancePreset::Compact,
            AppearancePreset::Compact => AppearancePreset::HighContrast,
            AppearancePreset::HighContrast => AppearancePreset::Minimal,
            AppearancePreset::Minimal => AppearancePreset::Classic,
        }
    }
}

pub struct Palette {
    pub background: Color,
    pub surface: Color,
    pub surface_alt: Color,
    pub border: Color,
    pub border_active: Color,
    pub text: Color,
    pub text_muted: Color,
    pub accent_soft: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub selection: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            background: Color::Rgb(15, 19, 27),
            surface: Color::Rgb(22, 27, 38),
            surface_alt: Color::Rgb(28, 34, 47),
            border: Color::Rgb(82, 95, 119),
            border_active: Color::Rgb(108, 184, 255),
            text: Color::Rgb(230, 236, 246),
            text_muted: Color::Rgb(160, 175, 198),
            accent_soft: Color::Rgb(35, 49, 66),
            accent: Color::Rgb(122, 202, 255),
            success: Color::Rgb(114, 210, 160),
            warning: Color::Rgb(255, 177, 110),
            selection: Color::Rgb(54, 70, 94),
        }
    }
}

pub struct Theme {
    palette: Palette,
    preset: AppearancePreset,
    accent: AccentPreset,
    tab_theme: TabThemePreset,
    syntax_theme: SyntaxThemePreset,
}

impl Theme {
    // Bangun theme dari kombinasi preset + accent.
    pub fn with_options(
        preset: AppearancePreset,
        accent: AccentPreset,
        tab_theme: TabThemePreset,
        syntax_theme: SyntaxThemePreset,
    ) -> Self {
        let mut theme = Self {
            palette: palette_for(preset),
            preset,
            accent,
            tab_theme,
            syntax_theme,
        };
        theme.apply_accent(accent);
        theme
    }

    // Ganti preset dasar lalu re-apply accent biar karakter tema tetap konsisten.
    pub fn set_preset(&mut self, preset: AppearancePreset) {
        self.preset = preset;
        self.palette = palette_for(preset);
        self.apply_accent(self.accent);
    }

    // Ganti aksen tanpa ngeganti DNA preset utamanya.
    pub fn set_accent(&mut self, accent: AccentPreset) {
        self.accent = accent;
        self.apply_accent(accent);
    }

    pub fn set_tab_theme(&mut self, tab_theme: TabThemePreset) {
        self.tab_theme = tab_theme;
    }

    pub fn set_syntax_theme(&mut self, syntax_theme: SyntaxThemePreset) {
        self.syntax_theme = syntax_theme;
    }

    // Ini dapur warna aksen: border aktif, soft background, dan warna brand diracik di sini.
    fn apply_accent(&mut self, accent: AccentPreset) {
        let (accent_main, accent_soft, active_border) = match accent {
            AccentPreset::Blue => (
                Color::Rgb(122, 202, 255),
                Color::Rgb(35, 49, 66),
                Color::Rgb(108, 184, 255),
            ),
            AccentPreset::Emerald => (
                Color::Rgb(124, 224, 184),
                Color::Rgb(32, 58, 52),
                Color::Rgb(118, 206, 173),
            ),
            AccentPreset::Amber => (
                Color::Rgb(255, 199, 126),
                Color::Rgb(62, 50, 36),
                Color::Rgb(240, 186, 115),
            ),
            AccentPreset::Mono => (
                Color::Rgb(196, 204, 216),
                Color::Rgb(52, 56, 63),
                Color::Rgb(184, 193, 208),
            ),
        };
        self.palette.accent = accent_main;
        self.palette.accent_soft = accent_soft;
        self.palette.border_active = active_border;
    }

    // Style header: tegas, rapi, dan siap jadi papan nama.
    pub fn header(&self) -> Style {
        Style::default()
            .fg(self.palette.text)
            .bg(self.palette.surface)
            .add_modifier(Modifier::BOLD)
    }

    // Panel utama: background kerja sehari-hari.
    pub fn panel(&self) -> Style {
        Style::default()
            .fg(self.palette.text)
            .bg(self.palette.background)
    }

    // Panel alternatif untuk blok sekunder.
    pub fn panel_alt(&self) -> Style {
        Style::default()
            .fg(self.palette.text)
            .bg(self.palette.surface)
    }

    // Border netral untuk keadaan normal.
    pub fn border(&self) -> Style {
        Style::default().fg(self.palette.border)
    }

    // Border aktif: biar panel fokus kelihatan "gue yang pegang keyboard".
    pub fn border_active(&self) -> Style {
        Style::default()
            .fg(self.palette.border_active)
            .add_modifier(Modifier::BOLD)
    }

    // Teks utama (isi pembicaraan inti).
    pub fn text(&self) -> Style {
        Style::default().fg(self.palette.text)
    }

    // Status bar tone yang lebih kalem.
    pub fn status(&self) -> Style {
        Style::default()
            .fg(self.palette.text_muted)
            .bg(self.palette.surface)
    }

    // Command/help bar tone khusus bawah layar.
    pub fn command(&self) -> Style {
        Style::default()
            .fg(self.palette.text_muted)
            .bg(self.palette.surface_alt)
    }

    // Tab aktif: disorot biar gak salah tab pas buru-buru.
    pub fn tab_active(&self) -> Style {
        match self.tab_theme {
            TabThemePreset::Soft => Style::default()
                .fg(self.palette.text)
                .bg(self.palette.surface_alt)
                .add_modifier(Modifier::BOLD),
            TabThemePreset::Balanced => Style::default()
                .fg(self.palette.text)
                .bg(self.palette.accent_soft)
                .add_modifier(Modifier::BOLD),
            TabThemePreset::Vivid => Style::default()
                .fg(self.palette.background)
                .bg(self.palette.accent)
                .add_modifier(Modifier::BOLD),
        }
    }

    // Tab nonaktif: tetap terbaca tapi gak teriak.
    pub fn tab_inactive(&self) -> Style {
        match self.tab_theme {
            TabThemePreset::Soft => Style::default()
                .fg(self.palette.text_muted)
                .bg(self.palette.surface),
            TabThemePreset::Balanced => Style::default()
                .fg(self.palette.text_muted)
                .bg(self.palette.background),
            TabThemePreset::Vivid => Style::default()
                .fg(self.palette.text)
                .bg(self.palette.surface_alt),
        }
    }

    // Separator kecil buat napas antar informasi.
    pub fn separator(&self) -> Style {
        Style::default().fg(self.palette.border)
    }

    // Selection style: area yang lagi dipilih user.
    pub fn selection(&self) -> Style {
        Style::default()
            .bg(self.palette.selection)
            .add_modifier(Modifier::BOLD)
    }

    // Teks muted untuk info tambahan.
    pub fn muted(&self) -> Style {
        Style::default().fg(self.palette.text_muted)
    }

    // Warna aksen utama.
    pub fn accent(&self) -> Style {
        Style::default().fg(self.palette.accent)
    }

    // State positif: operasi beres.
    pub fn success(&self) -> Style {
        Style::default().fg(self.palette.success)
    }

    // State warning/error ringan.
    pub fn warning(&self) -> Style {
        Style::default().fg(self.palette.warning)
    }

    // Gutter editor (nomor baris dan teman-temannya).
    pub fn gutter(&self) -> Style {
        Style::default().fg(self.palette.text_muted)
    }

    // Branding app di header.
    pub fn brand(&self) -> Style {
        Style::default()
            .fg(self.palette.accent)
            .add_modifier(Modifier::BOLD)
    }

    // Kelompok style syntax highlight: keyword, type, string, dll.
    pub fn syntax_keyword(&self) -> Style {
        match self.syntax_theme {
            SyntaxThemePreset::Soft => Style::default()
                .fg(Color::Rgb(255, 140, 162))
                .add_modifier(Modifier::BOLD),
            SyntaxThemePreset::Neon => Style::default()
                .fg(Color::Rgb(255, 112, 147))
                .add_modifier(Modifier::BOLD),
            SyntaxThemePreset::Mono => Style::default()
                .fg(Color::Rgb(214, 214, 214))
                .add_modifier(Modifier::BOLD),
            SyntaxThemePreset::Aurora => Style::default()
                .fg(Color::Rgb(255, 122, 198))
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn syntax_type(&self) -> Style {
        match self.syntax_theme {
            SyntaxThemePreset::Soft => Style::default().fg(Color::Rgb(128, 211, 201)),
            SyntaxThemePreset::Neon => Style::default().fg(Color::Rgb(102, 224, 210)),
            SyntaxThemePreset::Mono => Style::default().fg(Color::Rgb(195, 195, 195)),
            SyntaxThemePreset::Aurora => Style::default()
                .fg(Color::Rgb(122, 214, 255))
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn syntax_string(&self) -> Style {
        match self.syntax_theme {
            SyntaxThemePreset::Soft => Style::default().fg(Color::Rgb(239, 207, 136)),
            SyntaxThemePreset::Neon => Style::default().fg(Color::Rgb(255, 214, 120)),
            SyntaxThemePreset::Mono => Style::default().fg(Color::Rgb(186, 186, 186)),
            SyntaxThemePreset::Aurora => Style::default().fg(Color::Rgb(255, 214, 143)),
        }
    }

    pub fn syntax_number(&self) -> Style {
        match self.syntax_theme {
            SyntaxThemePreset::Soft => Style::default()
                .fg(Color::Rgb(136, 195, 255))
                .add_modifier(Modifier::BOLD),
            SyntaxThemePreset::Neon => Style::default()
                .fg(Color::Rgb(98, 196, 255))
                .add_modifier(Modifier::BOLD),
            SyntaxThemePreset::Mono => Style::default().fg(Color::Rgb(176, 176, 176)),
            SyntaxThemePreset::Aurora => Style::default()
                .fg(Color::Rgb(138, 174, 255))
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn syntax_comment(&self) -> Style {
        match self.syntax_theme {
            SyntaxThemePreset::Soft => Style::default()
                .fg(Color::Rgb(120, 131, 150))
                .add_modifier(Modifier::ITALIC),
            SyntaxThemePreset::Neon => Style::default()
                .fg(Color::Rgb(129, 141, 161))
                .add_modifier(Modifier::ITALIC),
            SyntaxThemePreset::Mono => Style::default()
                .fg(Color::Rgb(134, 134, 134))
                .add_modifier(Modifier::ITALIC),
            SyntaxThemePreset::Aurora => Style::default()
                .fg(Color::Rgb(134, 145, 172))
                .add_modifier(Modifier::ITALIC),
        }
    }

    pub fn syntax_tag(&self) -> Style {
        match self.syntax_theme {
            SyntaxThemePreset::Soft => Style::default().fg(Color::Rgb(182, 163, 255)),
            SyntaxThemePreset::Neon => Style::default().fg(Color::Rgb(193, 154, 255)),
            SyntaxThemePreset::Mono => Style::default().fg(Color::Rgb(204, 204, 204)),
            SyntaxThemePreset::Aurora => Style::default()
                .fg(Color::Rgb(201, 152, 255))
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn syntax_attribute(&self) -> Style {
        match self.syntax_theme {
            SyntaxThemePreset::Soft => Style::default().fg(Color::Rgb(255, 183, 132)),
            SyntaxThemePreset::Neon => Style::default().fg(Color::Rgb(255, 168, 112)),
            SyntaxThemePreset::Mono => Style::default().fg(Color::Rgb(189, 189, 189)),
            SyntaxThemePreset::Aurora => Style::default().fg(Color::Rgb(255, 172, 121)),
        }
    }

    pub fn syntax_value(&self) -> Style {
        match self.syntax_theme {
            SyntaxThemePreset::Soft => Style::default().fg(Color::Rgb(125, 218, 186)),
            SyntaxThemePreset::Neon => Style::default().fg(Color::Rgb(106, 228, 189)),
            SyntaxThemePreset::Mono => Style::default().fg(Color::Rgb(198, 198, 198)),
            SyntaxThemePreset::Aurora => Style::default()
                .fg(Color::Rgb(123, 230, 196))
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn syntax_operator(&self) -> Style {
        match self.syntax_theme {
            SyntaxThemePreset::Soft => Style::default().fg(Color::Rgb(182, 197, 222)),
            SyntaxThemePreset::Neon => Style::default().fg(Color::Rgb(170, 199, 235)),
            SyntaxThemePreset::Mono => Style::default().fg(Color::Rgb(170, 170, 170)),
            SyntaxThemePreset::Aurora => Style::default()
                .fg(Color::Rgb(176, 196, 236))
                .add_modifier(Modifier::UNDERLINED),
        }
    }

    // Pewarnaan indent bertingkat supaya struktur kode kelihatan tanpa mata juling.
    pub fn indent_level(&self, level: usize) -> Style {
        let bg = match level % 6 {
            0 => Color::Rgb(26, 33, 45),
            1 => Color::Rgb(28, 35, 47),
            2 => Color::Rgb(30, 37, 49),
            3 => Color::Rgb(32, 39, 52),
            4 => Color::Rgb(34, 41, 54),
            _ => Color::Rgb(36, 43, 56),
        };
        Style::default().fg(self.palette.text_muted).bg(bg)
    }
}

// Generator palette berdasarkan preset dasar sebelum aksen disuntikkan.
fn palette_for(preset: AppearancePreset) -> Palette {
    match preset {
        AppearancePreset::Classic => Palette {
            background: Color::Rgb(15, 19, 27),
            surface: Color::Rgb(22, 27, 38),
            surface_alt: Color::Rgb(28, 34, 47),
            border: Color::Rgb(82, 95, 119),
            border_active: Color::Rgb(108, 184, 255),
            text: Color::Rgb(230, 236, 246),
            text_muted: Color::Rgb(160, 175, 198),
            accent_soft: Color::Rgb(35, 49, 66),
            accent: Color::Rgb(122, 202, 255),
            success: Color::Rgb(114, 210, 160),
            warning: Color::Rgb(255, 177, 110),
            selection: Color::Rgb(54, 70, 94),
        },
        AppearancePreset::Compact => Palette {
            background: Color::Rgb(17, 20, 27),
            surface: Color::Rgb(23, 28, 37),
            surface_alt: Color::Rgb(30, 36, 47),
            border: Color::Rgb(88, 101, 126),
            border_active: Color::Rgb(130, 200, 255),
            text: Color::Rgb(230, 236, 245),
            text_muted: Color::Rgb(166, 179, 199),
            accent_soft: Color::Rgb(38, 56, 74),
            accent: Color::Rgb(135, 207, 255),
            success: Color::Rgb(122, 214, 168),
            warning: Color::Rgb(255, 188, 122),
            selection: Color::Rgb(58, 73, 98),
        },
        AppearancePreset::HighContrast => Palette {
            background: Color::Rgb(8, 10, 14),
            surface: Color::Rgb(16, 20, 28),
            surface_alt: Color::Rgb(24, 30, 42),
            border: Color::Rgb(140, 154, 178),
            border_active: Color::Rgb(150, 220, 255),
            text: Color::Rgb(246, 250, 255),
            text_muted: Color::Rgb(186, 200, 222),
            accent_soft: Color::Rgb(44, 63, 84),
            accent: Color::Rgb(154, 225, 255),
            success: Color::Rgb(140, 228, 181),
            warning: Color::Rgb(255, 208, 134),
            selection: Color::Rgb(68, 86, 116),
        },
        AppearancePreset::Minimal => Palette {
            background: Color::Rgb(20, 22, 25),
            surface: Color::Rgb(26, 29, 33),
            surface_alt: Color::Rgb(31, 35, 40),
            border: Color::Rgb(90, 97, 108),
            border_active: Color::Rgb(180, 193, 216),
            text: Color::Rgb(226, 230, 236),
            text_muted: Color::Rgb(167, 173, 184),
            accent_soft: Color::Rgb(41, 45, 51),
            accent: Color::Rgb(190, 200, 219),
            success: Color::Rgb(148, 198, 160),
            warning: Color::Rgb(221, 183, 137),
            selection: Color::Rgb(62, 68, 77),
        },
    }
}
