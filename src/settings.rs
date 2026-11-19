use std::collections::HashMap;
use std::path::PathBuf;

use gtk::glib;

pub struct Shortcut {
    pub id: &'static str,
    pub action: &'static str,
    pub title: &'static str,
    pub default: &'static str,
}

pub const SHORTCUTS: &[Shortcut] = &[
    Shortcut { id: "new", action: "win.new", title: "New document", default: "<Control>n" },
    Shortcut { id: "open", action: "win.open", title: "Open", default: "<Control>o" },
    Shortcut { id: "save", action: "win.save", title: "Save", default: "<Control>s" },
    Shortcut { id: "save-as", action: "win.save-as", title: "Save as", default: "<Shift><Control>s" },
    Shortcut { id: "close", action: "win.close-tab", title: "Close tab", default: "<Control>w" },
    Shortcut { id: "mode-editor", action: "win.mode::editor", title: "Editor view", default: "<Control>1" },
    Shortcut { id: "mode-split", action: "win.mode::split", title: "Split view", default: "<Control>2" },
    Shortcut { id: "mode-preview", action: "win.mode::preview", title: "Preview view", default: "<Control>3" },
    Shortcut { id: "export-pdf", action: "win.export-pdf", title: "Export as PDF", default: "<Control>e" },
    Shortcut { id: "export-png", action: "win.export-png", title: "Export as PNG", default: "<Shift><Control>e" },
    Shortcut { id: "settings", action: "win.settings", title: "Settings", default: "<Control>comma" },
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::System, Theme::Light, Theme::Dark];

    pub fn name(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Settings {
    pub theme: Theme,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
    accels: HashMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        let accels = SHORTCUTS.iter().map(|s| (s.id.to_string(), s.default.to_string())).collect();
        Settings { theme: Theme::System, width: 1200, height: 800, maximized: false, accels }
    }
}

impl Settings {
    pub fn parse(data: &str) -> Self {
        let mut settings = Settings::default();
        let file = glib::KeyFile::new();
        if file.load_from_data(data, glib::KeyFileFlags::NONE).is_err() {
            return settings;
        }
        if let Ok(name) = file.string("general", "theme") {
            settings.theme = Theme::ALL.into_iter().find(|t| t.name() == name).unwrap_or(Theme::System);
        }
        settings.width = file.integer("window", "width").unwrap_or(settings.width);
        settings.height = file.integer("window", "height").unwrap_or(settings.height);
        settings.maximized = file.boolean("window", "maximized").unwrap_or(settings.maximized);
        for shortcut in SHORTCUTS {
            if let Ok(accel) = file.string("shortcuts", shortcut.id) {
                settings.set_accel(shortcut.id, &accel);
            }
        }
        settings
    }

    pub fn serialize(&self) -> String {
        let file = glib::KeyFile::new();
        file.set_string("general", "theme", self.theme.name());
        file.set_integer("window", "width", self.width);
        file.set_integer("window", "height", self.height);
        file.set_boolean("window", "maximized", self.maximized);
        for shortcut in SHORTCUTS {
            file.set_string("shortcuts", shortcut.id, self.accel(shortcut.id));
        }
        file.to_data().to_string()
    }

    pub fn load() -> Self {
        std::fs::read_to_string(path()).map(|data| Settings::parse(&data)).unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.serialize())
    }

    pub fn accel(&self, id: &str) -> &str {
        self.accels.get(id).map(String::as_str).unwrap_or_default()
    }

    pub fn set_accel(&mut self, id: &str, accel: &str) {
        self.accels.insert(id.to_string(), accel.to_string());
    }

    pub fn reset_accels(&mut self) {
        self.accels = Settings::default().accels;
    }

    pub fn conflict(&self, id: &str, accel: &str) -> Option<&'static Shortcut> {
        let wanted = normalize(accel);
        if wanted.is_empty() {
            return None;
        }
        SHORTCUTS.iter().find(|s| s.id != id && normalize(self.accel(s.id)) == wanted)
    }
}

fn path() -> PathBuf {
    glib::user_config_dir().join("downmark").join("settings.ini")
}

pub fn normalize(accel: &str) -> String {
    let mut mods = Vec::new();
    let mut rest = accel;
    while let Some((modifier, tail)) = rest.strip_prefix('<').and_then(|r| r.split_once('>')) {
        mods.push(match modifier.to_ascii_lowercase().as_str() {
            "control" | "ctrl" | "primary" => "ctrl".to_string(),
            other => other.to_string(),
        });
        rest = tail;
    }
    if rest.is_empty() {
        return String::new();
    }
    mods.sort();
    mods.push(rest.to_ascii_lowercase());
    mods.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_use_system_theme_and_default_shortcuts() {
        let settings = Settings::default();
        assert_eq!(settings.theme, Theme::System);
        assert_eq!(settings.accel("save"), "<Control>s");
        assert_eq!(settings.accel("mode-split"), "<Control>2");
    }

    #[test]
    fn every_shortcut_has_a_unique_id_and_default() {
        let mut seen: Vec<String> = Vec::new();
        for s in SHORTCUTS {
            assert!(!seen.contains(&normalize(s.default)), "duplicate default {}", s.default);
            seen.push(normalize(s.default));
        }
        assert!(SHORTCUTS.iter().any(|s| s.id == "export-pdf"));
        assert!(SHORTCUTS.iter().any(|s| s.id == "export-png"));
    }

    #[test]
    fn new_and_close_work_on_tabs() {
        let action = |id: &str| SHORTCUTS.iter().find(|s| s.id == id).map(|s| s.action);
        assert_eq!(action("new"), Some("win.new"));
        assert_eq!(action("close"), Some("win.close-tab"));
    }

    #[test]
    fn garbage_falls_back_to_defaults() {
        assert_eq!(Settings::parse("not [ a key file"), Settings::default());
    }

    #[test]
    fn unknown_theme_falls_back_to_system() {
        assert_eq!(Settings::parse("[general]\ntheme=purple\n").theme, Theme::System);
    }

    #[test]
    fn round_trip_keeps_theme_and_shortcuts() {
        let mut settings = Settings { theme: Theme::Dark, ..Default::default() };
        settings.set_accel("save", "<Control><Alt>s");
        settings.set_accel("open", "");
        let loaded = Settings::parse(&settings.serialize());
        assert_eq!(loaded, settings);
        assert_eq!(loaded.accel("open"), "");
    }

    #[test]
    fn window_size_and_maximized_survive_a_round_trip() {
        let settings = Settings { width: 900, height: 650, maximized: true, ..Default::default() };
        let loaded = Settings::parse(&settings.serialize());
        assert_eq!((loaded.width, loaded.height, loaded.maximized), (900, 650, true));
    }

    #[test]
    fn window_defaults_to_1200_by_800_unmaximized() {
        let settings = Settings::parse("[general]\ntheme=light\n");
        assert_eq!((settings.width, settings.height, settings.maximized), (1200, 800, false));
    }

    #[test]
    fn missing_shortcut_keeps_default() {
        let settings = Settings::parse("[general]\ntheme=light\n");
        assert_eq!(settings.theme, Theme::Light);
        assert_eq!(settings.accel("save"), "<Control>s");
    }

    #[test]
    fn conflict_reports_other_action_using_same_accel() {
        let settings = Settings::default();
        assert_eq!(settings.conflict("open", "<Ctrl>S").map(|s| s.id), Some("save"));
    }

    #[test]
    fn no_conflict_with_itself_or_free_accel() {
        let settings = Settings::default();
        assert!(settings.conflict("save", "<Control>s").is_none());
        assert!(settings.conflict("save", "<Control><Alt>F9").is_none());
        assert!(settings.conflict("save", "").is_none());
    }

    #[test]
    fn normalize_ignores_modifier_order_and_aliases() {
        assert_eq!(normalize("<Shift><Control>s"), normalize("<ctrl><shift>S"));
        assert_eq!(normalize("<Primary>o"), normalize("<Control>o"));
        assert_ne!(normalize("<Control>o"), normalize("<Alt>o"));
    }
}
