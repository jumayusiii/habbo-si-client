#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod discord;
mod lang;
mod pages;
mod room;
mod settings;
mod theme;
mod updates;

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tao::dpi::{LogicalPosition, LogicalSize, PhysicalPosition};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget};
use tao::platform::windows::{WindowBuilderExtWindows, WindowExtWindows};
use tao::window::{Fullscreen, Icon, ResizeDirection, Window, WindowBuilder};
use wry::{Rect, Theme as WebTheme, WebContext, WebView, WebViewBuilder, WebViewBuilderExtWindows};

use discord::Discord;
use lang::Lang;
use settings::{Settings, SettingsForm};

const BAR_HEIGHT: f64 = 64.0;
const FOOTER_HEIGHT: f64 = 30.0;
const SPLASH_WIDTH: f64 = 560.0;
const SPLASH_HEIGHT: f64 = 320.0;

#[derive(Debug)]
enum Action {
    Chrome(String),
    Hotel(String),
    Sheet(u64, String),
    Room(String),
    PollRoom,
    Probe,
    ProbeRoom(String),
    ProbeDiag(String),
    Update(updates::UpdateInfo),
    Notes(updates::ReleaseNotes),
    ShowNotes,
    RefreshChrome,
    SplashDone,
    Smoke,
    Shot,
    ShotLive,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SheetKind {
    Settings,
    Notes,
    Update,
}

struct Sheet {
    id: u64,
    kind: SheetKind,
    _webview: WebView,
    window: Window,
    _context: WebContext,
    page_url: Option<String>,
    /// The saved palette, captured on the first live preview so closing or
    /// cancelling the sheet can put the client back exactly as it was.
    preview_theme: Option<String>,
}

struct Shell {
    window: Window,
    chrome: WebView,
    hotel: Option<WebView>,
    footer: WebView,
    splash: Option<WebView>,
    _hotel_context: WebContext,
    _surface_context: WebContext,
    fullscreen: bool,
    splash_active: bool,
    fps: Option<u32>,
    zoom: f64,
    room: String,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if let Some(path) = settings::value(&args, "--presence-json") {
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let json = discord::activity_json(
            std::process::id(),
            started,
            &discord::presence_details("SUNSET LOUNGE"),
            "On habbo.si",
            discord::ARTWORK_URL,
            &format!("{} v{}", settings::NAME, settings::VERSION),
        );
        if let Err(error) = fs::write(path, json) {
            eprintln!("could not write presence payload: {error}");
        }
        return;
    }
    if let Some(path) = settings::value(&args, "--update-check") {
        write_update_check(&args, path);
        return;
    }
    if let Some(path) = settings::value(&args, "--release-notes") {
        write_release_notes(path);
        return;
    }
    if settings::has(&args, "--version") {
        println!("{} {} (Rust)", settings::NAME, settings::VERSION);
        return;
    }

    if let Err(error) = run(args.clone()) {
        let dir = settings::data_dir(&args);
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(dir.join("error.log"), format!("{error}\n"));
        eprintln!("{error}");
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let dir = settings::data_dir(&args);
    settings::ensure(&dir);
    // The running WebView2 owns its profile, so a clear queued by the last run is
    // applied here, before the profile is opened again.
    settings::apply_pending_cache_clear(&dir);

    let mut settings = settings::load(&dir);
    apply_overrides(&mut settings, &args);
    settings.clamp_zoom();

    let shot_path = settings::value(&args, "--shot").map(PathBuf::from);
    let live_path = settings::value(&args, "--shot-live").map(PathBuf::from);
    let smoke_path = settings::value(&args, "--smoke").map(PathBuf::from);
    let probe_path = settings::value(&args, "--probe").map(PathBuf::from);
    let preview =
        shot_path.is_some() || live_path.is_some() || smoke_path.is_some() || probe_path.is_some();
    let presence_allowed = !preview;
    let show_splash = !preview && !settings::has(&args, "--nosplash");

    if let Some(path) = settings::value(&args, "--shot-settings") {
        let html = pages::settings_page(&settings, "off");
        return shot_sheet(path, "Settings — Habbo.si Client Beta", 900.0, 620.0, html);
    }
    if let Some(path) = settings::value(&args, "--shot-notes") {
        let Some(notes) = updates::current_notes() else {
            return Err("no release notes available".into());
        };
        let html = pages::notes_page(&notes, &settings);
        return shot_sheet(
            path,
            "What's new — Habbo.si Client Beta",
            700.0,
            640.0,
            html,
        );
    }

    let event_loop = EventLoopBuilder::<Action>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let title = base_title();
    let scale = 1.0;
    let logical = LogicalSize::new(
        settings.width.clamp(980, 10000) as f64 / scale,
        settings.height.clamp(620, 10000) as f64 / scale,
    );

    let window = WindowBuilder::new()
        .with_title(&title)
        .with_decorations(false)
        .with_visible(false)
        .with_transparent(false)
        .with_inner_size(logical)
        .with_min_inner_size(LogicalSize::new(980.0, 620.0))
        .with_maximized(settings.maximized)
        .with_window_icon(icon())
        .with_theme(Some(window_theme(&settings)))
        .with_background_color(theme::rgb(&settings.theme))
        .build(&event_loop)
        .map_err(|error| format!("could not create the window: {error}"))?;

    let mut hotel_context = WebContext::new(Some(settings::profile_dir(&dir)));
    let mut surface_context = WebContext::new(Some(settings::surface_dir(&dir)));
    let scale = window.scale_factor();
    let size = window.inner_size();
    let width = size.width as f64 / scale;
    let height = size.height as f64 / scale;
    let middle = (height - BAR_HEIGHT - FOOTER_HEIGHT).max(0.0);

    // Build the splash first and show the window as soon as it exists. The hotel, chrome
    // and footer stay hidden until it is dismissed, so WebView2 startup happens behind the
    // boot screen instead of leaving an empty client on screen for several seconds.
    let splash = if show_splash {
        let splash_width = SPLASH_WIDTH.min(width);
        let splash_height = SPLASH_HEIGHT.min(height);
        Some(
            WebViewBuilder::new_with_web_context(&mut surface_context)
                .with_html(pages::splash_page())
                .with_bounds(rect(
                    (width - splash_width) / 2.0,
                    (height - splash_height) / 2.0,
                    splash_width,
                    splash_height,
                ))
                .with_background_color((30, 30, 30, 255))
                .with_focused(false)
                .build_as_child(&window)
                .map_err(|error| format!("the splash screen could not start: {error}"))?,
        )
    } else {
        None
    };

    if splash.is_some() {
        window.set_visible(true);
        window.set_focus();
    }

    let hotel = if shot_path.is_some() {
        None
    } else {
        let hotel_proxy = proxy.clone();
        Some(
            WebViewBuilder::new_with_web_context(&mut hotel_context)
                .with_url(settings::CLIENT_URL)
                .with_background_color(theme::rgb(&settings.theme))
                .with_bounds(rect(0.0, BAR_HEIGHT, width, middle))
                .with_visible(!show_splash)
                .with_devtools(true)
                .with_clipboard(true)
                .with_theme(web_theme(&settings))
                .with_hotkeys_zoom(false)
                .with_initialization_script(INIT_SCRIPT)
                .with_additional_browser_args(&settings.browser_args())
                .with_ipc_handler(move |request| {
                    let _ = hotel_proxy.send_event(Action::Hotel(request.body().to_string()));
                })
                .build_as_child(&window)
                .map_err(|error| format!("WebView2 could not start: {error}"))?,
        )
    };
    let _ = hotel.as_ref().map(|hotel| hotel.zoom(settings.zoom));

    let chrome_proxy = proxy.clone();
    let chrome = WebViewBuilder::new_with_web_context(&mut surface_context)
        .with_html(pages::chrome_page(&settings))
        .with_bounds(rect(0.0, 0.0, width, BAR_HEIGHT))
        .with_background_color(theme::rgb(&settings.theme))
        .with_visible(!show_splash)
        .with_focused(false)
        .with_ipc_handler(move |request| {
            let _ = chrome_proxy.send_event(Action::Chrome(request.body().to_string()));
        })
        .build_as_child(&window)
        .map_err(|error| format!("the client chrome could not start: {error}"))?;

    let footer = WebViewBuilder::new_with_web_context(&mut surface_context)
        .with_html(pages::footer_page(&settings))
        .with_bounds(rect(0.0, BAR_HEIGHT + middle, width, FOOTER_HEIGHT))
        .with_background_color(theme::rgb(&settings.theme))
        .with_visible(!show_splash)
        .with_focused(false)
        .build_as_child(&window)
        .map_err(|error| format!("the client footer could not start: {error}"))?;

    let mut shell = Shell {
        window,
        chrome,
        hotel,
        footer,
        splash,
        _hotel_context: hotel_context,
        _surface_context: surface_context,
        fullscreen: false,
        splash_active: show_splash,
        fps: None,
        zoom: settings.zoom,
        room: String::new(),
    };
    shell.update_zoom_label();
    shell.update_max_icon(settings.maximized);
    footer_set(
        &shell.footer,
        &localized_starting(&settings),
        None,
        settings.showfps,
    );
    // Keep the window hidden until every surface exists, so the first visible frame is the
    // splash rather than an empty client for the time WebView2 takes to initialise.
    shell.window.set_visible(true);
    shell.window.set_focus();

    let main_id = shell.window.id();
    let mut sheet: Option<Sheet> = None;
    let mut sheet_seq = 0u64;
    let mut discord = Discord::new();
    if presence_allowed {
        start_presence(&mut discord, &settings);
    }

    if show_splash {
        let proxy = proxy.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1650));
            let _ = proxy.send_event(Action::SplashDone);
        });
    }

    if presence_allowed {
        let proxy = proxy.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(4));
                if proxy.send_event(Action::PollRoom).is_err() {
                    break;
                }
            }
        });
    }

    if let Some(path) = probe_path.clone() {
        let proxy = proxy.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(12));
            if proxy.send_event(Action::Probe).is_err() {
                let _ = path;
            }
        });
    }
    if smoke_path.is_some() {
        let proxy = proxy.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(20));
            let _ = proxy.send_event(Action::Smoke);
        });
    }
    if shot_path.is_some() {
        let proxy = proxy.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1800));
            let _ = proxy.send_event(Action::Shot);
        });
    }
    if live_path.is_some() {
        let proxy = proxy.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(10));
            let _ = proxy.send_event(Action::ShotLive);
        });
    }
    if !preview && !settings::has(&args, "--no-update-check") {
        spawn_update_check(
            proxy.clone(),
            settings.beta_channel(),
            settings.seenrelease.clone(),
        );
    }

    let mut probe_room: Option<String> = None;
    let mut probe_diag: Option<String> = None;
    let mut smoke_fired = false;

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } if window_id == main_id => {
                restore_open_sheet(&mut sheet, &shell, &mut settings, &proxy);
                persist(&shell, &mut settings, &dir, preview);
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                window_id, event, ..
            } if window_id == main_id => match event {
                WindowEvent::Resized(_) => {
                    shell.layout();
                    if !shell.fullscreen && !shell.window.is_maximized() {
                        let scale = shell.window.scale_factor();
                        let size = shell.window.inner_size();
                        settings.width = (size.width as f64 / scale).round() as u32;
                        settings.height = (size.height as f64 / scale).round() as u32;
                    }
                }
                WindowEvent::ScaleFactorChanged { .. } => shell.layout(),
                _ => {}
            },
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if sheet
                    .as_ref()
                    .map(|current| current.window.id() == window_id)
                    .unwrap_or(false)
                {
                    if let Some(mut current) = sheet.take() {
                        restore_theme(&mut current, &shell, &mut settings, &proxy);
                    }
                }
            }
            Event::UserEvent(action) => match action {
                Action::Chrome(message) => {
                    let quit = shell_action(
                        &message,
                        &mut shell,
                        &mut settings,
                        &dir,
                        &proxy,
                        target,
                        &mut sheet,
                        &mut sheet_seq,
                        &mut discord,
                    );
                    if quit {
                        restore_open_sheet(&mut sheet, &shell, &mut settings, &proxy);
                        persist(&shell, &mut settings, &dir, preview);
                        *control_flow = ControlFlow::Exit;
                    }
                }
                Action::RefreshChrome => {
                    // The chrome reloads asynchronously after a theme or language change.
                    shell.update_zoom_label();
                    shell.update_max_icon(shell.window.is_maximized());
                    footer_set(
                        &shell.footer,
                        &shell.status(&settings),
                        shell.fps,
                        settings.showfps,
                    );
                }
                Action::Hotel(message) => {
                    if let Some(value) = message.strip_prefix("fps:") {
                        if let Ok(value) = value.parse::<u32>() {
                            shell.fps = Some(value);
                            footer_set(
                                &shell.footer,
                                &shell.status(&settings),
                                shell.fps,
                                settings.showfps,
                            );
                        }
                        return;
                    }
                    if message == "loaded" {
                        footer_set(
                            &shell.footer,
                            lang(&settings).t("Connected", "Verbunden"),
                            shell.fps,
                            settings.showfps,
                        );
                        // A smoke run reports the URL only after a real navigation finished.
                        if smoke_path.is_some() && !smoke_fired {
                            smoke_fired = true;
                            let proxy = proxy.clone();
                            thread::spawn(move || {
                                thread::sleep(Duration::from_millis(1200));
                                let _ = proxy.send_event(Action::Smoke);
                            });
                        }
                        return;
                    }
                    let quit = shell_action(
                        &message,
                        &mut shell,
                        &mut settings,
                        &dir,
                        &proxy,
                        target,
                        &mut sheet,
                        &mut sheet_seq,
                        &mut discord,
                    );
                    if quit {
                        restore_open_sheet(&mut sheet, &shell, &mut settings, &proxy);
                        persist(&shell, &mut settings, &dir, preview);
                        *control_flow = ControlFlow::Exit;
                    }
                }
                Action::Sheet(id, message) => {
                    let mut close = false;
                    if let Some(current) = sheet.as_mut().filter(|current| current.id == id) {
                        close = handle_sheet_message(
                            &message,
                            current,
                            &shell,
                            &mut settings,
                            &dir,
                            &proxy,
                            &mut discord,
                        );
                        if close {
                            restore_theme(current, &shell, &mut settings, &proxy);
                        }
                    }
                    if close {
                        sheet = None;
                    }
                }
                Action::Room(raw) => {
                    let raw = room::unquote(&raw);
                    if let Some(found) = room::pick_room(&raw) {
                        if found != shell.room {
                            shell.room = found;
                            if presence_allowed && settings.discord {
                                discord.set_activity(
                                    discord::presence_details(&shell.room),
                                    "On habbo.si".to_string(),
                                );
                            }
                            footer_set(
                                &shell.footer,
                                &shell.status(&settings),
                                shell.fps,
                                settings.showfps,
                            );
                        }
                    }
                }
                Action::PollRoom => {
                    if let Some(hotel) = &shell.hotel {
                        let proxy = proxy.clone();
                        let _ = hotel.evaluate_script_with_callback(room::ROOM_JS, move |value| {
                            let _ = proxy.send_event(Action::Room(value));
                        });
                    }
                }
                Action::Probe => {
                    if let Some(hotel) = &shell.hotel {
                        let room_proxy = proxy.clone();
                        let _ = hotel.evaluate_script_with_callback(room::ROOM_JS, move |value| {
                            let _ = room_proxy.send_event(Action::ProbeRoom(value));
                        });
                        let diag_proxy = proxy.clone();
                        let _ = hotel.evaluate_script_with_callback(room::DIAG_JS, move |value| {
                            let _ = diag_proxy.send_event(Action::ProbeDiag(value));
                        });
                    }
                }
                Action::ProbeRoom(value) => {
                    probe_room = Some(value);
                    if let (Some(path), Some(diag)) = (&probe_path, &probe_diag) {
                        write_probe(path, probe_room.as_deref().unwrap_or(""), diag);
                        *control_flow = ControlFlow::Exit;
                    }
                }
                Action::ProbeDiag(value) => {
                    probe_diag = Some(value);
                    if let (Some(path), Some(found)) = (&probe_path, &probe_room) {
                        write_probe(path, found, probe_diag.as_deref().unwrap_or(""));
                        *control_flow = ControlFlow::Exit;
                    }
                }
                Action::Update(info) => {
                    if sheet.is_none() {
                        let id = next_sheet_id(&mut sheet_seq);
                        sheet = open_sheet(
                            target,
                            &proxy,
                            &shell.window,
                            &settings,
                            id,
                            SheetKind::Update,
                            "Update available — Habbo.si Client Beta",
                            560.0,
                            360.0,
                            pages::update_page(&info, &settings),
                            Some(info.page.clone()),
                            &dir.join("ui"),
                        );
                    }
                }
                Action::Notes(notes) => {
                    if sheet.is_none() {
                        let id = next_sheet_id(&mut sheet_seq);
                        let opened = open_sheet(
                            target,
                            &proxy,
                            &shell.window,
                            &settings,
                            id,
                            SheetKind::Notes,
                            "What's new — Habbo.si Client Beta",
                            700.0,
                            640.0,
                            pages::notes_page(&notes, &settings),
                            Some(notes.page.clone()),
                            &dir.join("ui"),
                        );
                        if opened.is_some() {
                            settings.seenrelease = settings::RELEASE_TAG.into();
                            let _ = settings::save(&dir, &settings);
                            sheet = opened;
                        }
                    }
                }
                Action::ShowNotes => {
                    let proxy = proxy.clone();
                    thread::spawn(move || {
                        if let Some(notes) = updates::current_notes() {
                            let _ = proxy.send_event(Action::Notes(notes));
                        }
                    });
                }
                Action::SplashDone => {
                    if let Some(splash) = shell.splash.take() {
                        let _ = splash.set_visible(false);
                    }
                    shell.splash_active = false;
                    shell.layout();
                    if let Some(hotel) = &shell.hotel {
                        let _ = hotel.focus();
                    }
                }
                Action::Smoke => {
                    if let Some(path) = &smoke_path {
                        let url = shell
                            .hotel
                            .as_ref()
                            .and_then(|hotel| hotel.url().ok())
                            .unwrap_or_default();
                        let ok = url.starts_with("http://") || url.starts_with("https://");
                        let text = format!(
                            "ok={}\r\nurl={url}\r\nversion={}\r\nchannel={}\r\n",
                            if ok { 1 } else { 0 },
                            settings::VERSION,
                            settings.channel
                        );
                        let _ = fs::write(path, text);
                    }
                    persist(&shell, &mut settings, &dir, true);
                    *control_flow = ControlFlow::Exit;
                }
                Action::Shot => {
                    if let Some(path) = &shot_path {
                        if let Err(error) = capture::capture_window(&shell.window, path) {
                            eprintln!("could not capture the window: {error}");
                        }
                    }
                    persist(&shell, &mut settings, &dir, true);
                    *control_flow = ControlFlow::Exit;
                }
                Action::ShotLive => {
                    if let Some(path) = &live_path {
                        if let Err(error) = capture::capture_window(&shell.window, path) {
                            eprintln!("could not capture the window: {error}");
                        }
                    }
                    persist(&shell, &mut settings, &dir, true);
                    *control_flow = ControlFlow::Exit;
                }
            },
            _ => {}
        }
    });
}

impl Shell {
    fn status(&self, settings: &Settings) -> String {
        let lang = Lang::new(settings.german);
        if self.room.is_empty() {
            lang.t("On habbo.si", "Auf habbo.si").to_string()
        } else {
            self.room.clone()
        }
    }

    fn layout(&self) {
        let scale = self.window.scale_factor();
        let size = self.window.inner_size();
        let width = size.width as f64 / scale;
        let height = size.height as f64 / scale;
        let bar = if self.fullscreen { 0.0 } else { BAR_HEIGHT };
        let foot = if self.fullscreen { 0.0 } else { FOOTER_HEIGHT };
        let middle = (height - bar - foot).max(0.0);
        let surfaces = !self.splash_active;
        let _ = self.chrome.set_visible(surfaces && !self.fullscreen);
        let _ = self.footer.set_visible(surfaces && !self.fullscreen);
        let _ = self.chrome.set_bounds(rect(0.0, 0.0, width, bar));
        let _ = self.footer.set_bounds(rect(0.0, bar + middle, width, foot));
        if let Some(hotel) = &self.hotel {
            let _ = hotel.set_visible(surfaces);
            let _ = hotel.set_bounds(rect(0.0, bar, width, middle));
        }
        if let Some(splash) = &self.splash {
            let splash_width = SPLASH_WIDTH.min(width);
            let splash_height = SPLASH_HEIGHT.min(height);
            let _ = splash.set_bounds(rect(
                (width - splash_width) / 2.0,
                (height - splash_height) / 2.0,
                splash_width,
                splash_height,
            ));
        }
    }

    fn update_zoom_label(&self) {
        let percent = (self.zoom * 100.0).round() as i64;
        let _ = self
            .chrome
            .evaluate_script(&format!("window.setZoom && window.setZoom({percent});"));
    }

    fn update_max_icon(&self, maximized: bool) {
        let _ = self.chrome.evaluate_script(&format!(
            "window.setMaximized && window.setMaximized({maximized});"
        ));
    }

    fn apply_zoom(&mut self) {
        if let Some(hotel) = &self.hotel {
            let _ = hotel.zoom(self.zoom);
        }
        self.update_zoom_label();
    }
}

#[allow(clippy::too_many_arguments)]
fn shell_action(
    message: &str,
    shell: &mut Shell,
    settings: &mut Settings,
    dir: &Path,
    proxy: &EventLoopProxy<Action>,
    target: &EventLoopWindowTarget<Action>,
    sheet: &mut Option<Sheet>,
    sheet_seq: &mut u64,
    discord: &mut Discord,
) -> bool {
    match message {
        "drag" => {
            let _ = shell.window.drag_window();
        }
        "resize-top" => {
            let _ = shell.window.drag_resize_window(ResizeDirection::North);
        }
        "resize-bottom" => {
            let _ = shell.window.drag_resize_window(ResizeDirection::South);
        }
        "resize-left" => {
            let _ = shell.window.drag_resize_window(ResizeDirection::West);
        }
        "resize-right" => {
            let _ = shell.window.drag_resize_window(ResizeDirection::East);
        }
        "resize-top-left" => {
            let _ = shell.window.drag_resize_window(ResizeDirection::NorthWest);
        }
        "resize-top-right" => {
            let _ = shell.window.drag_resize_window(ResizeDirection::NorthEast);
        }
        "resize-bottom-left" => {
            let _ = shell.window.drag_resize_window(ResizeDirection::SouthWest);
        }
        "resize-bottom-right" => {
            let _ = shell.window.drag_resize_window(ResizeDirection::SouthEast);
        }
        "minimize" => shell.window.set_minimized(true),
        "maximize" => {
            let maximized = shell.window.is_maximized();
            shell.window.set_maximized(!maximized);
            shell.update_max_icon(!maximized);
        }
        "close" => return true,
        "reload" => {
            if let Some(hotel) = &shell.hotel {
                let _ = hotel.reload();
                footer_set(
                    &shell.footer,
                    lang(settings).t("Reloading…", "Wird neu geladen…"),
                    shell.fps,
                    settings.showfps,
                );
            }
        }
        "settings" => {
            if let Some(current) = sheet.as_ref() {
                current.window.set_focus();
                return false;
            }
            let presence = discord.status();
            let id = next_sheet_id(sheet_seq);
            *sheet = open_sheet(
                target,
                proxy,
                &shell.window,
                settings,
                id,
                SheetKind::Settings,
                "Settings — Habbo.si Client Beta",
                900.0,
                620.0,
                pages::settings_page(settings, &presence),
                None,
                &dir.join("ui"),
            );
        }
        "fullscreen" => toggle_fullscreen(shell, settings),
        "leave-fullscreen" => {
            if shell.fullscreen {
                toggle_fullscreen(shell, settings);
            }
        }
        "devtools" => {
            if settings.devtools {
                if let Some(hotel) = &shell.hotel {
                    hotel.open_devtools();
                }
            }
        }
        "zoom-in" => {
            shell.zoom = (shell.zoom + 0.1).clamp(0.5, 2.0);
            shell.apply_zoom();
        }
        "zoom-out" => {
            shell.zoom = (shell.zoom - 0.1).clamp(0.5, 2.0);
            shell.apply_zoom();
        }
        "zoom-reset" => {
            shell.zoom = 1.0;
            shell.apply_zoom();
        }
        "clear-cache" => {
            let _ = settings::queue_cache_clear(dir);
            footer_set(
                &shell.footer,
                lang(settings).t(
                    "Cache clears on the next start",
                    "Cache wird beim nächsten Start geleert",
                ),
                shell.fps,
                settings.showfps,
            );
        }
        _ => {}
    }
    false
}

fn toggle_fullscreen(shell: &mut Shell, settings: &Settings) {
    shell.fullscreen = !shell.fullscreen;
    shell.window.set_fullscreen(if shell.fullscreen {
        Some(Fullscreen::Borderless(None))
    } else {
        None
    });
    shell.layout();
    if !shell.fullscreen {
        footer_set(
            &shell.footer,
            &shell.status(settings),
            shell.fps,
            settings.showfps,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_sheet_message(
    message: &str,
    sheet: &mut Sheet,
    shell: &Shell,
    settings: &mut Settings,
    dir: &Path,
    proxy: &EventLoopProxy<Action>,
    discord: &mut Discord,
) -> bool {
    // The sheet header doubles as its title bar, so a plain drag message has no JSON.
    if message == "drag" {
        let _ = sheet.window.drag_window();
        return false;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(message) else {
        return false;
    };
    let action = value
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    match action {
        "close" => true,
        "folder" => {
            let _ = open::that(dir);
            false
        }
        "notes" => {
            let _ = proxy.send_event(Action::ShowNotes);
            true
        }
        "open" => {
            let url = value
                .get("url")
                .and_then(|v| v.as_str())
                .map(str::to_owned)
                .or_else(|| sheet.page_url.clone());
            if let Some(url) = url {
                let _ = open::that(url);
            }
            true
        }
        "cache" => {
            let _ = settings::queue_cache_clear(dir);
            true
        }
        "preview" if sheet.kind == SheetKind::Settings => {
            let theme = match value
                .get("theme")
                .and_then(|v| v.as_str())
                .unwrap_or("dark")
                .to_ascii_lowercase()
                .as_str()
            {
                "light" => "light",
                "navy" => "navy",
                _ => "dark",
            };
            if sheet.preview_theme.is_none() {
                sheet.preview_theme = Some(settings.theme.clone());
            }
            if settings.theme != theme {
                settings.theme = theme.into();
                apply_theme(shell, settings, proxy);
            }
            false
        }
        "save" if sheet.kind == SheetKind::Settings => {
            if let Some(form) = value.get("form") {
                if let Ok(form) = serde_json::from_value::<SettingsForm>(form.clone()) {
                    apply_form(form, shell, settings, dir, discord, proxy);
                }
            }
            // Saving keeps the chosen palette; it must not be rolled back on close.
            sheet.preview_theme = None;
            true
        }
        _ => false,
    }
}

fn apply_form(
    form: SettingsForm,
    shell: &Shell,
    settings: &mut Settings,
    dir: &Path,
    discord: &mut Discord,
    proxy: &EventLoopProxy<Action>,
) {
    let previous_discord = settings.discord;
    let previous_id = settings.discordid.clone();

    settings.theme = match form.theme.to_ascii_lowercase().as_str() {
        "light" => "light".into(),
        "navy" => "navy".into(),
        _ => "dark".into(),
    };
    settings.german = form.german;
    settings.channel = if form.channel.eq_ignore_ascii_case("beta") {
        "beta".into()
    } else {
        "stable".into()
    };
    settings.gpu = form.gpu;
    settings.nothrottle = form.nothrottle;
    settings.unlockfps = form.unlockfps;
    settings.showfps = form.showfps;
    settings.devtools = form.devtools;
    settings.discord = form.discord;
    settings.discordid = form.discordid.trim().to_string();
    let _ = settings::save(dir, settings);

    // Rebuild both chrome surfaces so the new theme and language apply immediately.
    apply_theme(shell, settings, proxy);
    footer_set(
        &shell.footer,
        lang(settings).t(
            "Renderer switches apply after a restart",
            "Renderer-Schalter gelten nach einem Neustart",
        ),
        None,
        settings.showfps,
    );

    if settings.discord != previous_discord || settings.discordid != previous_id {
        discord.stop();
        start_presence(discord, settings);
    }

    let _ = shell;
}

/// Rebuilds the client's themed surfaces from the in-memory settings, without saving.
/// Used both after Save and for the settings sheet's live theme preview.
fn apply_theme(shell: &Shell, settings: &Settings, proxy: &EventLoopProxy<Action>) {
    let _ = shell.chrome.load_html(&pages::chrome_page(settings));
    let _ = shell.footer.load_html(&pages::footer_page(settings));
    shell.window.set_theme(Some(window_theme(settings)));
    shell
        .window
        .set_background_color(Some(theme::rgb(&settings.theme)));
    // Reloading replaces the chrome document, so re-apply the live zoom and maximize state
    // once the new page has run instead of leaving it at 100% and a restore icon.
    let refresh = proxy.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(400));
        let _ = refresh.send_event(Action::RefreshChrome);
    });
}

/// Puts the palette back to the value it had when the sheet first previewed.
fn restore_theme(
    sheet: &mut Sheet,
    shell: &Shell,
    settings: &mut Settings,
    proxy: &EventLoopProxy<Action>,
) {
    if let Some(original) = sheet.preview_theme.take() {
        settings.theme = original;
        apply_theme(shell, settings, proxy);
    }
}

/// Drops an open sheet after putting its preview back, so quitting never saves a transient
/// palette the user never confirmed.
fn restore_open_sheet(
    sheet: &mut Option<Sheet>,
    shell: &Shell,
    settings: &mut Settings,
    proxy: &EventLoopProxy<Action>,
) {
    if let Some(mut current) = sheet.take() {
        restore_theme(&mut current, shell, settings, proxy);
    }
}

fn start_presence(discord: &mut Discord, settings: &Settings) {
    match discord::configure(settings) {
        Some(id) => discord.start(&id),
        None => discord.stop(),
    }
}

fn footer_set(footer: &WebView, status: &str, fps: Option<u32>, show_fps: bool) {
    let script = format!(
        "window.setFooter && window.setFooter({}, {}, {});",
        serde_json::to_string(status).unwrap_or_else(|_| "\"\"".into()),
        fps.map(|value| value.to_string())
            .unwrap_or_else(|| "null".into()),
        show_fps
    );
    let _ = footer.evaluate_script(&script);
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        position: LogicalPosition::new(x, y).into(),
        size: LogicalSize::new(width, height).into(),
    }
}

fn next_sheet_id(seq: &mut u64) -> u64 {
    *seq = seq.wrapping_add(1);
    *seq
}

#[allow(clippy::too_many_arguments)]
fn open_sheet(
    target: &EventLoopWindowTarget<Action>,
    proxy: &EventLoopProxy<Action>,
    parent: &Window,
    app: &Settings,
    id: u64,
    kind: SheetKind,
    title: &str,
    width: f64,
    height: f64,
    html: String,
    page_url: Option<String>,
    ui_root: &Path,
) -> Option<Sheet> {
    // Sheets are part of the client, not separate apps: no native frame, no taskbar
    // button, and centred over the window they belong to.
    let scale = parent.scale_factor();
    let parent_pos = parent
        .outer_position()
        .unwrap_or(PhysicalPosition::new(0, 0));
    let parent_size = parent.outer_size();
    let sheet_width = (width * scale).round() as i32;
    let sheet_height = (height * scale).round() as i32;
    let x = parent_pos.x + (parent_size.width as i32 - sheet_width) / 2;
    let y = parent_pos.y + (parent_size.height as i32 - sheet_height) / 2;

    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(LogicalSize::new(width, height))
        .with_position(PhysicalPosition::new(x, y))
        .with_decorations(false)
        .with_resizable(false)
        .with_skip_taskbar(true)
        .with_owner_window(parent.hwnd())
        .with_background_color(theme::rgb(&app.theme))
        .with_theme(Some(window_theme(app)))
        .build(target)
        .ok()?;

    // A frame-less window still carries invisible resize borders, so centre it on the
    // real outer size once Windows has decided them.
    let outer = window.outer_size();
    window.set_outer_position(PhysicalPosition::new(
        parent_pos.x + (parent_size.width as i32 - outer.width as i32) / 2,
        parent_pos.y + (parent_size.height as i32 - outer.height as i32) / 2,
    ));

    let ui_dir = ui_root.join(format!("ui-{id}"));
    let _ = fs::create_dir_all(&ui_dir);
    let mut context = WebContext::new(Some(ui_dir));
    let ipc_proxy = proxy.clone();
    let webview = WebViewBuilder::new_with_web_context(&mut context)
        .with_html(html)
        .with_background_color(theme::rgb(&app.theme))
        .with_theme(web_theme(app))
        .with_devtools(false)
        .with_ipc_handler(move |request| {
            let _ = ipc_proxy.send_event(Action::Sheet(id, request.body().to_string()));
        })
        .build(&window)
        .ok()?;

    Some(Sheet {
        id,
        kind,
        _webview: webview,
        window,
        _context: context,
        page_url,
        preview_theme: None,
    })
}

/// Renders a sheet to a PNG for design review (`--shot-settings`, `--shot-notes`).
fn shot_sheet(
    path: &str,
    title: &str,
    width: f64,
    height: f64,
    html: String,
) -> Result<(), String> {
    let event_loop = EventLoopBuilder::<Action>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(LogicalSize::new(width, height))
        .with_decorations(false)
        .with_position(LogicalPosition::new(-20000.0, -20000.0))
        .build(&event_loop)
        .map_err(|error| format!("could not create the capture window: {error}"))?;

    let ui_dir = std::env::temp_dir().join(format!("habbo-si-shot-{}", std::process::id()));
    let _ = fs::create_dir_all(&ui_dir);
    let mut context = WebContext::new(Some(ui_dir));
    let webview = WebViewBuilder::new_with_web_context(&mut context)
        .with_html(html)
        .with_background_color((11, 20, 32, 255))
        .build(&window)
        .map_err(|error| format!("the capture surface could not start: {error}"))?;

    let target = PathBuf::from(path);
    {
        let proxy = proxy.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(2200));
            let _ = proxy.send_event(Action::Shot);
        });
    }

    let _webview = webview;
    let window_id = window.id();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                window_id: closed,
                event: WindowEvent::CloseRequested,
                ..
            } if closed == window_id => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(Action::Shot) => {
                if let Err(error) = capture::capture_window(&window, &target) {
                    eprintln!("could not capture the sheet: {error}");
                }
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

fn spawn_update_check(proxy: EventLoopProxy<Action>, beta: bool, seen: String) {
    thread::spawn(move || {
        if let Some(info) = updates::latest(beta) {
            if updates::has_update(&info, beta) {
                let _ = proxy.send_event(Action::Update(info));
                return;
            }
        }
        if seen != settings::RELEASE_TAG {
            if let Some(notes) = updates::current_notes() {
                let _ = proxy.send_event(Action::Notes(notes));
            }
        }
    });
}

fn write_update_check(args: &[String], path: &str) {
    let mut settings = settings::load(&settings::data_dir(args));
    if let Some(channel) = settings::value(args, "--channel") {
        settings.channel = if channel.eq_ignore_ascii_case("stable") {
            "stable".into()
        } else {
            "beta".into()
        };
    }
    let beta = settings.beta_channel();
    let info = updates::latest(beta);
    let text = format!(
        "running={}\r\nchannel={}\r\nlatest={}\r\npage={}\r\nnewer={}\r\nerror={}\r\n",
        settings::VERSION,
        if beta { "beta" } else { "stable" },
        info.as_ref().map(|i| i.display()).unwrap_or(""),
        info.as_ref().map(|i| i.page.as_str()).unwrap_or(""),
        info.as_ref()
            .map(|i| updates::has_update(i, beta))
            .unwrap_or(false),
        if info.is_none() {
            "no release or network"
        } else {
            ""
        }
    );
    if let Err(error) = fs::write(path, text) {
        eprintln!("could not write update check: {error}");
    }
}

fn write_release_notes(path: &str) {
    let notes = updates::current_notes();
    let text = format!(
        "tag={}\r\nname={}\r\npage={}\r\nbody_chars={}\r\nerror={}\r\n",
        notes.as_ref().map(|n| n.tag.as_str()).unwrap_or(""),
        notes.as_ref().map(|n| n.name.as_str()).unwrap_or(""),
        notes.as_ref().map(|n| n.page.as_str()).unwrap_or(""),
        notes.as_ref().map(|n| n.body.chars().count()).unwrap_or(0),
        if notes.is_none() {
            "no notes or network"
        } else {
            ""
        }
    );
    if let Err(error) = fs::write(path, text) {
        eprintln!("could not write release notes: {error}");
    }
}

fn write_probe(path: &Path, room_raw: &str, diag: &str) {
    let detected = room::pick_room(&room::unquote(room_raw)).unwrap_or_default();
    let text = format!(
        "detected={detected}\r\nroom_probe={}\r\ndom={}\r\nframes=0\r\n",
        room::unquote(room_raw),
        room::unquote(diag)
    );
    let _ = fs::write(path, text);
}

fn apply_overrides(settings: &mut Settings, args: &[String]) {
    if let Some(channel) = settings::value(args, "--channel") {
        settings.channel = if channel.eq_ignore_ascii_case("stable") {
            "stable".into()
        } else {
            "beta".into()
        };
    }
    if let Some(lang) = settings::value(args, "--lang") {
        settings.german = lang.eq_ignore_ascii_case("de");
    }
    if let Some(theme) = settings::value(args, "--theme") {
        settings.theme = match theme.to_ascii_lowercase().as_str() {
            "light" => "light".into(),
            "navy" => "navy".into(),
            _ => "dark".into(),
        };
    }
}

fn predicted_language(settings: &Settings) -> Lang {
    Lang::new(settings.german)
}

fn lang(settings: &Settings) -> Lang {
    predicted_language(settings)
}

fn window_theme(settings: &Settings) -> tao::window::Theme {
    if settings.theme == "light" {
        tao::window::Theme::Light
    } else {
        tao::window::Theme::Dark
    }
}

fn web_theme(settings: &Settings) -> WebTheme {
    if settings.theme == "light" {
        WebTheme::Light
    } else {
        WebTheme::Dark
    }
}

fn localized_starting(settings: &Settings) -> String {
    lang(settings).t("Starting…", "Startet…").to_string()
}

fn persist(shell: &Shell, settings: &mut Settings, dir: &Path, preview: bool) {
    settings.maximized = shell.window.is_maximized();
    if !settings.maximized && !shell.fullscreen {
        let scale = shell.window.scale_factor();
        let size = shell.window.inner_size();
        settings.width = (size.width as f64 / scale).round() as u32;
        settings.height = (size.height as f64 / scale).round() as u32;
    }
    settings.zoom = shell.zoom;
    if !preview {
        let _ = settings::save(dir, settings);
    }
}

fn base_title() -> String {
    format!("{} {} (Rust)", settings::NAME, settings::VERSION)
}

fn icon() -> Option<Icon> {
    let data = image::load_from_memory(include_bytes!("../../assets/logo.png")).ok()?;
    let rgba = image::imageops::resize(
        &data.to_rgba8(),
        64,
        64,
        image::imageops::FilterType::Lanczos3,
    );
    Icon::from_rgba(rgba.into_raw(), 64, 64).ok()
}

const INIT_SCRIPT: &str = r#"
(function () {
  if (window.top !== window) return;
  var send = function (message) {
    try { window.ipc.postMessage(message); } catch (_) {}
  };

  if (!window.__hscRustKeys) {
    window.__hscRustKeys = true;
    window.addEventListener('keydown', function (event) {
      var key = event.key;
      var command = event.ctrlKey || event.metaKey;
      if (key === 'F5') { event.preventDefault(); send('reload'); }
      else if (key === 'F11') { event.preventDefault(); send('fullscreen'); }
      else if (key === 'Escape') { send('leave-fullscreen'); }
      else if (key === 'F12') { event.preventDefault(); send('devtools'); }
      else if (command && (key === '+' || key === '=')) { event.preventDefault(); send('zoom-in'); }
      else if (command && key === '-') { event.preventDefault(); send('zoom-out'); }
      else if (command && key === '0') { event.preventDefault(); send('zoom-reset'); }
      else if (command && key === ',') { event.preventDefault(); send('settings'); }
    }, true);
    window.addEventListener('load', function () { send('loaded'); });
  }

  if (window.__hscRustFps) return;
  window.__hscRustFps = true;
  var frames = 0;
  var started = performance.now();
  function frame(now) {
    frames++;
    if (now - started >= 1000) {
      try {
        window.ipc.postMessage('fps:' + Math.round(frames * 1000 / (now - started)));
      } catch (_) {}
      frames = 0;
      started = now;
    }
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
})();
"#;
