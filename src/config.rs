//! Настройки, которые переживают перезапуск: тема, режим отображения,
//! сортировка, размер окна и последний открытый каталог.

use std::path::PathBuf;

use iced::Theme;
use serde::{Deserialize, Serialize};

use crate::fsops::places;
use crate::fsops::SortKey;

/// Темы, доступные в переключателе. Основной набор — Catppuccin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    CatppuccinMocha,
    CatppuccinMacchiato,
    CatppuccinFrappe,
    CatppuccinLatte,
    Dark,
    Light,
    Nord,
    Dracula,
    GruvboxDark,
    SolarizedDark,
    TokyoNightStorm,
    KanagawaWave,
    Oxocarbon,
    Ferra,
}

impl ThemeChoice {
    pub const ALL: [ThemeChoice; 14] = [
        ThemeChoice::CatppuccinMocha,
        ThemeChoice::CatppuccinMacchiato,
        ThemeChoice::CatppuccinFrappe,
        ThemeChoice::CatppuccinLatte,
        ThemeChoice::Dark,
        ThemeChoice::Light,
        ThemeChoice::Nord,
        ThemeChoice::Dracula,
        ThemeChoice::GruvboxDark,
        ThemeChoice::SolarizedDark,
        ThemeChoice::TokyoNightStorm,
        ThemeChoice::KanagawaWave,
        ThemeChoice::Oxocarbon,
        ThemeChoice::Ferra,
    ];

    pub fn theme(self) -> Theme {
        match self {
            ThemeChoice::CatppuccinMocha => Theme::CatppuccinMocha,
            ThemeChoice::CatppuccinMacchiato => Theme::CatppuccinMacchiato,
            ThemeChoice::CatppuccinFrappe => Theme::CatppuccinFrappe,
            ThemeChoice::CatppuccinLatte => Theme::CatppuccinLatte,
            ThemeChoice::Dark => Theme::Dark,
            ThemeChoice::Light => Theme::Light,
            ThemeChoice::Nord => Theme::Nord,
            ThemeChoice::Dracula => Theme::Dracula,
            ThemeChoice::GruvboxDark => Theme::GruvboxDark,
            ThemeChoice::SolarizedDark => Theme::SolarizedDark,
            ThemeChoice::TokyoNightStorm => Theme::TokyoNightStorm,
            ThemeChoice::KanagawaWave => Theme::KanagawaWave,
            ThemeChoice::Oxocarbon => Theme::Oxocarbon,
            ThemeChoice::Ferra => Theme::Ferra,
        }
    }
}

impl std::fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ThemeChoice::CatppuccinMocha => "Catppuccin Mocha",
            ThemeChoice::CatppuccinMacchiato => "Catppuccin Macchiato",
            ThemeChoice::CatppuccinFrappe => "Catppuccin Frappé",
            ThemeChoice::CatppuccinLatte => "Catppuccin Latte",
            ThemeChoice::Dark => "Тёмная",
            ThemeChoice::Light => "Светлая",
            ThemeChoice::Nord => "Nord",
            ThemeChoice::Dracula => "Dracula",
            ThemeChoice::GruvboxDark => "Gruvbox Dark",
            ThemeChoice::SolarizedDark => "Solarized Dark",
            ThemeChoice::TokyoNightStorm => "Tokyo Night Storm",
            ThemeChoice::KanagawaWave => "Kanagawa Wave",
            ThemeChoice::Oxocarbon => "Oxocarbon",
            ThemeChoice::Ferra => "Ferra",
        };
        f.write_str(name)
    }
}

/// Режим отображения основной панели.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    /// Таблица с колонками: имя, размер, тип, дата.
    Details,
    /// Плотные колонки, только имена.
    Compact,
    /// Крупные значки сеткой.
    Icons,
}

impl ViewMode {
    pub const ALL: [ViewMode; 3] = [ViewMode::Details, ViewMode::Compact, ViewMode::Icons];

    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Details => "Подробно",
            ViewMode::Compact => "Компактно",
            ViewMode::Icons => "Значки",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeChoice,
    pub view: ViewMode,
    pub show_hidden: bool,
    pub sort_key: SortKey,
    pub sort_ascending: bool,
    pub dirs_first: bool,
    pub sidebar_width: f32,
    pub icon_size: u16,
    pub window_size: (f32, f32),
    /// Пользовательские закладки для панели быстрого доступа.
    pub bookmarks: Vec<PathBuf>,
    pub last_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::CatppuccinMocha,
            view: ViewMode::Details,
            show_hidden: false,
            sort_key: SortKey::Name,
            sort_ascending: true,
            dirs_first: true,
            sidebar_width: 220.0,
            icon_size: 72,
            window_size: (1180.0, 720.0),
            bookmarks: Vec::new(),
            last_dir: None,
        }
    }
}

impl Config {
    fn path() -> PathBuf {
        places::config_dir().join("file-man/config.json")
    }

    pub fn load() -> Self {
        let Ok(text) = std::fs::read_to_string(Self::path()) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    /// Сохранение «как получится»: сбой настроек не должен ломать работу.
    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }
}
