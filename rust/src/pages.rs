use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use std::sync::OnceLock;

use crate::lang::Lang;
use crate::settings::{BUILD_DATE, NAME, Settings, VERSION};
use crate::theme::{Palette, palette};
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
  color-scheme: dark;
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
.themes button.dark { background: #0b1420; color: #e6effc; }
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

fn head(title: &str, p: &Palette) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{}</title><style>{}\n:root{{{}}}</style></head><body>",
        esc(title),
        CSS,
        p.css()
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
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>{}\n:root{{{}}}\
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
         .zoom{{display:flex;align-items:center;border:1px solid var(--line);border-radius:7px;\
         height:30px;padding:0 2px;color:var(--dim);font-size:10px;}}\
         .zoom button{{height:26px;min-width:22px;padding:0;}}\
         .zoom span{{width:38px;text-align:center;font-size:10px;}}\
         .sep{{width:1px;height:22px;background:var(--line);margin:0 6px;}}\
         </style></head><body><div id=\"bar\">\
         <div class=\"brand\" id=\"drag\"><img src=\"{}\" alt=\"HABBO\">\
         <span class=\"chip\" title=\"{}\">CLIENT</span><span class=\"chip filled\" title=\"{}\">BETA</span></div>\
         <div class=\"actions\">\
         <div class=\"zoom\"><button data-act=\"zoom-out\" title=\"{}\">−</button><span id=\"zoom\">100%</span>\
         <button data-act=\"zoom-in\" title=\"{}\">+</button></div>\
         <button data-act=\"reload\" title=\"{}\"><svg viewBox=\"0 0 24 24\"><path d=\"M20 11a8 8 0 0 0-14.8-4L3 10\"/><path d=\"M3 4v6h6\"/><path d=\"M4 13a8 8 0 0 0 14.8 4L21 14\"/><path d=\"M21 20v-6h-6\"/></svg></button>\
         <button data-act=\"settings\" title=\"{}\"><svg viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"12\" r=\"3\"/><path d=\"M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-4v-.2a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.6-1H3v-4h.2a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-1.6V3h4v.2a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v4H21a1.7 1.7 0 0 0-1.6 1z\"/></svg></button>\
         <div class=\"sep\"></div>\
         <button class=\"win\" data-act=\"minimize\" title=\"{}\"><svg viewBox=\"0 0 24 24\"><path d=\"M5 12h14\"/></svg></button>\
         <button class=\"win\" data-act=\"maximize\" id=\"max\" title=\"{}\"><svg viewBox=\"0 0 24 24\"><rect x=\"5\" y=\"5\" width=\"14\" height=\"14\" rx=\"1\"/></svg></button>\
         <button class=\"win close\" data-act=\"close\" title=\"{}\"><svg viewBox=\"0 0 24 24\"><path d=\"M6 6l12 12M18 6L6 18\"/></svg></button>\
         </div></div><script>",
        CSS,
        p.css(),
        wordmark(),
        esc(l.t("Client version", "Client-Version")),
        esc(l.t("Beta channel build", "Beta-Kanal-Build")),
        esc(l.t("Zoom out", "Verkleinern")),
        esc(l.t("Zoom in", "Vergrößern")),
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
window.setZoom = (value) => { document.getElementById('zoom').textContent = value + '%'; };
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
  else if (command && (event.key === '+' || event.key === '=')) { event.preventDefault(); send('zoom-in'); }
  else if (command && event.key === '-') { event.preventDefault(); send('zoom-out'); }
  else if (command && event.key === '0') { event.preventDefault(); send('zoom-reset'); }
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
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>{}\n:root{{{}}}\
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
        p.css(),
        VERSION
    )
}

pub fn splash_page() -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>\
         html,body{{margin:0;height:100%;background:#0b1420;color:#e6effc;overflow:hidden;\
         font:13px \"Segoe UI\",sans-serif;}}\
         .splash{{height:100%;display:flex;flex-direction:column;align-items:center;justify-content:center;\
         animation:fade 1.35s ease forwards;}}\
         .splash img{{width:310px;height:123px;object-fit:contain;image-rendering:pixelated;}}\
         .rule{{width:40px;height:2px;background:#f6c452;margin:24px 0 13px;}}\
         .hotel{{letter-spacing:2.6px;font-size:9px;font-weight:700;color:#8fa4c0;}}\
         .version{{margin-top:8px;font-size:10px;color:#617691;}}\
         @keyframes fade{{0%{{opacity:0;transform:scale(.985)}}18%{{opacity:1;transform:scale(1)}}82%{{opacity:1}}100%{{opacity:0}}}}\
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
        l.t("Settings — Habbo.si Client Beta", "Einstellungen — Habbo.si Client Beta"),
        &p,
    );
    page.push_str("<div class=\"wrap\">");
    page.push_str(&brand(l.t("SETTINGS", "EINSTELLUNGEN")));
    page.push_str(&format!(
        "<h1>{}</h1><p class=\"sub\">{}</p>",
        l.t("Settings", "Einstellungen"),
        l.t(
            "Renderer options, update channel and client tools.",
            "Renderer-Optionen, Update-Kanal und Client-Werkzeuge."
        )
    ));

    page.push_str(&format!(
        "<div class=\"card\"><h2>{}</h2>\
         <div class=\"row\"><div><div class=\"label\">{}</div>\
         <div class=\"note\">{}</div></div><div class=\"langs\">\
         <button class=\"act\" data-lang=\"en\" aria-pressed=\"{}\">EN</button>\
         <button class=\"act\" data-lang=\"de\" aria-pressed=\"{}\">DE</button></div></div>\
         <div class=\"row\" style=\"display:block;cursor:default\"><div class=\"label\">{}</div>\
         <div class=\"themes\">\
         <button class=\"dark\" data-theme=\"dark\" aria-pressed=\"{}\">{}</button>\
         <button class=\"light\" data-theme=\"light\" aria-pressed=\"{}\">{}</button>\
         <button class=\"navy\" data-theme=\"navy\" aria-pressed=\"{}\">{}</button>\
         </div></div></div>",
        l.t("LANGUAGE & APPEARANCE", "SPRACHE & AUSSEHEN"),
        l.t("Chrome language", "Sprache der Oberfläche"),
        l.t("Applies to the client chrome and sheets.", "Gilt für Client-Oberfläche und Dialoge."),
        if settings.german { "false" } else { "true" },
        if settings.german { "true" } else { "false" },
        l.t("Theme", "Design"),
        if settings.theme == "dark" { "true" } else { "false" },
        l.t("Dark", "Dunkel"),
        if settings.theme == "light" { "true" } else { "false" },
        l.t("Light", "Hell"),
        if settings.theme == "navy" { "true" } else { "false" },
        l.t("Navy", "Marine")
    ));

    page.push_str(&format!(
        "<div class=\"card\"><h2>{}</h2>\
         <label class=\"row\"><div><div class=\"label\">{}</div>\
         <div class=\"note\">{}</div></div><input type=\"checkbox\" id=\"gpu\"{}><span class=\"switch\"></span></label>\
         <label class=\"row\"><div><div class=\"label\">{}</div>\
         <div class=\"note\">{}</div></div><input type=\"checkbox\" id=\"unlockfps\"{}><span class=\"switch\"></span></label>\
         <label class=\"row\"><div><div class=\"label\">{}</div>\
         <div class=\"note\">{}</div></div><input type=\"checkbox\" id=\"nothrottle\"{}><span class=\"switch\"></span></label>\
         <label class=\"row\"><div><div class=\"label\">{}</div>\
         <div class=\"note\">{}</div></div><input type=\"checkbox\" id=\"showfps\"{}><span class=\"switch\"></span></label>\
         <label class=\"row\"><div><div class=\"label\">{}</div>\
         <div class=\"note\">{}</div></div><input type=\"checkbox\" id=\"devtools\"{}><span class=\"switch\"></span></label>\
         <p class=\"sub\" style=\"margin:10px 0 0\">{}</p></div>",
        l.t("PERFORMANCE", "LEISTUNG"),
        l.t("Hardware acceleration", "Hardware-Beschleunigung"),
        l.t("GPU rasterisation for the hotel renderer", "GPU-Rasterisierung für den Hotel-Renderer"),
        checked(settings.gpu),
        l.t("Unlock frame rate", "Bildrate freigeben"),
        l.t("Uncapped renderer for 60–120 FPS", "Renderer ohne 60-FPS-Grenze"),
        checked(settings.unlockfps),
        l.t("Keep running in the background", "Im Hintergrund weiterlaufen"),
        l.t("No timer throttling while the window is covered", "Kein Timer-Drosseln bei verdecktem Fenster"),
        checked(settings.nothrottle),
        l.t("Show FPS", "FPS anzeigen"),
        l.t("Frame rate readout in the footer", "Bildrate in der Fußzeile"),
        checked(settings.showfps),
        l.t("Developer tools", "Entwicklerwerkzeuge"),
        l.t("Open the inspector with F12", "Den Inspector mit F12 öffnen"),
        checked(settings.devtools),
        l.t(
            "Renderer switches take effect after a restart.",
            "Renderer-Schalter werden nach einem Neustart wirksam."
        )
    ));

    page.push_str(&format!(
        "<div class=\"card\"><h2>{}</h2>\
         <label class=\"row\"><div><div class=\"label\">{}</div>\
         <div class=\"note\">{}</div></div><input type=\"checkbox\" id=\"channel\"{}><span class=\"switch\"></span></label>\
         <div class=\"buttons\"><button class=\"act ghost\" id=\"notes\">{}</button></div></div>",
        l.t("UPDATES", "UPDATES"),
        l.t("Beta channel", "Beta-Kanal"),
        l.t("Follow GitHub pre-releases instead of stable releases", "GitHub-Vorabversionen statt stabilen Releases folgen"),
        checked(settings.beta_channel()),
        l.t("Release notes", "Release-Notizen")
    ));

    page.push_str(&format!(
        "<div class=\"card\"><h2>DISCORD RICH PRESENCE</h2>\
         <label class=\"row\"><div><div class=\"label\">{}</div>\
         <div class=\"note\">{}</div></div><input type=\"checkbox\" id=\"discord\"{}><span class=\"switch\"></span></label>\
         <div class=\"fieldrow\"><input class=\"field\" id=\"discordid\" maxlength=\"20\" \
         placeholder=\"{}\" value=\"{}\"></div>\
         <div class=\"status\"><span class=\"dot{}\"></span><span>{}</span></div>\
         <div class=\"buttons\"><button class=\"act ghost\" id=\"portal\">{}</button></div></div>",
        l.t("Discord Rich Presence", "Discord Rich Presence"),
        l.t("Big artwork, client version and your room", "Großes Artwork, Version und dein Raum"),
        checked(settings.discord),
        l.t("Discord application ID (17–20 digits)", "Discord-Application-ID (17–20 Ziffern)"),
        esc(&settings.discordid),
        if presence == "connected" { " on" } else { "" },
        esc(presence),
        l.t("Developer Portal", "Entwickler-Portal")
    ));

    page.push_str(&format!(
        "<div class=\"card\"><h2>{}</h2>\
         <p class=\"sub\" style=\"margin:0 0 10px\">{}</p>\
         <div class=\"buttons\"><button class=\"act ghost\" id=\"folder\">{}</button>\
         <button class=\"act danger\" id=\"cache\">{}</button></div></div>",
        l.t("PROFILE & TOOLS", "PROFIL & WERKZEUGE"),
        l.t(
            "Cached assets and the saved session live in the Rust profile.",
            "Cache und gespeicherte Sitzung liegen im Rust-Profil."
        ),
        l.t("Open profile folder", "Profilordner"),
        l.t("Clear cache", "Cache leeren")
    ));

    page.push_str(&format!(
        "<div class=\"buttons\">\
         <button class=\"act ghost\" id=\"cancel\">{}</button><span class=\"spacer\"></span>\
         <button class=\"act primary\" id=\"save\">{}</button></div>",
        l.t("Cancel", "Abbrechen"),
        l.t("Save", "Speichern")
    ));
    page.push_str(&foot());
    page.push_str("</div><script>");
    page.push_str(&format!(
        "const initialTheme = {}; const initialGerman = {};\n",
        serde_json::to_string(&settings.theme).unwrap_or_else(|_| "\"dark\"".into()),
        settings.german
    ));
    page.push_str(
        r#"
const q = (id) => document.getElementById(id);
const send = (payload) => { try { window.ipc.postMessage(JSON.stringify(payload)); } catch (_) {} };
document.querySelectorAll('[data-theme]').forEach((b) => b.addEventListener('click', () => {
  document.querySelectorAll('[data-theme]').forEach((x) => x.setAttribute('aria-pressed', x === b ? 'true' : 'false'));
  const theme = b.dataset.theme;
  const vars = {
    dark: { bg:'#0b1420', bar:'#0e1928', footer:'#0b1420', card:'#111d2e', card2:'#0e1827', line:'#203047', text:'#e6effc', muted:'#8fa4c0', dim:'#617691', gold:'#f6c452', green:'#56d68b', toast:'#1b2c40', toastText:'#e6effc', rim:'#2a3d56' },
    light: { bg:'#f2f5f9', bar:'#ffffff', footer:'#eef2f7', card:'#ffffff', card2:'#f8fafc', line:'#d5dee9', text:'#142033', muted:'#5b6b80', dim:'#8494a8', gold:'#b47a00', green:'#18834d', toast:'#26374d', toastText:'#ffffff', rim:'#c5d0dd' },
    navy: { bg:'#081426', bar:'#0d1d33', footer:'#081426', card:'#10233c', card2:'#0c1b30', line:'#1d3552', text:'#e6effc', muted:'#91a9c7', dim:'#66819f', gold:'#f6c452', green:'#56d68b', toast:'#1b304b', toastText:'#e6effc', rim:'#2a455f' }
  }[theme];
  const s = document.documentElement.style;
  Object.keys(vars).forEach((k) => s.setProperty('--' + (k === 'toastText' ? 'toast-text' : k), vars[k]));
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

pub fn update_page(info: &UpdateInfo, settings: &Settings) -> String {
    let p = theme(settings);
    let l = lang(settings);
    let mut page = head(l.t("Update available — Habbo.si Client Beta", "Update verfügbar — Habbo.si Client Beta"), &p);
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
    let mut page = head(l.t("What's new — Habbo.si Client Beta", "Was ist neu — Habbo.si Client Beta"), &p);
    page.push_str("<div class=\"wrap\">");
    page.push_str(&brand("WHAT'S NEW"));
    page.push_str(&format!(
        "<h1>{}</h1><p class=\"sub\">{} · {}</p>\
         <pre class=\"notes\">{}</pre>",
        esc(&notes.name),
        l.t("Release notes for the build you are running", "Release-Notizen für deine Version"),
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
    for raw in markdown.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
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
        let Some(mid) = after_start.find("](") else { break };
        let label = &after_start[..mid];
        let after_mid = &after_start[mid + 2..];
        let Some(end) = after_mid.find(')') else { break };
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
        assert_eq!(plain_text("## Hi\n\n- [site](https://x.test)"), "Hi\n\n• site (https://x.test)");
    }

    #[test]
    fn settings_form_has_all_controls() {
        let page = settings_page(&Settings::default(), "off");
        for id in ["gpu", "unlockfps", "nothrottle", "channel", "discord", "discordid", "cache", "save"] {
            assert!(page.contains(&format!("id=\"{id}\"")));
        }
    }
}
