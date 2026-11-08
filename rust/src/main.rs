#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pages;
mod settings;
mod updates;

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Fullscreen, Icon, Window, WindowBuilder};
use wry::{
    Theme as WebTheme, WebContext, WebView, WebViewBuilder, WebViewBuilderExtWindows,
};

use settings::{Settings, SettingsForm};

#[derive(Debug)]
enum Action {
    Ipc(String),
    SheetMsg(u64, String),
    Update(updates::UpdateInfo),
    Notes(updates::ReleaseNotes),
    ShowNotes,
    Smoke,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SheetKind {
    Settings,
    Notes,
    Update,
}

struct Client {
    webview: WebView,
    window: Window,
    _context: WebContext,
}

struct Sheet {
    id: u64,
    kind: SheetKind,
    _webview: WebView,
    window: Window,
    _context: WebContext,
    page_url: Option<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

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
    let mut settings = settings::load(&dir);
    if let Some(channel) = settings::value(&args, "--channel") {
        settings.channel = if channel.eq_ignore_ascii_case("stable") {
            "stable".into()
        } else {
            "beta".into()
        };
    }

    let event_loop = EventLoopBuilder::<Action>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let title = base_title();
    let window = WindowBuilder::new()
        .with_title(&title)
        .with_inner_size(LogicalSize::new(
            settings.width.clamp(980, 10000) as f64,
            settings.height.clamp(620, 10000) as f64,
        ))
        .with_min_inner_size(LogicalSize::new(980.0, 620.0))
        .with_maximized(settings.maximized)
        .with_window_icon(icon())
        .with_theme(Some(tao::window::Theme::Dark))
        .build(&event_loop)
        .map_err(|e| format!("could not create the window: {e}"))?;

    settings::ensure(&dir);
    let mut context = WebContext::new(Some(dir.join("WebView2")));
    let ipc_proxy = proxy.clone();
    let webview = WebViewBuilder::new_with_web_context(&mut context)
        .with_url(settings::CLIENT_URL)
        .with_background_color((11, 20, 32, 255))
        .with_devtools(true)
        .with_clipboard(true)
        .with_theme(WebTheme::Dark)
        .with_hotkeys_zoom(false)
        .with_initialization_script(INIT_SCRIPT)
        .with_additional_browser_args(settings.browser_args())
        .with_ipc_handler(move |request| {
            let _ = ipc_proxy.send_event(Action::Ipc(request.body().to_string()));
        })
        .build(&window)
        .map_err(|e| format!("WebView2 could not start: {e}"))?;

    let _ = webview.zoom(settings.zoom);
    let client = Client {
        webview,
        window,
        _context: context,
    };
    let main_id = client.window.id();
    let mut sheet: Option<Sheet> = None;
    let mut sheet_seq = 0u64;
    let mut fullscreen = false;
    let mut fps: Option<u32> = None;

    // Give the first paint a moment to finish before a smoke run reports the URL.
    let smoke_path = settings::value(&args, "--smoke").map(PathBuf::from);
    if smoke_path.is_some() {
        let proxy = proxy.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(8));
            let _ = proxy.send_event(Action::Smoke);
        });
    } else if !settings::has(&args, "--no-update-check") {
        spawn_update_check(
            proxy.clone(),
            settings.beta_channel(),
            settings.seenrelease.clone(),
        );
    }

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent { window_id, event, .. } => {
                if window_id == main_id {
                    match event {
                        WindowEvent::CloseRequested => {
                            settings.maximized = client.window.is_maximized();
                            if !settings.maximized && !fullscreen {
                                let size = client.window.inner_size();
                                settings.width = size.width.max(1);
                                settings.height = size.height.max(1);
                            }
                            let _ = settings::save(&dir, &settings);
                            *control_flow = ControlFlow::Exit;
                        }
                        WindowEvent::Resized(size) => {
                            if !fullscreen && !client.window.is_maximized() {
                                settings.width = size.width.max(1);
                                settings.height = size.height.max(1);
                            }
                        }
                        _ => {}
                    }
                    return;
                }

                let close_sheet = sheet
                    .as_ref()
                    .map(|s| {
                        s.window.id() == window_id
                            && matches!(event, WindowEvent::CloseRequested)
                    })
                    .unwrap_or(false);
                if close_sheet {
                    sheet = None;
                }
            }
            Event::UserEvent(action) => match action {
                Action::Ipc(message) => {
                    handle_ipc(
                        &message,
                        &client,
                        &mut settings,
                        &dir,
                        &proxy,
                        target,
                        &mut sheet,
                        &mut sheet_seq,
                        &mut fullscreen,
                        &mut fps,
                    );
                }
                Action::SheetMsg(id, message) => {
                    let close = sheet
                        .as_ref()
                        .filter(|s| s.id == id)
                        .map(|current| {
                            handle_sheet_message(
                                &message,
                                current,
                                &client,
                                &mut settings,
                                &dir,
                                &proxy,
                            )
                        })
                        .unwrap_or(false);
                    if close {
                        sheet = None;
                    }
                }
                Action::Update(info) => {
                    if sheet.is_none() {
                        let id = next_sheet_id(&mut sheet_seq);
                        sheet = open_sheet(
                            target,
                            &proxy,
                            id,
                            SheetKind::Update,
                            "Update available — Habbo.si Client Beta",
                            560.0,
                            360.0,
                            pages::update_page(&info),
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
                            id,
                            SheetKind::Notes,
                            "What's new — Habbo.si Client Beta",
                            700.0,
                            640.0,
                            pages::notes_page(&notes),
                            Some(notes.page.clone()),
                            &dir.join("ui"),
                        );
                        if opened.is_some() {
                            // The sheet is shown once per profile. If it could not be
                            // created, leave the marker alone so the next launch retries.
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
                Action::Smoke => {
                    if let Some(path) = &smoke_path {
                        let url = client.webview.url().unwrap_or_default();
                        let ok = url.starts_with("http://") || url.starts_with("https://");
                        let text = format!(
                            "ok={}\r\nurl={url}\r\nversion={}\r\nchannel={}\r\n",
                            if ok { 1 } else { 0 },
                            settings::VERSION,
                            settings.channel
                        );
                        let _ = fs::write(path, text);
                    }
                    *control_flow = ControlFlow::Exit;
                }
            },
            _ => {}
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn handle_ipc(
    message: &str,
    client: &Client,
    settings: &mut Settings,
    dir: &Path,
    proxy: &EventLoopProxy<Action>,
    target: &EventLoopWindowTarget<Action>,
    sheet: &mut Option<Sheet>,
    sheet_seq: &mut u64,
    fullscreen: &mut bool,
    fps: &mut Option<u32>,
) {
    if let Some(value) = message.strip_prefix("fps:") {
        if let Ok(value) = value.parse::<u32>() {
            *fps = Some(value);
            set_window_title(client, settings, *fps);
        }
        return;
    }

    match message {
        "reload" => {
            let _ = client.webview.reload();
        }
        "settings" => {
            if let Some(current) = sheet.as_ref() {
                current.window.set_focus();
                return;
            }
            let id = next_sheet_id(sheet_seq);
            *sheet = open_sheet(
                target,
                proxy,
                id,
                SheetKind::Settings,
                "Settings — Habbo.si Client Beta",
                900.0,
                700.0,
                pages::settings_page(settings),
                None,
                &dir.join("ui"),
            );
        }
        "fullscreen" => {
            *fullscreen = !*fullscreen;
            client.window.set_fullscreen(if *fullscreen {
                Some(Fullscreen::Borderless(None))
            } else {
                None
            });
        }
        "leave-fullscreen" => {
            if *fullscreen {
                *fullscreen = false;
                client.window.set_fullscreen(None);
            }
        }
        "devtools" => {
            if settings.devtools {
                client.webview.open_devtools();
            }
        }
        "zoom-in" => zoom(client, settings, 0.1),
        "zoom-out" => zoom(client, settings, -0.1),
        "zoom-reset" => {
            settings.zoom = 1.0;
            let _ = client.webview.zoom(settings.zoom);
        }
        _ => {}
    }
}

fn zoom(client: &Client, settings: &mut Settings, delta: f64) {
    settings.zoom = (settings.zoom + delta).clamp(0.5, 2.0);
    let _ = client.webview.zoom(settings.zoom);
}

fn handle_sheet_message(
    message: &str,
    sheet: &Sheet,
    client: &Client,
    settings: &mut Settings,
    dir: &Path,
    proxy: &EventLoopProxy<Action>,
) -> bool {
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
        "save" if sheet.kind == SheetKind::Settings => {
            if let Some(form) = value.get("form") {
                if let Ok(form) = serde_json::from_value::<SettingsForm>(form.clone()) {
                    let channel_changed = settings.channel
                        != if form.channel.eq_ignore_ascii_case("beta") {
                            "beta"
                        } else {
                            "stable"
                        };
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
                    let _ = settings::save(dir, settings);
                    set_window_title(client, settings, None);
                    if channel_changed {
                        spawn_update_check(
                            proxy.clone(),
                            settings.beta_channel(),
                            settings.seenrelease.clone(),
                        );
                    }
                }
            }
            true
        }
        _ => false,
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
    id: u64,
    kind: SheetKind,
    title: &str,
    width: f64,
    height: f64,
    html: String,
    page_url: Option<String>,
    ui_root: &Path,
) -> Option<Sheet> {
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(LogicalSize::new(width, height))
        .with_min_inner_size(LogicalSize::new(520.0, 260.0))
        .with_resizable(true)
        .with_theme(Some(tao::window::Theme::Dark))
        .build(target)
        .ok()?;

    let ui_dir = ui_root.join(format!("ui-{id}"));
    let _ = fs::create_dir_all(&ui_dir);
    let mut context = WebContext::new(Some(ui_dir));
    let ipc_proxy = proxy.clone();
    let webview = WebViewBuilder::new_with_web_context(&mut context)
        .with_html(html)
        .with_background_color((11, 20, 32, 255))
        .with_devtools(false)
        .with_ipc_handler(move |request| {
            let _ = ipc_proxy.send_event(Action::SheetMsg(id, request.body().to_string()));
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
    })
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
        if info.is_none() { "no release or network" } else { "" }
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
        if notes.is_none() { "no notes or network" } else { "" }
    );
    if let Err(error) = fs::write(path, text) {
        eprintln!("could not write release notes: {error}");
    }
}

fn base_title() -> String {
    format!("{} {} (Rust)", settings::NAME, settings::VERSION)
}

fn set_window_title(client: &Client, settings: &Settings, fps: Option<u32>) {
    let title = if settings.showfps {
        match fps {
            Some(fps) => format!("{} — {fps} FPS", base_title()),
            None => base_title(),
        }
    } else {
        base_title()
    };
    client.window.set_title(&title);
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
