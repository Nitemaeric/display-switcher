use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub settings: AppSettings,
    pub groups: Vec<DisplayGroup>,
    #[serde(default)]
    pub onboarding_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub launch_on_startup: bool,
    #[serde(default = "default_steam_path")]
    pub steam_path: String,
    #[serde(default = "default_true")]
    pub minimize_to_tray: bool,
    #[serde(default = "default_retention")]
    pub telemetry_retention: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayGroup {
    pub id: String,
    pub name: String,
    pub display_ids: Vec<String>,
    pub profile_file: String,
    #[serde(default)]
    pub hotkey: Option<String>,
    #[serde(default)]
    pub gamepad_chord: Option<GamepadChord>,
    pub post_action: PostAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamepadChord {
    pub buttons: Vec<String>,
    #[serde(default = "default_hold_ms")]
    pub hold_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PostAction {
    Builtin { action: String },
    LaunchProgram { path: String, args: Option<String> },
    RunCommand { command: String },
}

fn default_theme() -> String {
    "system".into()
}

fn default_steam_path() -> String {
    "auto".into()
}

fn default_true() -> bool {
    true
}

fn default_retention() -> usize {
    500
}

fn default_hold_ms() -> u64 {
    400
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            settings: AppSettings {
                theme: default_theme(),
                launch_on_startup: false,
                steam_path: default_steam_path(),
                minimize_to_tray: true,
                telemetry_retention: default_retention(),
            },
            groups: vec![],
            onboarding_complete: false,
        }
    }
}

/// A group can only be activated when it has at least one assigned display.
pub fn is_group_activatable(group: &DisplayGroup) -> bool {
    !group.display_ids.is_empty()
}

pub fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("display-switcher")
}

pub fn config_path() -> PathBuf {
    app_data_dir().join("config.json")
}

pub fn profiles_dir() -> PathBuf {
    app_data_dir().join("profiles")
}

pub fn resolve_profile_path(relative: &str) -> PathBuf {
    if relative.contains('/') || relative.contains('\\') {
        app_data_dir().join(relative)
    } else {
        profiles_dir().join(relative)
    }
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        let config = AppConfig::default();
        let _ = save_config(&config);
        return config;
    }

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let dir = app_data_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(profiles_dir()).map_err(|e| e.to_string())?;
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(config_path(), content).map_err(|e| e.to_string())
}

pub fn new_group_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn default_profile_filename(group_id: &str) -> String {
    format!("{group_id}.json")
}