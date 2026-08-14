use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// All user-configurable options for the AcreetionOS AppStore.
/// Persisted as TOML at `~/.config/acreetion-appstore/settings.toml`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // ─── General ───────────────────────────────────────────────────────────
    pub check_updates_on_launch: bool,
    pub remember_window_size: bool,
    pub window_width: i32,
    pub window_height: i32,
    pub default_category: String,
    pub show_notifications: bool,

    // ─── AUR (optional, toggled from the sidebar) ──────────────────────────
    pub aur_enabled: bool,
    pub aur_helper: AurHelper,
    pub aur_build_dir: String,
    pub aur_clean_build: bool,
    pub aur_include_devel: bool,
    pub aur_sort_by_votes: bool,
    pub aur_check_updates: bool,
    pub aur_refresh_on_sync: bool,
    pub aur_skip_review: bool,

    // ─── Backends ──────────────────────────────────────────────────────────
    pub pacman_enabled: bool,
    pub flatpak_enabled: bool,
    pub snap_enabled: bool,
    pub appimage_enabled: bool,
    pub flatpak_auto_install: bool,
    pub snap_auto_install: bool,

    // ─── Updates ───────────────────────────────────────────────────────────
    pub update_check_frequency: UpdateFrequency,
    pub refresh_databases_before_update: bool,
    pub remove_orphans_on_uninstall: bool,
    pub download_updates_in_parallel: bool,

    // ─── Appearance ────────────────────────────────────────────────────────
    pub theme: Theme,
    pub list_icon_size: i32,
    pub show_source_badges: bool,
    pub show_descriptions: bool,
    pub compact_rows: bool,
    pub use_grid_view: bool,

    // ─── Behaviour ─────────────────────────────────────────────────────────
    pub confirm_before_install: bool,
    pub confirm_before_remove: bool,
    pub show_output_expander: bool,
    pub command_timeout: u64, // 0 = use per-operation defaults

    // ─── AppImage ──────────────────────────────────────────────────────────
    pub appimage_dirs: Vec<String>,
    pub appimage_register_appimaged: bool,
    pub appimage_scan_on_launch: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AurHelper {
    Auto,
    Yay,
    Paru,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateFrequency {
    Never,
    EveryHour,
    Every6Hours,
    Daily,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Settings {
    pub fn config_path() -> PathBuf {
        let dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".config"))
                    .unwrap_or_else(|_| PathBuf::from("/tmp"))
            });
        dir.join("acreetion-appstore").join("settings.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(raw) => match toml::from_str::<Settings>(&raw) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("acreetion-appstore: failed to parse {}: {}", path.display(), e);
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match toml::to_string_pretty(self) {
            Ok(raw) => {
                if let Err(e) = std::fs::write(&path, raw) {
                    eprintln!("acreetion-appstore: failed to write {}: {}", path.display(), e);
                }
            }
            Err(e) => eprintln!("acreetion-appstore: failed to serialize settings: {}", e),
        }
    }

    /// The AUR helper that should actually be used, given what's installed.
    pub fn resolved_aur_helper(&self, yay_available: bool, paru_available: bool) -> Option<AurHelper> {
        if !self.aur_enabled {
            return None;
        }
        match self.aur_helper {
            AurHelper::Yay => if yay_available { Some(AurHelper::Yay) } else { None },
            AurHelper::Paru => if paru_available { Some(AurHelper::Paru) } else { None },
            AurHelper::Auto => {
                if paru_available {
                    Some(AurHelper::Paru)
                } else if yay_available {
                    Some(AurHelper::Yay)
                } else {
                    None
                }
            }
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // General
            check_updates_on_launch: true,
            remember_window_size: true,
            window_width: 1024,
            window_height: 768,
            default_category: "installed".to_string(),
            show_notifications: true,

            // AUR — OFF until the user ticks the sidebar box
            aur_enabled: false,
            aur_helper: AurHelper::Auto,
            aur_build_dir: String::new(), // empty = use helper default (~/.cache/yay, ~/.cache/paru)
            aur_clean_build: false,
            aur_include_devel: true,
            aur_sort_by_votes: true,
            aur_check_updates: true,
            aur_refresh_on_sync: true,
            aur_skip_review: false,

            // Backends
            pacman_enabled: true,
            flatpak_enabled: true,
            snap_enabled: true,
            appimage_enabled: true,
            flatpak_auto_install: true,
            snap_auto_install: false,

            // Updates
            update_check_frequency: UpdateFrequency::Never,
            refresh_databases_before_update: true,
            remove_orphans_on_uninstall: false,
            download_updates_in_parallel: false,

            // Appearance
            theme: Theme::System,
            list_icon_size: 24,
            show_source_badges: true,
            show_descriptions: true,
            compact_rows: false,
            use_grid_view: false,

            // Behaviour
            confirm_before_install: true,
            confirm_before_remove: true,
            show_output_expander: true,
            command_timeout: 0,

            // AppImage
            appimage_dirs: vec![
                "~/Applications".to_string(),
                "~/.local/bin".to_string(),
            ],
            appimage_register_appimaged: true,
            appimage_scan_on_launch: true,
        }
    }
}
