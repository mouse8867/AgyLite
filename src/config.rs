use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum AfterLaunch {
    /// Hide the window into the system tray (alpha.2: real tray icon).
    Minimise,
    /// Close AgyLite after a successful launch.
    Close,
    /// Keep the window open.
    Keep,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config {
    pub agy_path: Option<String>,
    pub proxy_host: String,
    pub proxy_port: Option<u16>,
    pub kill_residual: bool,
    pub autostart: bool,
    pub after_launch: AfterLaunch,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agy_path: None,
            proxy_host: "127.0.0.1".to_string(),
            proxy_port: None,
            kill_residual: true,
            autostart: false,
            after_launch: AfterLaunch::Minimise,
        }
    }
}

/// Portable layout: config.json lives next to the executable.
pub fn config_path() -> PathBuf {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    dir.join("config.json")
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<Config>(&text) {
                return cfg;
            }
        }
        Config::default()
    }

    pub fn save(&self) {
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(config_path(), text);
        }
    }
}
