use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use std::sync::OnceLock;

use crate::lang::Lang;
use crate::settings::{BUILD_DATE, BUILD_TIME, NAME, Settings, VERSION};
use crate::theme::{Palette, palette, scheme};
use crate::updates::{ReleaseNotes, UpdateInfo};

fn wordmark() -> &'static str {
    static WORDMARK: OnceLock<String> = OnceLock::new();
    WORDMARK.get_or_init(|| {
        format!(
            "data:image/png;base64,{}",
            STANDARD.encode(include_bytes!("../../assets/habbo-wordmark.png"))
        )
    })
}

fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn checked(on: bool) -> &'static str {
    if on { " checked" } else { "" }
}

const CSS: &str = r#"
:root {
  --bg: #0b1420;
  --bar: #0e1928;
  --footer: #0b1420;
  --card: #111d2e;
  --card2: #0e1827;
  --line: #203047;
  --text: #e6effc;
  --muted: #8fa4c0;
  --dim: #617691;
  --gold: #f6c452;
  --green: #56d68b;
  --toast: #1b2c40;
  --toast-text: #e6effc;
  --rim: #2a3d56;
}
* { box-sizing: border-box; }
html, body { margin: 0; min-height: 100%; background: var(--bg); color: var(--text); }
body { font: 14px/1.45 "Segoe UI", "Segoe UI Variable Text", sans-serif; padding: 0; }
button, input { font: inherit; }
button { color: inherit; }
.wrap { max-width: 980px; margin: 0 auto; padding: 22px 26px 28px; }
.brand { display: flex; align-items: center; gap: 14px; min-height: 46px; }
.brand img { width: 116px; height: 46px; object-fit: contain; }
.chip { display: inline-flex; align-items: center; height: 22px; padding: 0 9px; border: 1px solid var(--gold); border-radius: 6px; color: var(--gold); font-size: 10px; font-weight: 700; letter-spacing: 1.4px; }
.chip.filled { background: var(--gold); color: var(--bg); }
h1 { margin: 6px 0 2px; font-size: 26px; font-weight: 650; letter-spacing: -0.2px; }
h2 { margin: 0 0 12px; color: var(--dim); font-size: 10px; font-weight: 700; letter-spacing: 1.5px; text-transform: uppercase; }
.sub { color: var(--muted); margin: 0 0 18px; }
.card { background: linear-gradient(180deg, var(--card), var(--card2)); border: 1px solid var(--line); border-radius: 12px; padding: 18px 20px; margin: 0 0 14px; }
.row { display: flex; align-items: center; justify-content: space-between; gap: 18px; padding: 9px 0; min-height: 48px; cursor: pointer; }
.row + .row { border-top: 1px solid color-mix(in srgb, var(--line) 65%, transparent); }
.row .label { font-size: 14px; font-weight: 600; }
.row .note { color: var(--muted); font-size: 12px; margin-top: 2px; }
.row input[type=checkbox] { position: absolute; opacity: 0; pointer-events: none; }
.switch { flex: 0 0 auto; width: 40px; height: 22px; border-radius: 999px; background: color-mix(in srgb, var(--line) 75%, var(--bg)); position: relative; transition: background .15s; }
.switch::after { content: ""; position: absolute; width: 16px; height: 16px; border-radius: 50%; background: var(--muted); top: 3px; left: 3px; transition: transform .15s, background .15s; }
.row input:checked + .switch { background: var(--gold); }
.row input:checked + .switch::after { transform: translateX(18px); background: var(--bg); }
.buttons { display: flex; flex-wrap: wrap; gap: 9px; margin-top: 8px; }
button.act { appearance: none; border: 1px solid var(--line); background: color-mix(in srgb, var(--card) 82%, var(--bg)); color: var(--text); border-radius: 7px; height: 36px; padding: 0 16px; font-weight: 600; font-size: 13px; cursor: pointer; }
button.act:hover { border-color: var(--gold); }
button.primary { background: var(--gold); color: var(--bg); border-color: var(--gold); }
button.primary:hover { filter: brightness(1.08); }
button.ghost { background: transparent; }
button.danger { color: #ff9a9a; }
.spacer { flex: 1; }
.field { width: 100%; height: 36px; border: 1px solid var(--line); border-radius: 7px; background: var(--bg); color: var(--text); padding: 0 10px; outline: none; }
.field:focus { border-color: var(--gold); }
.langs { display: flex; gap: 7px; }
.langs button[aria-pressed=true], .themes button[aria-pressed=true] { border-color: var(--gold); box-shadow: inset 0 0 0 1px var(--gold); }
.themes { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; margin: 10px 0 4px; }
.themes button { height: 54px; border: 1px solid var(--line); border-radius: 8px; cursor: pointer; font-weight: 600; }
.themes button.dark { background: #1e1e1e; color: #e8e8e8; }
.themes button.light { background: #f2f5f9; color: #142033; }
.themes button.navy { background: #081426; color: #e6effc; }
.foot { display: flex; justify-content: space-between; align-items: center; color: var(--dim); font-size: 11px; margin-top: 18px; }
pre.notes { margin: 0; padding: 16px 18px; background: color-mix(in srgb, var(--bg) 75%, var(--card)); border: 1px solid var(--line); border-radius: 9px; color: var(--text); white-space: pre-wrap; word-wrap: break-word; font: 13px/1.55 "Segoe UI", "Segoe UI Variable Text", sans-serif; max-height: 440px; overflow: auto; }
.big { font-size: 24px; font-weight: 650; letter-spacing: -.2px; margin: 8px 0 4px; }
.meta { color: var(--muted); font-size: 13px; margin-bottom: 16px; }
a { color: var(--gold); }
.status { display: flex; align-items: center; gap: 8px; color: var(--muted); font-size: 12px; margin-top: 9px; }
.dot { width: 7px; height: 7px; border-radius: 50%; background: var(--dim); }
.dot.on { background: var(--gold); }
::-webkit-scrollbar { width: 10px; }
::-webkit-scrollbar-track { background: var(--bg); }
::-webkit-scrollbar-thumb { background: var(--line); border-radius: 8px; }
::-webkit-scrollbar-thumb:hover { background: var(--rim); }
@media (max-width: 640px) { .wrap { padding: 16px; } .row { align-items: flex-start; } }
"#;

const SHEET_DRAG: &str = r#"<script>
document.addEventListener('mousedown', function (event) {
  if (event.button === 0 && event.clientY < 64 && !event.target.closest('button')) {
    try { window.ipc.postMessage('drag'); } catch (_) {}
  }
});
document.addEventListener('contextmenu', function (event) { event.preventDefault(); });
</script>"#;

fn head(title: &str, p: &Palette, scheme: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{}</title><style>{}\n:root{{color-scheme:{};{}}}</style></head><body>{}",
        esc(title),
        CSS,
        scheme,
        p.css(),
        SHEET_DRAG
    )
}

fn brand(label: &str) -> String {
    format!(
        "<div class=\"brand\"><img src=\"{}\" alt=\"HABBO\"><span class=\"chip\">{}</span></div>",
        wordmark(),
        esc(label)
    )
}

fn foot() -> String {
    format!(
        "<div class=\"foot\"><span>{} · Rust</span><span>v{} · {}</span></div>",
        esc(NAME),
        VERSION,
        BUILD_DATE
    )
}

fn lang(settings: &Settings) -> Lang {
    Lang::new(settings.german)
}

fn theme(settings: &Settings) -> Palette {
    palette(&settings.theme)
}

pub fn chrome_page(settings: &Settings) -> String {
    let p = theme(settings);
    let l = lang(settings);
    let mut page = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>{}\n:root{{color-scheme:{};{}}}\
         html,body{{height:64px;overflow:hidden;margin:0;background:var(--bar);color:var(--text);\
         font:13px/1.2 \"Segoe UI\",sans-serif;user-select:none;}}\
         #bar{{height:64px;display:flex;align-items:center;gap:12px;padding:0 10px 0 22px;\
         border-bottom:1px solid var(--line);background:var(--bar);}}\
         .brand{{display:flex;align-items:center;gap:12px;flex:1;min-width:100px;height:100%;}}\
         .brand img{{width:116px;height:46px;object-fit:contain;pointer-events:none;}}\
         .chip{{display:inline-flex;align-items:center;height:20px;padding:0 8px;border:1px solid var(--gold);\
         border-radius:5px;color:var(--gold);font-size:9px;font-weight:700;letter-spacing:1.2px;}}\
         .chip.filled{{background:var(--gold);color:var(--bg);}}\
         .actions{{display:flex;align-items:center;gap:7px;}}\
         button{{appearance:none;border:0;background:transparent;color:var(--muted);height:30px;\
         min-width:30px;border-radius:6px;display:inline-flex;align-items:center;justify-content:center;\
         cursor:pointer;font:600 12px \"Segoe UI\",sans-serif;padding:0 5px;}}\
         button:hover{{background:color-mix(in srgb,var(--line) 45%,transparent);color:var(--text);}}\
         button.win{{width:42px;border-radius:0;height:64px;}}\
         button.win:hover.close{{background:#c42b1c;color:white;}}\
         svg{{width:16px;height:16px;stroke:currentColor;fill:none;stroke-width:1.8;\
         stroke-linecap:round;stroke-linejoin:round;}}\
         .sep{{width:1px;height:22px;background:var(--line);margin:0 6px;}}\
         </style></head><body><div id=\"bar\">\
         <div class=\"brand\" id=\"drag\"><img src=\"{}\" alt=\"HABBO\">\
         <span class=\"chip\" title=\"{}\">CLIENT</span></div>\
         <div class=\"actions\">\
         <button data-act=\"reload\" title=\"{}\"><svg viewBox=\"0 0 24 24\"><path d=\"M20 11a8 8 0 0 0-14.8-4L3 10\"/><path d=\"M3 4v6h6\"/><path d=\"M4 13a8 8 0 0 0 14.8 4L21 14\"/><path d=\"M21 20v-6h-6\"/></svg></button>\
         <button data-act=\"settings\" title=\"{}\"><svg viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"12\" r=\"3\"/><path d=\"M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-4v-.2a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.6-1H3v-4h.2a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-1.6V3h4v.2a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v4H21a1.7 1.7 0 0 0-1.6 1z\"/></svg></button>\
         <div class=\"sep\"></div>\
         <button class=\"win\" data-act=\"minimize\" title=\"{}\"><svg viewBox=\"0 0 24 24\"><path d=\"M5 12h14\"/></svg></button>\
         <button class=\"win\" data-act=\"maximize\" id=\"max\" title=\"{}\"><svg viewBox=\"0 0 24 24\"><rect x=\"5\" y=\"5\" width=\"14\" height=\"14\" rx=\"1\"/></svg></button>\
         <button class=\"win close\" data-act=\"close\" title=\"{}\"><svg viewBox=\"0 0 24 24\"><path d=\"M6 6l12 12M18 6L6 18\"/></svg></button>\
         </div></div><script>",
        CSS,
        scheme(&settings.theme),
        p.css(),
        wordmark(),
        esc(&format!("{NAME} {VERSION} · {} {BUILD_TIME}", BUILD_DATE)),
        esc(l.t("Reload (F5)", "Neu laden (F5)")),
        esc(l.t("Settings (Ctrl+,)", "Einstellungen (Strg+,)")),
        esc(l.t("Minimize", "Minimieren")),
        esc(l.t("Maximize", "Maximieren")),
        esc(l.t("Close", "Schließen"))
    );
    page.push_str(
        r#"
const send = (message) => { try { window.ipc.postMessage(message); } catch (_) {} };
document.querySelectorAll('[data-act]').forEach((b) => b.addEventListener('click', () => send(b.dataset.act)));
const bar = document.getElementById('bar');
bar.addEventListener('mousedown', (event) => {
  if (event.button !== 0 || event.target.closest('button')) return;
  const bounds = bar.getBoundingClientRect();
  const edge = 6;
  if (event.clientY <= edge) send(event.clientX <= edge ? 'resize-top-left' : event.clientX >= innerWidth - edge ? 'resize-top-right' : 'resize-top');
  else if (event.clientX <= edge) send('resize-left');
  else if (event.clientX >= innerWidth - edge) send('resize-right');
  else if (event.detail === 1) send('drag');
  else send('maximize');
});
bar.addEventListener('dblclick', (event) => {
  if (!event.target.closest('button')) send('maximize');
});
document.addEventListener('contextmenu', (event) => event.preventDefault());
window.setMaximized = (value) => {
  const max = document.getElementById('max');
  max.title = value ? 'Restore' : 'Maximize';
  max.innerHTML = value
    ? '<svg viewBox="0 0 24 24"><rect x="8" y="8" width="11" height="11" rx="1"/><path d="M5 16V5h11"/></svg>'
    : '<svg viewBox="0 0 24 24"><rect x="5" y="5" width="14" height="14" rx="1"/></svg>';
};
document.addEventListener('keydown', (event) => {
  const command = event.ctrlKey || event.metaKey;
  if (event.key === 'F5') { event.preventDefault(); send('reload'); }
  else if (event.key === 'F11') { event.preventDefault(); send('fullscreen'); }
  else if (event.key === 'F12') { event.preventDefault(); send('devtools'); }
  else if (command && event.key === ',') { event.preventDefault(); send('settings'); }
});
"#,
    );
    page.push_str("</script></body></html>");
    page
}

pub fn footer_page(settings: &Settings) -> String {
    let p = theme(settings);
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>{}\n:root{{color-scheme:{};{}}}\
         html,body{{height:30px;overflow:hidden;margin:0;background:var(--footer);color:var(--dim);\
         font:11px/30px \"Segoe UI\",sans-serif;user-select:none;}}\
         #footer{{height:30px;display:flex;align-items:center;gap:14px;padding:0 18px;\
         border-top:1px solid var(--line);}}\
         #status{{flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;}}\
         #fps{{color:var(--muted);}}\
         #version{{color:var(--dim);}}\
         </style></head><body><div id=\"footer\"><span id=\"status\"></span>\
         <span id=\"fps\"></span><span id=\"version\">v{}</span></div><script>\
         window.setFooter = (status, fps, showFps) => {{\
           document.getElementById('status').textContent = status || '';\
           const f = document.getElementById('fps');\
           f.textContent = showFps ? (fps === null || fps === undefined ? '— FPS' : fps + ' FPS') : '';\
           f.style.color = fps !== null && fps >= 60 ? 'var(--green)' : 'var(--muted)';\
         }};\
         </script></body></html>",
        CSS,
        scheme(&settings.theme),
        p.css(),
        VERSION
    )
}

pub fn splash_page() -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>\
         html,body{{margin:0;height:100%;background:#1e1e1e;color:#e8e8e8;overflow:hidden;\
         font:13px \"Segoe UI\",sans-serif;}}\
         .splash{{height:100%;display:flex;flex-direction:column;align-items:center;justify-content:center;}}\
         .splash img{{width:310px;height:123px;object-fit:contain;image-rendering:pixelated;}}\
         .rule{{width:40px;height:2px;background:#f6c452;margin:24px 0 13px;}}\
         .hotel{{letter-spacing:2.6px;font-size:9px;font-weight:700;color:#a6a6a6;}}\
         .version{{margin-top:8px;font-size:10px;color:#7d7d7d;}}\
         </style></head><body><div class=\"splash\"><img src=\"{}\" alt=\"HABBO\">\
         <div class=\"rule\"></div><div class=\"hotel\">HOTEL CLIENT</div>\
         <div class=\"version\">v{}</div></div></body></html>",
        wordmark(),
        VERSION
    )
}

pub fn settings_page(settings: &Settings, presence: &str) -> String {
    let p = theme(settings);
    let l = lang(settings);
    let mut page = head(
        l.t(
            "Settings — Habbo.si Client Beta",
            "Einstellungen — Habbo.si Client Beta",
        ),
        &p,
        scheme(&settings.theme),
    );
    page.push_str(&format!(
        r#"<style>
html,body{{height:100%;overflow:hidden;}}
body{{display:flex;flex-direction:column;background:var(--bg);}}
.head{{flex:0 0 64px;display:flex;align-items:center;gap:12px;padding:0 10px 0 22px;border-bottom:1px solid var(--line);background:var(--bar);}}
.head img{{width:116px;height:46px;object-fit:contain;pointer-events:none;}}
.head .spacer{{flex:1;}}
.langs{{display:inline-flex;height:28px;border:1px solid var(--line);border-radius:7px;overflow:hidden;}}
.langs button{{border:0;background:transparent;color:var(--muted);font:600 11px "Segoe UI",sans-serif;padding:0 11px;cursor:pointer;}}
.langs button[aria-pressed="true"]{{background:var(--gold);color:var(--bg);}}
.winclose{{width:38px;height:30px;border:0;border-radius:6px;background:transparent;color:var(--muted);display:inline-flex;align-items:center;justify-content:center;cursor:pointer;}}
.winclose:hover{{background:#c42b1c;color:#fff;}}
.winclose svg{{width:15px;height:15px;stroke:currentColor;fill:none;stroke-width:1.8;stroke-linecap:round;}}
.body{{flex:1;min-height:0;overflow:auto;padding:16px 22px 12px;display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr);gap:16px 26px;align-content:start;}}
.col{{display:flex;flex-direction:column;gap:14px;min-width:0;}}
section h2{{margin:0 0 6px 2px;font-size:10px;font-weight:700;letter-spacing:1.5px;color:var(--dim);}}
.card{{background:linear-gradient(180deg,var(--card),var(--card2));border:1px solid var(--line);border-radius:10px;padding:2px 14px;}}
.row{{display:flex;align-items:center;justify-content:space-between;gap:16px;min-height:44px;padding:5px 0;cursor:pointer;}}
.row + .row{{border-top:1px solid color-mix(in srgb,var(--line) 62%,transparent);}}
.row .label{{font-size:13px;font-weight:600;line-height:1.25;}}
.row .note{{color:var(--muted);font-size:11px;line-height:1.3;margin-top:1px;}}
.row input[type=checkbox]{{position:absolute;opacity:0;pointer-events:none;}}
.switch{{flex:0 0 auto;width:38px;height:21px;border-radius:999px;background:color-mix(in srgb,var(--line) 75%,var(--bg));position:relative;transition:background .15s;}}
.switch::after{{content:"";position:absolute;width:15px;height:15px;border-radius:50%;background:var(--muted);top:3px;left:3px;transition:transform .15s,background .15s;}}
.row input:checked + .switch{{background:var(--gold);}}
.row input:checked + .switch::after{{transform:translateX(17px);background:var(--bg);}}
.themes{{display:grid;grid-template-columns:repeat(3,1fr);gap:8px;padding:10px 0 12px;}}
.themes button{{height:62px;border:1px solid var(--line);border-radius:8px;cursor:pointer;font:600 12px "Segoe UI",sans-serif;}}
.themes button[aria-pressed="true"]{{border-color:var(--gold);box-shadow:inset 0 0 0 1px var(--gold);}}
.themes button.dark{{background:#1e1e1e;color:#e8e8e8;}}
.themes button.light{{background:#f2f5f9;color:#142033;}}
.themes button.navy{{background:#081426;color:#e6effc;}}
.hint{{margin:6px 2px 0;color:var(--dim);font-size:11px;}}
.fieldrow{{padding:4px 0 2px;}}
.field{{width:100%;height:34px;border:1px solid var(--line);border-radius:7px;background:var(--bg);color:var(--text);padding:0 10px;outline:none;font:12px "Segoe UI",sans-serif;}}
.field:focus{{border-color:var(--gold);}}
.status{{display:flex;align-items:center;gap:7px;color:var(--muted);font-size:11px;padding:6px 0 4px;}}
.dot{{width:6px;height:6px;border-radius:50%;background:var(--dim);}}
.dot.on{{background:var(--gold);}}
.tools{{display:flex;flex-wrap:wrap;gap:8px;padding:8px 0 10px;}}
button.act{{appearance:none;border:1px solid var(--line);background:color-mix(in srgb,var(--card) 82%,var(--bg));color:var(--text);border-radius:7px;height:34px;padding:0 14px;font:600 12px "Segoe UI",sans-serif;cursor:pointer;}}
button.act:hover{{border-color:var(--gold);}}
button.primary{{background:var(--gold);color:var(--bg);border-color:var(--gold);}}
button.ghost{{background:transparent;}}
.foot{{flex:0 0 58px;display:flex;align-items:center;gap:8px;padding:0 22px;border-top:1px solid var(--line);background:var(--bar);}}
.foot .spacer{{flex:1;}}
.foot .stamp{{color:var(--dim);font-size:11px;}}
::-webkit-scrollbar{{width:10px;}}
::-webkit-scrollbar-track{{background:var(--bg);}}
::-webkit-scrollbar-thumb{{background:var(--line);border-radius:8px;}}
@media (max-width:700px){{.body{{grid-template-columns:minmax(0,1fr);}}}}
</style>
<div class="head"><img src="{}" alt="HABBO"><span class="chip">SETTINGS</span>
<span class="spacer"></span>
<div class="langs"><button data-lang="en" aria-pressed="{}">EN</button><button data-lang="de" aria-pressed="{}">DE</button></div>
<button class="winclose" id="close" title="{}"><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg></button></div>
<div class="body">
<div class="col">
<section><h2>{}</h2><div class="card"><div class="themes">
<button class="dark" data-theme="dark" aria-pressed="{}">{}</button>
<button class="light" data-theme="light" aria-pressed="{}">{}</button>
<button class="navy" data-theme="navy" aria-pressed="{}">{}</button>
</div></div></section>
<section><h2>{}</h2><div class="card">
<label class="row"><div><div class="label">{}</div><div class="note">{}</div></div><input type="checkbox" id="gpu"{}><span class="switch"></span></label>
<label class="row"><div><div class="label">{}</div><div class="note">{}</div></div><input type="checkbox" id="unlockfps"{}><span class="switch"></span></label>
<label class="row"><div><div class="label">{}</div><div class="note">{}</div></div><input type="checkbox" id="nothrottle"{}><span class="switch"></span></label>
<label class="row"><div><div class="label">{}</div><div class="note">{}</div></div><input type="checkbox" id="showfps"{}><span class="switch"></span></label>
<label class="row"><div><div class="label">{}</div><div class="note">{}</div></div><input type="checkbox" id="devtools"{}><span class="switch"></span></label>
</div><p class="hint">{}</p></section>
</div>
<div class="col">
<section><h2>DISCORD</h2><div class="card">
<label class="row"><div><div class="label">{}</div><div class="note">{}</div></div><input type="checkbox" id="discord"{}><span class="switch"></span></label>
<div class="fieldrow"><input class="field" id="discordid" maxlength="20" placeholder="{}" value="{}"></div>
<div class="status"><span class="dot{}"></span><span>{}</span></div>
<div class="tools"><button class="act ghost" id="portal">{}</button></div>
</div></section>
<section><h2>{}</h2><div class="card">
<label class="row"><div><div class="label">{}</div><div class="note">{}</div></div><input type="checkbox" id="channel"{}><span class="switch"></span></label>
<div class="tools"><button class="act ghost" id="notes">{}</button></div>
</div></section>
</div>
</div>
<div class="foot">
<button class="act ghost" id="folder">{}</button>
<button class="act ghost" id="cache">{}</button>
<span class="spacer"></span>
<span class="stamp">v{} · {} {}</span>
<span class="spacer"></span>
<button class="act" id="cancel">{}</button>
<button class="act primary" id="save">{}</button>
</div>
<script>"#,
        wordmark(),
        if settings.german { "false" } else { "true" },
        if settings.german { "true" } else { "false" },
        esc(l.t("Close", "Schließen")),
        l.t("APPEARANCE", "ERSCHEINUNGSBILD"),
        if settings.theme == "dark" { "true" } else { "false" },
        l.t("Dark", "Dunkel"),
        if settings.theme == "light" { "true" } else { "false" },
        l.t("Light", "Hell"),
        if settings.theme == "navy" { "true" } else { "false" },
        l.t("Navy", "Marine"),
        l.t("PERFORMANCE", "LEISTUNG"),
        l.t("Hardware acceleration", "Hardware-Beschleunigung"),
        l.t(
            "GPU rasterisation for the hotel renderer",
            "GPU-Rasterisierung für den Hotel-Renderer"
        ),
        checked(settings.gpu),
        l.t("Unlock frame rate", "Bildrate freigeben"),
        l.t("Uncapped renderer for 60–120 FPS", "Renderer ohne 60-FPS-Grenze"),
        checked(settings.unlockfps),
        l.t("Keep running in the background", "Im Hintergrund weiterlaufen"),
        l.t(
            "No timer throttling while the window is covered",
            "Kein Timer-Drosseln bei verdecktem Fenster"
        ),
        checked(settings.nothrottle),
        l.t("Show FPS", "FPS anzeigen"),
        l.t("Frame rate readout in the footer", "Bildrate in der Fußzeile"),
        checked(settings.showfps),
        l.t("Developer tools", "Entwicklerwerkzeuge"),
        l.t("Open the inspector with F12", "Den Inspector mit F12 öffnen"),
        checked(settings.devtools),
        l.t(
            "Restart the client to apply the renderer switches.",
            "Für die Renderer-Schalter den Client neu starten."
        ),
        l.t("Discord Rich Presence", "Discord Rich Presence"),
        l.t(
            "Big artwork, client version and your room — built in",
            "Großes Artwork, Version und dein Raum – integriert"
        ),
        checked(settings.discord),
        l.t(
            "Built-in app (leave empty)",
            "Integrierte App (leer lassen)"
        ),
        esc(&settings.discordid),
        if presence == "connected" { " on" } else { "" },
        esc(&presence_label(l, presence)),
        l.t("Developer Portal", "Entwickler-Portal"),
        l.t("UPDATES", "UPDATES"),
        l.t("Beta channel", "Beta-Kanal"),
        l.t(
            "Follow GitHub pre-releases",
            "GitHub-Vorabversionen folgen"
        ),
        checked(settings.beta_channel()),
        l.t("Release notes", "Release-Notizen"),
        l.t("Open profile folder", "Profilordner"),
        l.t("Clear cache", "Cache leeren"),
        VERSION,
        BUILD_DATE,
        BUILD_TIME,
        l.t("Cancel", "Abbrechen"),
        l.t("Save", "Speichern")
    ));
    page.push_str(&format!(
        "const initialTheme = {}; const initialGerman = {};\n\
         const themeVars = {{ dark: {}, light: {}, navy: {} }};\n",
        serde_json::to_string(&settings.theme).unwrap_or_else(|_| "\"dark\"".into()),
        settings.german,
        palette("dark").js(),
        palette("light").js(),
        palette("navy").js()
    ));
    page.push_str(
        r#"
const q = (id) => document.getElementById(id);
const send = (payload) => { try { window.ipc.postMessage(JSON.stringify(payload)); } catch (_) {} };
document.querySelectorAll('[data-theme]').forEach((b) => b.addEventListener('click', () => {
  document.querySelectorAll('[data-theme]').forEach((x) => x.setAttribute('aria-pressed', x === b ? 'true' : 'false'));
  const vars = themeVars[b.dataset.theme];
  const s = document.documentElement.style;
  Object.keys(vars).forEach((k) => s.setProperty('--' + (k === 'toastText' ? 'toast-text' : k), vars[k]));
  s.colorScheme = b.dataset.theme === 'light' ? 'light' : 'dark';
  send({ action: 'preview', theme: b.dataset.theme });
}));
let german = initialGerman;
document.querySelectorAll('[data-lang]').forEach((b) => b.addEventListener('click', () => {
  german = b.dataset.lang === 'de';
  document.querySelectorAll('[data-lang]').forEach((x) => x.setAttribute('aria-pressed', x === b ? 'true' : 'false'));
}));
function save() {
  send({ action: 'save', form: {
    theme: document.querySelector('[data-theme][aria-pressed="true"]')?.dataset.theme || initialTheme,
    german: german,
    channel: q('channel').checked ? 'beta' : 'stable',
    gpu: q('gpu').checked,
    nothrottle: q('nothrottle').checked,
    unlockfps: q('unlockfps').checked,
    showfps: q('showfps').checked,
    devtools: q('devtools').checked,
    discord: q('discord').checked,
    discordid: q('discordid').value.trim()
  }});
}
q('save').addEventListener('click', save);
q('cancel').addEventListener('click', () => send({ action: 'close' }));
q('close').addEventListener('click', () => send({ action: 'close' }));
q('folder').addEventListener('click', () => send({ action: 'folder' }));
q('notes').addEventListener('click', () => send({ action: 'notes' }));
q('portal').addEventListener('click', () => send({ action: 'open', url: 'https://discord.com/developers/applications' }));
q('cache').addEventListener('click', () => {
  if (confirm('Clear cached assets and the saved hotel session on the next start?')) send({ action: 'cache' });
});
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') send({ action: 'close' });
  if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) save();
});
"#,
    );
    page.push_str("</script></body></html>");
    page
}

fn presence_label(l: Lang, presence: &str) -> &'static str {
    match presence {
        "connected" => l.t("Connected", "Verbunden"),
        "connecting" => l.t("Connecting…", "Verbinde…"),
        "no-discord" => l.t("Discord is not running", "Discord läuft nicht"),
        "error" => l.t(
            "Discord refused the connection",
            "Discord hat die Verbindung abgelehnt",
        ),
        _ => l.t("Off", "Aus"),
    }
}
pub fn update_page(info: &UpdateInfo, settings: &Settings) -> String {
    let p = theme(settings);
    let l = lang(settings);
    let mut page = head(
        l.t(
            "Update available — Habbo.si Client Beta",
            "Update verfügbar — Habbo.si Client Beta",
        ),
        &p,
        scheme(&settings.theme),
    );
    page.push_str("<div class=\"wrap\">");
    page.push_str(&brand("UPDATE"));
    page.push_str(&format!(
        "<h1>{}</h1><p class=\"sub\">{}</p>\
         <div class=\"card\"><div class=\"big\">{}</div>\
         <div class=\"meta\">{}: v{} · {}: {}</div>\
         <p class=\"sub\" style=\"margin:0\">{}</p></div>",
        l.t("A newer build is available", "Eine neuere Version ist verfügbar"),
        l.t("This client follows the selected GitHub release channel.", "Dieser Client folgt dem gewählten GitHub-Release-Kanal."),
        esc(info.display()),
        l.t("Installed", "Installiert"),
        VERSION,
        l.t("Latest", "Neueste"),
        esc(&info.latest),
        l.t(
            "The client never downloads or installs anything. Open the release page to read the notes and download it yourself.",
            "Der Client lädt oder installiert nichts. Öffne die Release-Seite, um die Notizen zu lesen und selbst herunterzuladen."
        )
    ));
    page.push_str(&format!(
        "<div class=\"buttons\"><button class=\"act ghost\" id=\"later\">{}</button>\
         <span class=\"spacer\"></span><button class=\"act primary\" id=\"open\">{}</button></div>",
        l.t("Later", "Später"),
        l.t("Open release", "Release öffnen")
    ));
    page.push_str(&foot());
    page.push_str("</div><script>const url = ");
    page.push_str(&serde_json::to_string(&info.page).unwrap_or_else(|_| "\"\"".into()));
    page.push_str(
        r#";
function send(payload) { try { window.ipc.postMessage(JSON.stringify(payload)); } catch (_) {} }
document.getElementById('later').onclick = () => send({ action: 'close' });
document.getElementById('open').onclick = () => send({ action: 'open', url });
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') send({ action: 'close' });
  if (e.key === 'Enter') send({ action: 'open', url });
});
"#,
    );
    page.push_str("</script></body></html>");
    page
}

pub fn notes_page(notes: &ReleaseNotes, settings: &Settings) -> String {
    let p = theme(settings);
    let l = lang(settings);
    let mut page = head(
        l.t(
            "What's new — Habbo.si Client Beta",
            "Was ist neu — Habbo.si Client Beta",
        ),
        &p,
        scheme(&settings.theme),
    );
    page.push_str("<div class=\"wrap\">");
    page.push_str(&brand("WHAT'S NEW"));
    page.push_str(&format!(
        "<h1>{}</h1><p class=\"sub\">{} · {}</p>\
         <pre class=\"notes\">{}</pre>",
        esc(&notes.name),
        l.t(
            "Release notes for the build you are running",
            "Release-Notizen für deine Version"
        ),
        esc(&notes.tag),
        esc(&plain_text(&notes.body))
    ));
    page.push_str(&format!(
        "<div class=\"buttons\" style=\"margin-top:14px\">\
         <button class=\"act ghost\" id=\"close\">{}</button><span class=\"spacer\"></span>\
         <button class=\"act primary\" id=\"open\">{}</button></div>",
        l.t("Close", "Schließen"),
        l.t("Open release", "Release öffnen")
    ));
    page.push_str(&foot());
    page.push_str("</div><script>const url = ");
    page.push_str(&serde_json::to_string(&notes.page).unwrap_or_else(|_| "\"\"".into()));
    page.push_str(
        r#";
function send(payload) { try { window.ipc.postMessage(JSON.stringify(payload)); } catch (_) {} }
document.getElementById('close').onclick = () => send({ action: 'close' });
document.getElementById('open').onclick = () => send({ action: 'open', url });
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') send({ action: 'close' });
});
"#,
    );
    page.push_str("</script></body></html>");
    page
}

/// A deliberately small Markdown-to-text pass, matching the C# sheet's readable output
/// without pulling a Markdown parser into the port.
fn plain_text(markdown: &str) -> String {
    let mut out = String::new();
    let mut blank = true;
    for raw in markdown
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .split('\n')
    {
        let mut line = raw.trim().to_string();
        if line == "---" || line == "***" || line == "___" {
            line.clear();
        }
        if line.is_empty() {
            if !blank {
                out.push('\n');
                blank = true;
            }
            continue;
        }

        let bullet = line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ");
        if bullet {
            line = line[2..].to_string();
        }
        let hashes = line.chars().take_while(|c| *c == '#').count();
        if hashes > 0 && hashes <= 6 && line.as_bytes().get(hashes) == Some(&b' ') {
            line = line[hashes + 1..].to_string();
        }
        if let Some(rest) = line.strip_prefix("> ") {
            line = rest.to_string();
        }

        line = replace_links(&line)
            .replace("**", "")
            .replace("__", "")
            .replace('`', "")
            .replace('*', "");
        if bullet {
            line = format!("• {line}");
        }
        out.push_str(&line);
        out.push('\n');
        blank = false;
    }
    out.trim().to_string()
}

fn replace_links(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        let after_start = &rest[start + 1..];
        let Some(mid) = after_start.find("](") else {
            break;
        };
        let label = &after_start[..mid];
        let after_mid = &after_start[mid + 2..];
        let Some(end) = after_mid.find(')') else {
            break;
        };
        out.push_str(&rest[..start]);
        out.push_str(label);
        out.push_str(" (");
        out.push_str(&after_mid[..end]);
        out.push(')');
        rest = &after_mid[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn escapes_release_notes() {
        let page = notes_page(
            &ReleaseNotes {
                tag: "1.2.0-beta.3".into(),
                name: "Test <script>".into(),
                body: "- **Hello** <b>x</b>".into(),
                page: "https://example.com".into(),
            },
            &Settings::default(),
        );
        assert!(page.contains("&lt;script&gt;"));
        assert!(!page.contains("<script>Test"));
        assert!(page.contains("• Hello &lt;b&gt;x&lt;/b&gt;"));
    }

    #[test]
    fn formats_links_and_headings() {
        assert_eq!(
            plain_text("## Hi\n\n- [site](https://x.test)"),
            "Hi\n\n• site (https://x.test)"
        );
    }

    #[test]
    fn header_shows_the_build_stamp() {
        let page = chrome_page(&Settings::default());
        assert!(page.contains(VERSION));
        assert!(page.contains(BUILD_DATE));
        assert!(page.contains(BUILD_TIME));
        assert!(page.contains(">CLIENT</span>"));
        assert!(!page.contains(">BETA</span>"));
    }

    #[test]
    fn light_theme_switches_native_controls() {
        let mut settings = Settings::default();
        settings.theme = "light".into();
        let chrome = chrome_page(&settings);
        assert!(chrome.contains("color-scheme:light"));
        assert!(!chrome.contains("color-scheme:dark"));
        assert!(settings_page(&settings, "off").contains("color-scheme:light"));
    }

    #[test]
    fn theme_tiles_preview_on_the_client() {
        let page = settings_page(&Settings::default(), "off");
        assert!(page.contains("action: 'preview'"));
    }

    #[test]
    fn window_backdrop_matches_the_theme() {
        assert_eq!(crate::theme::rgb("light"), (242, 245, 249, 255));
        assert_eq!(crate::theme::rgb("navy"), (8, 20, 38, 255));
        assert_eq!(crate::theme::rgb("dark"), (30, 30, 30, 255));
    }

    #[test]
    fn dark_palette_uses_the_neutral_grey() {
        let page = settings_page(&Settings::default(), "off");
        assert!(page.contains("bg:'#1e1e1e'"));
        assert!(chrome_page(&Settings::default()).contains("--bg:#1e1e1e"));
    }

    #[test]
    fn settings_form_has_all_controls() {
        let page = settings_page(&Settings::default(), "off");
        for id in [
            "gpu",
            "unlockfps",
            "nothrottle",
            "channel",
            "discord",
            "discordid",
            "cache",
            "save",
        ] {
            assert!(page.contains(&format!("id=\"{id}\"")));
        }
    }
}
