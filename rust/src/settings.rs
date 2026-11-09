use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const NAME: &str = "Habbo.si Client Beta";
/// The product name shown in Discord's artwork hover; the build channel stays out of it.
pub const SHORT_NAME: &str = "Habbo.si Client";
pub const VERSION: &str = "1.2.0-beta.5";
pub const RELEASE_TAG: &str = "1.2.0-beta.5";
/// Stamped by `build.rs`, so the header always shows the binary it belongs to.
pub const BUILD_DATE: &str = match option_env!("HSC_BUILD_DATE") {
    Some(value) => value,
    None => "2026-11-08",
};
pub const BUILD_TIME: &str = match option_env!("HSC_BUILD_TIME") {
    Some(value) => value,
    None => "00:00",
};
pub const CLIENT_URL: &str = "https://habbo.si/client";
pub const DATA_FOLDER: &str = "HabboSiClientBetaRust";

/// Everything the Rust beta keeps between runs. The field names intentionally match the
/// C# beta client where they overlap, but this port uses its own folder and WebView2 profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub german: bool,
    pub gpu: bool,
    pub nothrottle: bool,
    pub unlockfps: bool,
    pub showfps: bool,
    pub devtools: bool,
    pub discord: bool,
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
            gpu: true,
            nothrottle: true,
            unlockfps: true,
            showfps: true,
            devtools: false,
            discord: true,
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
}

#[derive(Debug, Deserialize)]
pub struct SettingsForm {
    pub theme: String,
    pub german: bool,
    pub channel: String,
    pub gpu: bool,
    pub nothrottle: bool,
    pub unlockfps: bool,
    pub showfps: bool,
    pub devtools: bool,
    pub discord: bool,
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
    let _ = fs::create_dir_all(profile_dir(dir));
    let _ = fs::create_dir_all(surface_dir(dir));
}

pub fn profile_dir(dir: &Path) -> PathBuf {
    dir.join("WebView2")
}

/// The chrome, footer and splash share one profile, separate from the hotel surface.
/// WebView2 refuses two environments over the same user data folder when their
/// browser arguments differ, which the hotel surface needs for the renderer switches.
pub fn surface_dir(dir: &Path) -> PathBuf {
    dir.join("WebView2-surface")
}

pub fn clear_cache_marker(dir: &Path) -> PathBuf {
    dir.join("clear-cache")
}

pub fn queue_cache_clear(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(clear_cache_marker(dir), VERSION)
}

/// Called before WebView2 opens its profile. The running WebView2 owns the files,
/// so clearing is deliberately deferred until the next launch.
pub fn apply_pending_cache_clear(dir: &Path) {
    if clear_cache_marker(dir).exists() {
        let _ = fs::remove_dir_all(profile_dir(dir));
        let _ = fs::remove_file(clear_cache_marker(dir));
    }
    let _ = fs::create_dir_all(profile_dir(dir));
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
            if settings.channel != "beta" && settings.channel != "stable" {
                settings.channel = "beta".into();
            }
            if settings.theme != "light" && settings.theme != "navy" {
                settings.theme = "dark".into();
            }
            settings
        }
        Err(_) => Settings::default(),
    }
}

pub fn save(dir: &Path, settings: &Settings) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let settings = settings.clone();
    let text = serde_json::to_string_pretty(&settings).unwrap_or_else(|_| "{}".into());
    fs::write(settings_path(dir), format!("{text}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_beta() {
        assert!(Settings::default().beta_channel());
        assert_eq!(
            Settings::default()
                .browser_args()
                .matches("--disable-frame-rate-limit")
                .count(),
            1
        );
    }
}
