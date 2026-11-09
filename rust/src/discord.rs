use serde_json::json;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::settings::{NAME, VERSION};

pub const ARTWORK_URL: &str =
    "https://raw.githubusercontent.com/jumayusiii/habbo-si-client/main/assets/logo.png";

#[derive(Clone, Debug)]
struct Activity {
    details: String,
    state: String,
}

pub struct Discord {
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<String>>,
    activity: Arc<Mutex<Activity>>,
    worker: Option<JoinHandle<()>>,
}

impl Discord {
    pub fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new("off".into())),
            activity: Arc::new(Mutex::new(Activity {
                details: "Exploring the hotel".into(),
                state: "On habbo.si".into(),
            })),
            worker: None,
        }
    }

    pub fn status(&self) -> String {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| "off".into())
    }

    pub fn valid_id(id: &str) -> bool {
        let id = id.trim();
        (17..=20).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit())
    }

    pub fn start(&mut self, id: &str) {
        self.stop();
        let id = id.trim();
        if !Self::valid_id(id) {
            self.set_status("off");
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let status = Arc::new(Mutex::new("connecting".into()));
        let activity = Arc::new(Mutex::new(
            self.activity
                .lock()
                .map(|a| a.clone())
                .unwrap_or_else(|_| Activity {
                    details: "Exploring the hotel".into(),
                    state: "On habbo.si".into(),
                }),
        ));
        let worker_stop = stop.clone();
        let worker_status = status.clone();
        let worker_activity = activity.clone();
        let client_id = id.to_string();
        let worker = thread::spawn(move || {
            run_worker(client_id, worker_stop, worker_status, worker_activity);
        });
        self.stop = stop;
        self.status = status;
        self.activity = activity;
        self.worker = Some(worker);
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.set_status("off");
    }

    pub fn set_activity(&self, details: String, state: String) {
        if let Ok(mut activity) = self.activity.lock() {
            activity.details = details;
            activity.state = state;
        }
    }

    fn set_status(&self, value: &str) {
        if let Ok(mut status) = self.status.lock() {
            *status = value.into();
        }
    }
}

impl Drop for Discord {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_worker(
    client_id: String,
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<String>>,
    activity: Arc<Mutex<Activity>>,
) {
    let Some(mut pipe) = open_pipe(&stop) else {
        set(&status, "no-discord");
        return;
    };
    if !handshake(&mut pipe, &client_id) {
        set(&status, "error");
        return;
    }

    if let Ok(mut reader) = pipe.try_clone() {
        thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            while reader.read(&mut buffer).map(|n| n > 0).unwrap_or(false) {}
        });
    }

    set(&status, "connected");
    let mut last = Instant::now() - Duration::from_secs(10);
    while !stop.load(Ordering::Relaxed) {
        if last.elapsed() >= Duration::from_millis(3500) {
            let current = activity
                .lock()
                .map(|a| a.clone())
                .unwrap_or_else(|_| Activity {
                    details: "Exploring the hotel".into(),
                    state: "On habbo.si".into(),
                });
            let frame = activity_json(
                std::process::id(),
                now_unix(),
                &current.details,
                &current.state,
                ARTWORK_URL,
                &format!("{NAME} v{VERSION}"),
            );
            if !write_frame(&mut pipe, 1, &frame) {
                break;
            }
            last = Instant::now();
        }
        thread::sleep(Duration::from_millis(200));
    }
    set(&status, "off");
}

fn set(status: &Arc<Mutex<String>>, value: &str) {
    if let Ok(mut status) = status.lock() {
        *status = value.into();
    }
}

fn open_pipe(stop: &Arc<AtomicBool>) -> Option<File> {
    for index in 0..10 {
        if stop.load(Ordering::Relaxed) {
            return None;
        }
        let path = format!(r"\\.\pipe\discord-ipc-{index}");
        if let Ok(file) = OpenOptions::new().read(true).write(true).open(path) {
            return Some(file);
        }
    }
    None
}

fn handshake(pipe: &mut File, client_id: &str) -> bool {
    let payload = json!({ "v": 1, "client_id": client_id }).to_string();
    write_frame(pipe, 0, &payload)
}

fn write_frame(pipe: &mut File, opcode: u32, payload: &str) -> bool {
    let bytes = payload.as_bytes();
    let mut header = [0u8; 8];
    header[..4].copy_from_slice(&opcode.to_le_bytes());
    header[4..].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
    pipe.write_all(&header).is_ok() && pipe.write_all(bytes).is_ok() && pipe.flush().is_ok()
}

pub fn configure(settings: &crate::settings::Settings) -> Option<String> {
    if !settings.discord {
        return None;
    }
    let custom = settings.discordid.trim();
    if Discord::valid_id(custom) {
        Some(custom.to_string())
    } else {
        None
    }
}

pub fn presence_details(room: &str) -> String {
    let room = pretty_room(room);
    if room.is_empty() {
        "Exploring the hotel".into()
    } else {
        format!("Hanging out in {room}")
    }
}

pub fn activity_json(
    pid: u32,
    start: i64,
    details: &str,
    state: &str,
    image: &str,
    large_text: &str,
) -> String {
    let nonce = format!(
        "{}-{}",
        now_unix(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    );
    json!({
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": pid,
            "activity": {
                "details": if details.len() < 2 { "Exploring the hotel" } else { details },
                "state": if state.len() < 2 { "On habbo.si" } else { state },
                "assets": { "large_image": image, "large_text": large_text },
                "timestamps": { "start": start },
                "instance": true
            }
        },
        "nonce": nonce
    })
    .to_string()
}

/// The C# client title-cases shouty or all-lowercase room names but keeps a normal name as-is.
fn pretty_room(room: &str) -> String {
    let mut s = room.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.is_empty() {
        return s;
    }
    if s.chars().count() > 48 {
        s = s
            .chars()
            .take(47)
            .collect::<String>()
            .trim_end()
            .to_string();
        s.push('…');
    }
    let lower = s.to_lowercase();
    let shouty = s == s.to_uppercase() && s != lower;
    let flat = s == lower;
    if shouty || flat {
        let mut out = String::with_capacity(s.len());
        for word in lower.split(' ') {
            if !out.is_empty() {
                out.push(' ');
            }
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                out.extend(first.to_uppercase());
                out.extend(chars);
            }
        }
        s = out;
    }
    s
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn validates_application_ids() {
        assert!(Discord::valid_id("12345678901234567"));
        assert!(!Discord::valid_id("123"));
        assert!(!Discord::valid_id("1234567890123456x"));
    }

    #[test]
    fn presence_is_sentence_cased() {
        assert_eq!(
            presence_details("SUNSET LOUNGE"),
            "Hanging out in Sunset Lounge"
        );
        assert_eq!(
            presence_details("sunset lounge"),
            "Hanging out in Sunset Lounge"
        );
        assert_eq!(
            presence_details("Sunset Lounge"),
            "Hanging out in Sunset Lounge"
        );
        assert_eq!(presence_details(""), "Exploring the hotel");
    }

    #[test]
    fn activity_contains_the_expected_shape() {
        let value: Value = serde_json::from_str(&activity_json(
            42,
            100,
            "Hanging out in Sunrise",
            "On habbo.si",
            ARTWORK_URL,
            "Habbo.si Client",
        ))
        .unwrap();
        assert_eq!(value["cmd"], "SET_ACTIVITY");
        assert_eq!(value["args"]["pid"], 42);
        assert_eq!(
            value["args"]["activity"]["details"],
            "Hanging out in Sunrise"
        );
    }
}
