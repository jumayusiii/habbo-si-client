use serde_json::json;
use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::windows::io::AsRawHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::settings::{SHORT_NAME, VERSION};

pub const ARTWORK_URL: &str =
    "https://raw.githubusercontent.com/jumayusiii/habbo-si-client/main/assets/logo.png";

/// Application ID of the Habbo.si Discord application, baked in so presence works without
/// the player registering an app. A valid ID typed into Settings still takes precedence.
pub const BUILT_IN_APP_ID: &str = "1563961714989604955";

/// Hover text on the large artwork: the short client name and the running build's version.
pub fn large_text() -> String {
    format!("{SHORT_NAME} v{VERSION}")
}

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const OP_PING: u32 = 3;
const OP_PONG: u32 = 4;

#[derive(Clone, Debug)]
struct Activity {
    details: String,
    state: String,
}

impl Default for Activity {
    fn default() -> Self {
        Self {
            details: "Exploring the hotel".into(),
            state: "On habbo.si".into(),
        }
    }
}

pub struct Discord {
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<String>>,
    activity: Arc<Mutex<Activity>>,
    /// Set when the visible lines change; the worker sends those without waiting for the
    /// 25 s heartbeat that keeps the presence alive.
    dirty: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Discord {
    pub fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new("off".into())),
            activity: Arc::new(Mutex::new(Activity::default())),
            dirty: Arc::new(AtomicBool::new(false)),
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
            self.activity.lock().map(|a| a.clone()).unwrap_or_default(),
        ));
        let dirty = Arc::new(AtomicBool::new(true));
        let worker_stop = stop.clone();
        let worker_status = status.clone();
        let worker_activity = activity.clone();
        let worker_dirty = dirty.clone();
        let client_id = id.to_string();
        // The elapsed time in Discord keeps counting across reconnects, exactly like the
        // C# client: a new connection must not reset it to 0:00.
        let started = now_unix();
        let worker = thread::spawn(move || {
            run_worker(
                client_id,
                started,
                worker_stop,
                worker_status,
                worker_activity,
                worker_dirty,
            );
        });
        self.stop = stop;
        self.status = status;
        self.activity = activity;
        self.dirty = dirty;
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
        self.dirty.store(true, Ordering::Relaxed);
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

/// Discord starts its IPC pipe after the desktop client has booted, so a client that opens the
/// pipe once at launch never shows a presence. This supervisor keeps trying while the app runs
/// and reconnects whenever Discord restarts or drops the connection.
fn run_worker(
    client_id: String,
    started: i64,
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<String>>,
    activity: Arc<Mutex<Activity>>,
    dirty: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Relaxed) {
        set(&status, "connecting");
        let Some(mut pipe) = open_pipe(&stop) else {
            set(&status, "no-discord");
            if !sleep_stop(&stop, Duration::from_secs(8)) {
                break;
            }
            continue;
        };
        if !handshake(&mut pipe, &client_id, &stop) {
            set(&status, "error");
            if !sleep_stop(&stop, Duration::from_secs(6)) {
                break;
            }
            continue;
        }
        set(&status, "connected");
        dirty.store(true, Ordering::Relaxed);
        session(&mut pipe, started, &stop, &status, &activity, &dirty);
        if !stop.load(Ordering::Relaxed) {
            set(&status, "connecting");
            let _ = sleep_stop(&stop, Duration::from_secs(4));
        }
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

/// Sends the handshake and waits for Discord's READY before any activity frame is written.
/// A rejected application ID arrives as an ERROR frame, so the client reports it instead of
/// claiming to be connected while Discord shows nothing.
fn handshake(pipe: &mut File, client_id: &str, stop: &Arc<AtomicBool>) -> bool {
    let payload = json!({ "v": 1, "client_id": client_id }).to_string();
    if !write_frame(pipe, OP_HANDSHAKE, &payload) {
        return false;
    }
    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline {
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        match poll_frame(pipe) {
            Poll::Frame(OP_PING, payload) => {
                if !write_frame(pipe, OP_PONG, &payload) {
                    return false;
                }
            }
            Poll::Frame(OP_CLOSE, _) | Poll::Closed => return false,
            Poll::Frame(OP_FRAME, payload) => match event(&payload).as_deref() {
                Some("READY") => return true,
                Some("ERROR") => return false,
                _ => {}
            },
            Poll::Frame(_, _) => {}
            Poll::Empty => thread::sleep(Duration::from_millis(50)),
        }
    }
    false
}

fn session(
    pipe: &mut File,
    started: i64,
    stop: &Arc<AtomicBool>,
    status: &Arc<Mutex<String>>,
    activity: &Arc<Mutex<Activity>>,
    dirty: &Arc<AtomicBool>,
) {
    let mut last_send: Option<Instant> = None;
    while !stop.load(Ordering::Relaxed) {
        loop {
            match poll_frame(pipe) {
                Poll::Frame(OP_PING, payload) => {
                    if !write_frame(pipe, OP_PONG, &payload) {
                        return;
                    }
                }
                Poll::Frame(OP_CLOSE, _) | Poll::Closed => return,
                Poll::Frame(OP_FRAME, payload) => {
                    if event(&payload).as_deref() == Some("ERROR") {
                        set(status, "error");
                        return;
                    }
                }
                Poll::Frame(_, _) => {}
                Poll::Empty => break,
            }
        }

        // Discord accepts roughly five activity updates per 20 s; the change flag avoids
        // burning that budget while the heartbeat keeps the presence from expiring.
        let changed = dirty.load(Ordering::Relaxed);
        let due = changed || last_send.is_none_or(|sent| sent.elapsed() >= Duration::from_secs(15));
        let allowed = last_send.is_none_or(|sent| sent.elapsed() >= Duration::from_secs(4));
        if due && allowed {
            let current = activity.lock().map(|a| a.clone()).unwrap_or_default();
            let frame = activity_json(
                std::process::id(),
                started,
                &current.details,
                &current.state,
                ARTWORK_URL,
                &large_text(),
            );
            if !write_frame(pipe, OP_FRAME, &frame) {
                return;
            }
            last_send = Some(Instant::now());
            dirty.store(false, Ordering::Relaxed);
        }

        if !sleep_stop(stop, Duration::from_millis(250)) {
            return;
        }
    }
}

fn sleep_stop(stop: &Arc<AtomicBool>, duration: Duration) -> bool {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        let left = deadline.saturating_duration_since(Instant::now());
        thread::sleep(left.min(Duration::from_millis(100)));
    }
    !stop.load(Ordering::Relaxed)
}

fn event(payload: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(payload).ok()?;
    value
        .get("evt")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

enum Poll {
    Frame(u32, String),
    Empty,
    Closed,
}

/// Reads only when the pipe already holds a whole frame. A blocking read parked on the
/// synchronous handle would also block the activity writes from this thread, which is why
/// the presence never left the client before.
fn poll_frame(pipe: &mut File) -> Poll {
    let mut header = [0u8; 8];
    let mut read = 0u32;
    let mut available = 0u32;
    let ok = unsafe {
        PeekNamedPipe(
            pipe.as_raw_handle(),
            header.as_mut_ptr().cast(),
            header.len() as u32,
            &mut read,
            &mut available,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Poll::Closed;
    }
    if available < 8 {
        return Poll::Empty;
    }
    let opcode = u32::from_le_bytes(header[0..4].try_into().unwrap_or_default());
    let length = u32::from_le_bytes(header[4..8].try_into().unwrap_or_default()) as usize;
    if length > 8 * 1024 * 1024 {
        return Poll::Closed;
    }
    if (available as usize) < 8 + length {
        return Poll::Empty;
    }
    let mut full = [0u8; 8];
    let mut payload = vec![0u8; length];
    if pipe.read_exact(&mut full).is_err() {
        return Poll::Closed;
    }
    if length > 0 && pipe.read_exact(&mut payload).is_err() {
        return Poll::Closed;
    }
    Poll::Frame(opcode, String::from_utf8_lossy(&payload).into_owned())
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn PeekNamedPipe(
        pipe: *mut c_void,
        buffer: *mut c_void,
        size: u32,
        read: *mut u32,
        available: *mut u32,
        left: *mut u32,
    ) -> i32;
}

fn write_frame(pipe: &mut File, opcode: u32, payload: &str) -> bool {
    let bytes = payload.as_bytes();
    let mut header = [0u8; 8];
    header[..4].copy_from_slice(&opcode.to_le_bytes());
    header[4..].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
    pipe.write_all(&header).is_ok() && pipe.write_all(bytes).is_ok() && pipe.flush().is_ok()
}

/// Picks the custom application ID when the field holds a valid one, otherwise the ID
/// this build ships with. An invalid or empty field therefore still gets a working
/// presence instead of silently doing nothing.
pub fn resolve_id(custom: &str, built_in: &str) -> Option<String> {
    let custom = custom.trim();
    if Discord::valid_id(custom) {
        return Some(custom.to_string());
    }
    let built_in = built_in.trim();
    if Discord::valid_id(built_in) {
        return Some(built_in.to_string());
    }
    None
}

pub fn configure(settings: &crate::settings::Settings) -> Option<String> {
    if !settings.discord {
        return None;
    }
    resolve_id(&settings.discordid, BUILT_IN_APP_ID)
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
    fn ships_a_valid_built_in_application_id() {
        assert!(Discord::valid_id(BUILT_IN_APP_ID));
    }

    #[test]
    fn presence_uses_the_built_in_app_by_default() {
        let mut settings = crate::settings::Settings::default();
        assert_eq!(configure(&settings), Some(BUILT_IN_APP_ID.to_string()));
        settings.discordid = "76543210987654321".into();
        assert_eq!(configure(&settings), Some("76543210987654321".to_string()));
        settings.discord = false;
        assert_eq!(configure(&settings), None);
    }

    #[test]
    fn built_in_application_id_fills_in_without_a_custom_one() {
        assert_eq!(
            resolve_id("", "12345678901234567").as_deref(),
            Some("12345678901234567")
        );
        assert_eq!(
            resolve_id("  76543210987654321  ", "12345678901234567").as_deref(),
            Some("76543210987654321")
        );
        assert_eq!(
            resolve_id("not-an-id", "12345678901234567").as_deref(),
            Some("12345678901234567")
        );
        assert_eq!(resolve_id("", ""), None);
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
    fn artwork_hover_names_the_build() {
        assert_eq!(large_text(), format!("Habbo.si Client v{VERSION}"));
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
