use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use std::sync::OnceLock;

use crate::settings::{BUILD_DATE, NAME, VERSION};
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
  --card: #111d2e;
  --card2: #0e1827;
  --line: #203047;
  --text: #e6effc;
  --muted: #8fa4c0;
  --dim: #617691;
  --gold: #f6c452;
  --gold-dim: #493d20;
}
* { box-sizing: border-box; }
html, body { margin: 0; min-height: 100%; background: var(--bg); color: var(--text); }
body {
  font: 14px/1.45 "Segoe UI", "Segoe UI Variable Text", sans-serif;
  padding: 0;
}
.wrap { max-width: 980px; margin: 0 auto; padding: 22px 26px 28px; }
.brand { display: flex; align-items: center; gap: 14px; min-height: 46px; }
.brand img { width: 116px; height: 46px; object-fit: contain; }
.chip {
  display: inline-flex; align-items: center; height: 22px; padding: 0 9px;
  border: 1px solid var(--gold); border-radius: 6px; color: var(--gold);
  font-size: 10px; font-weight: 700; letter-spacing: 1.4px;
}
h1 { margin: 6px 0 2px; font-size: 26px; font-weight: 650; letter-spacing: -0.2px; }
.sub { color: var(--muted); margin: 0 0 18px; }
.rule { height: 1px; background: var(--line); margin: 18px 0; }
.card {
  background: linear-gradient(180deg, var(--card), var(--card2));
  border: 1px solid var(--line); border-radius: 12px; padding: 18px 20px;
  margin: 0 0 14px;
}
.card h2 {
  margin: 0 0 12px; color: var(--dim); font-size: 10px; font-weight: 700;
  letter-spacing: 1.5px; text-transform: uppercase;
}
.row {
  display: flex; align-items: center; justify-content: space-between;
  gap: 18px; padding: 9px 0; min-height: 48px; cursor: pointer;
}
.row + .row { border-top: 1px solid rgba(32,48,71,.55); }
.row .label { font-size: 14px; font-weight: 600; }
.row .note { color: var(--muted); font-size: 12px; margin-top: 2px; }
.row input { position: absolute; opacity: 0; pointer-events: none; }
.switch {
  flex: 0 0 auto; width: 40px; height: 22px; border-radius: 999px;
  background: #26374d; position: relative; transition: background .15s;
}
.switch::after {
  content: ""; position: absolute; width: 16px; height: 16px; border-radius: 50%;
  background: #9aaec6; top: 3px; left: 3px; transition: transform .15s, background .15s;
}
.row input:checked + .switch { background: var(--gold); }
.row input:checked + .switch::after { transform: translateX(18px); background: #1a2432; }
.buttons { display: flex; flex-wrap: wrap; gap: 9px; margin-top: 8px; }
button {
  appearance: none; border: 1px solid var(--line); background: #152338;
  color: var(--text); border-radius: 7px; height: 36px; padding: 0 16px;
  font: 600 13px "Segoe UI", sans-serif; cursor: pointer;
}
button:hover { border-color: #3b5574; background: #1a2a41; }
button.primary { background: var(--gold); color: #132033; border-color: var(--gold); }
button.primary:hover { background: #ffd56b; }
button.ghost { background: transparent; }
.spacer { flex: 1; }
.foot {
  display: flex; justify-content: space-between; align-items: center;
  color: var(--dim); font-size: 11px; margin-top: 18px;
}
pre.notes {
  margin: 0; padding: 16px 18px; background: #0b1523; border: 1px solid var(--line);
  border-radius: 9px; color: var(--text); white-space: pre-wrap; word-wrap: break-word;
  font: 13px/1.55 "Segoe UI", "Segoe UI Variable Text", sans-serif;
  max-height: 420px; overflow: auto;
}
.big { font-size: 24px; font-weight: 650; letter-spacing: -.2px; margin: 8px 0 4px; }
.meta { color: var(--muted); font-size: 13px; margin-bottom: 16px; }
a { color: var(--gold); }
::-webkit-scrollbar { width: 10px; }
::-webkit-scrollbar-track { background: #0b1420; }
::-webkit-scrollbar-thumb { background: #2a3d56; border-radius: 8px; }
::-webkit-scrollbar-thumb:hover { background: #3a5270; }
@media (max-width: 640px) {
  .wrap { padding: 16px; }
  .row { align-items: flex-start; }
}
"#;

fn head(title: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{}</title><style>{}</style></head><body>",
        esc(title),
        CSS
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
        "<div class=\"foot\"><span>{} · Rust port</span><span>v{} · built {}</span></div>",
        esc(NAME),
        VERSION,
        BUILD_DATE
    )
}

pub fn settings_page(settings: &crate::settings::Settings) -> String {
    let mut page = head("Settings — Habbo.si Client Beta");
    page.push_str("<div class=\"wrap\">");
    page.push_str(&brand("SETTINGS"));
    page.push_str("<h1>Settings</h1><p class=\"sub\">Renderer options, update channel and client tools.</p>");

    page.push_str("<div class=\"card\"><h2>Updates</h2>");
    page.push_str(&format!(
        "<label class=\"row\"><div><div class=\"label\">Beta channel</div>\
         <div class=\"note\">Follow GitHub pre-releases instead of stable releases</div></div>\
         <input type=\"checkbox\" id=\"channel\"{}><span class=\"switch\"></span></label>",
        checked(settings.beta_channel())
    ));
    page.push_str(
        "<div class=\"buttons\" style=\"margin-top:8px\">\
         <button id=\"notes\" class=\"ghost\">Release notes</button></div>",
    );
    page.push_str("</div>");

    page.push_str("<div class=\"card\"><h2>Performance</h2>");
    page.push_str(&format!(
        "<label class=\"row\"><div><div class=\"label\">Hardware acceleration</div>\
         <div class=\"note\">GPU rasterisation and zero-copy for the hotel renderer</div></div>\
         <input type=\"checkbox\" id=\"gpu\"{}><span class=\"switch\"></span></label>\
         <label class=\"row\"><div><div class=\"label\">Unlock frame rate</div>\
         <div class=\"note\">Remove Chromium's refresh-rate ceiling</div></div>\
         <input type=\"checkbox\" id=\"unlockfps\"{}><span class=\"switch\"></span></label>\
         <label class=\"row\"><div><div class=\"label\">Keep running in the background</div>\
         <div class=\"note\">Do not throttle timers when the window is covered</div></div>\
         <input type=\"checkbox\" id=\"nothrottle\"{}><span class=\"switch\"></span></label>",
        checked(settings.gpu),
        checked(settings.unlockfps),
        checked(settings.nothrottle)
    ));
    page.push_str("</div>");

    page.push_str("<div class=\"card\"><h2>Client</h2>");
    page.push_str(&format!(
        "<label class=\"row\"><div><div class=\"label\">Show FPS</div>\
         <div class=\"note\">Frame-rate readout in the window title</div></div>\
         <input type=\"checkbox\" id=\"showfps\"{}><span class=\"switch\"></span></label>\
         <label class=\"row\"><div><div class=\"label\">Developer tools</div>\
         <div class=\"note\">Allow the inspector to open with F12</div></div>\
         <input type=\"checkbox\" id=\"devtools\"{}><span class=\"switch\"></span></label>",
        checked(settings.showfps),
        checked(settings.devtools)
    ));
    page.push_str("</div>");

    page.push_str(
        "<p class=\"sub\" style=\"margin:2px 0 14px\">Renderer switches take effect after a restart. \
         Zoom is saved with the window.</p>\
         <div class=\"buttons\">\
           <button id=\"folder\" class=\"ghost\">Open profile folder</button>\
           <span class=\"spacer\"></span>\
           <button id=\"cancel\">Cancel</button>\
           <button id=\"save\" class=\"primary\">Save</button>\
         </div>",
    );
    page.push_str(&foot());
    page.push_str("</div><script>");
    page.push_str(
        r#"
const q = (id) => document.getElementById(id);
function send(payload) {
  try { window.ipc.postMessage(JSON.stringify(payload)); } catch (_) {}
}
function save() {
  send({ action: "save", form: {
    channel: q("channel").checked ? "beta" : "stable",
    gpu: q("gpu").checked,
    nothrottle: q("nothrottle").checked,
    unlockfps: q("unlockfps").checked,
    showfps: q("showfps").checked,
    devtools: q("devtools").checked
  }});
}
q("save").addEventListener("click", save);
q("cancel").addEventListener("click", () => send({ action: "close" }));
q("folder").addEventListener("click", () => send({ action: "folder" }));
q("notes").addEventListener("click", () => send({ action: "notes" }));
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") send({ action: "close" });
  if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) save();
});
"#,
    );
    page.push_str("</script></body></html>");
    page
}

pub fn update_page(info: &UpdateInfo) -> String {
    let mut page = head("Update available — Habbo.si Client Beta");
    page.push_str("<div class=\"wrap\">");
    page.push_str(&brand("UPDATE"));
    page.push_str("<h1>A newer build is available</h1>");
    page.push_str(&format!(
        "<p class=\"sub\">This beta port follows the GitHub pre-release feed.</p>\
         <div class=\"card\"><div class=\"big\">{}</div>\
         <div class=\"meta\">Installed: v{} · Latest: {}</div>\
         <p class=\"sub\" style=\"margin:0\">The client never downloads or installs anything. \
         Open the release page to read the notes and download it yourself.</p></div>",
        esc(info.display()),
        VERSION,
        esc(&info.latest)
    ));
    page.push_str(
        "<div class=\"buttons\"><button id=\"later\" class=\"ghost\">Later</button>\
         <span class=\"spacer\"></span><button id=\"open\" class=\"primary\">Open release</button></div>",
    );
    page.push_str(&foot());
    page.push_str("</div><script>");
    page.push_str("const url = ");
    page.push_str(&serde_json::to_string(&info.page).unwrap_or_else(|_| "\"\"".into()));
    page.push_str(
        r#";
function send(payload) { try { window.ipc.postMessage(JSON.stringify(payload)); } catch (_) {} }
document.getElementById("later").onclick = () => send({ action: "close" });
document.getElementById("open").onclick = () => send({ action: "open", url });
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") send({ action: "close" });
  if (e.key === "Enter") send({ action: "open", url });
});
"#,
    );
    page.push_str("</script></body></html>");
    page
}

pub fn notes_page(notes: &ReleaseNotes) -> String {
    let mut page = head("What's new — Habbo.si Client Beta");
    page.push_str("<div class=\"wrap\">");
    page.push_str(&brand("WHAT'S NEW"));
    page.push_str(&format!(
        "<h1>{}</h1><p class=\"sub\">Release notes for the build you are running · {}</p>",
        esc(&notes.name),
        esc(&notes.tag)
    ));
    page.push_str(&format!(
        "<pre class=\"notes\">{}</pre>",
        esc(&plain_text(&notes.body))
    ));
    page.push_str(
        "<div class=\"buttons\" style=\"margin-top:14px\">\
         <button id=\"close\" class=\"ghost\">Close</button>\
         <span class=\"spacer\"></span>\
         <button id=\"open\" class=\"primary\">Open release</button></div>",
    );
    page.push_str(&foot());
    page.push_str("</div><script>");
    page.push_str("const url = ");
    page.push_str(&serde_json::to_string(&notes.page).unwrap_or_else(|_| "\"\"".into()));
    page.push_str(
        r#";
function send(payload) { try { window.ipc.postMessage(JSON.stringify(payload)); } catch (_) {} }
document.getElementById("close").onclick = () => send({ action: "close" });
document.getElementById("open").onclick = () => send({ action: "open", url });
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") send({ action: "close" });
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

    #[test]
    fn escapes_release_notes() {
        let page = notes_page(&ReleaseNotes {
            tag: "1.2.0-beta.1".into(),
            name: "Test <script>".into(),
            body: "- **Hello** <b>x</b>".into(),
            page: "https://example.com".into(),
        });
        assert!(page.contains("&lt;script&gt;"));
        assert!(!page.contains("<script>Test"));
        assert!(page.contains("• Hello &lt;b&gt;x&lt;/b&gt;"));
    }

    #[test]
    fn formats_links_and_headings() {
        assert_eq!(plain_text("## Hi\n\n- [site](https://x.test)"), "Hi\n\n• site (https://x.test)");
    }
}
