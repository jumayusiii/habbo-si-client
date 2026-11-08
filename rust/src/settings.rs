use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const NAME: &str = "Habbo.si Client Beta";
pub const VERSION: &str = "1.2.0-beta.2";
pub const RELEASE_TAG: &str = "1.2.0-beta.2";
pub const BUILD_DATE: &str = "2026-11-08";
pub const CLIENT_URL: &str = "https://habbo.si/client";
pub const DATA_FOLDER: &str = "HabboSiClientBetaRust";

/// Everything the Rust beta keeps between runs. The field names intentionally match the
/// C# beta client where they overlap, but this port uses its own folder and WebView2 profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub german: bool,
    pub zoom: f64,
    pub gpu: bool,
    pub nothrottle: bool,
    pub unlockfps: bool,
    pub showfps: bool,
    pub devtools: bool,
    pub channel: String,
    pub seenrelease: String,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            german: false,
            zoom: 1.0,
            gpu: true,
            nothrottle: true,
            unlockfps: true,
            showfps: true,
            devtools: false,
            channel: "beta".into(),
            seenrelease: String::new(),
            width: 1360,
            height: 860,
            maximized: false,
        }
    }
}

impl Settings {
    pub fn beta_channel(&self) -> bool {
        self.channel == "beta"
    }

    pub fn browser_args(&self) -> String {
        let mut args = Vec::new();
        if self.gpu {
            args.push("--enable-gpu-rasterization");
            args.push("--enable-zero-copy");
            args.push("--ignore-gpu-blocklist");
        }
        if self.unlockfps {
            args.push("--disable-frame-rate-limit");
            args.push("--disable-gpu-vsync");
        }
        if self.nothrottle {
            args.push("--disable-background-timer-throttling");
            args.push("--disable-renderer-backgrounding");
            args.push("--disable-backgrounding-occluded-windows");
            args.push("--disable-features=CalculateNativeWinOcclusion,IntensiveWakeUpThrottling");
        }
        // These are the defaults wry would otherwise add. They are kept here because
        // with_additional_browser_args replaces the generated argument list.
        args.push("--disable-component-update");
        args.push("--autoplay-policy=no-user-gesture-required");
        args.join(" ")
    }

    pub fn clamp_zoom(&mut self) {
        if !self.zoom.is_finite() {
            self.zoom = 1.0;
        }
        self.zoom = self.zoom.clamp(0.5, 2.0);
    }
}

#[derive(Debug, Deserialize)]
pub struct SettingsForm {
    pub channel: String,
    pub gpu: bool,
    pub nothrottle: bool,
    pub unlockfps: bool,
    pub showfps: bool,
    pub devtools: bool,
}

pub fn value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let mut i = 0;
    while i < args.len() {
        if args[i].eq_ignore_ascii_case(name) {
            return args.get(i + 1).map(String::as_str);
        }
        i += 1;
    }
    None
}

pub fn has(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a.eq_ignore_ascii_case(name))
}

pub fn data_dir(args: &[String]) -> PathBuf {
    if let Some(path) = value(args, "--data") {
        return PathBuf::from(path);
    }
    if let Some(root) = env::var_os("LOCALAPPDATA") {
        return PathBuf::from(root).join(DATA_FOLDER);
    }
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(DATA_FOLDER)
}

pub fn ensure(dir: &Path) {
    let _ = fs::create_dir_all(dir);
    let _ = fs::create_dir_all(dir.join("WebView2"));
}

pub fn settings_path(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

pub fn load(dir: &Path) -> Settings {
    let text = match fs::read_to_string(settings_path(dir)) {
        Ok(text) => text,
        Err(_) => return Settings::default(),
    };
    match serde_json::from_str::<Settings>(&text) {
        Ok(mut settings) => {
            settings.clamp_zoom();
            if settings.channel != "beta" && settings.channel != "stable" {
                settings.channel = "beta".into();
            }
            settings
        }
        Err(_) => Settings::default(),
    }
}

pub fn save(dir: &Path, settings: &Settings) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let mut settings = settings.clone();
    settings.clamp_zoom();
    let text = serde_json::to_string_pretty(&settings).unwrap_or_else(|_| "{}".into());
    fs::write(settings_path(dir), format!("{text}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_beta() {
        assert!(Settings::default().beta_channel());
        assert_eq!(Settings::default().browser_args().matches("--disable-frame-rate-limit").count(), 1);
    }

    #[test]
    fn clamps_zoom() {
        let mut s = Settings { zoom: 5.0, ..Settings::default() };
        s.clamp_zoom();
        assert_eq!(s.zoom, 2.0);
    }
}
