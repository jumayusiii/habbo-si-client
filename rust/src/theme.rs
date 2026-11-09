#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: &'static str,
    pub bar: &'static str,
    pub footer: &'static str,
    pub card: &'static str,
    pub card2: &'static str,
    pub line: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub dim: &'static str,
    pub gold: &'static str,
    pub green: &'static str,
    pub toast: &'static str,
    pub toast_text: &'static str,
    pub rim: &'static str,
}

/// Which native control scheme a palette needs (`color-scheme`).
pub fn scheme(name: &str) -> &'static str {
    if name == "light" { "light" } else { "dark" }
}

/// The window backdrop for a palette, so a slow paint never flashes white in dark mode.
pub fn rgb(name: &str) -> (u8, u8, u8, u8) {
    let hex = palette(name).bg.trim_start_matches('#');
    let channel = |start: usize| {
        hex.get(start..start + 2)
            .and_then(|part| u8::from_str_radix(part, 16).ok())
            .unwrap_or(0)
    };
    if hex.len() == 6 {
        (channel(0), channel(2), channel(4), 255)
    } else {
        (30, 30, 30, 255)
    }
}

pub fn palette(name: &str) -> Palette {
    match name {
        "light" => Palette {
            bg: "#f2f5f9",
            bar: "#ffffff",
            footer: "#eef2f7",
            card: "#ffffff",
            card2: "#f8fafc",
            line: "#d5dee9",
            text: "#142033",
            muted: "#5b6b80",
            dim: "#8494a8",
            gold: "#b47a00",
            green: "#18834d",
            toast: "#26374d",
            toast_text: "#ffffff",
            rim: "#c5d0dd",
        },
        "navy" => Palette {
            bg: "#081426",
            bar: "#0d1d33",
            footer: "#081426",
            card: "#10233c",
            card2: "#0c1b30",
            line: "#1d3552",
            text: "#e6effc",
            muted: "#91a9c7",
            dim: "#66819f",
            gold: "#f6c452",
            green: "#56d68b",
            toast: "#1b304b",
            toast_text: "#e6effc",
            rim: "#2a455f",
        },
        _ => Palette {
            bg: "#1e1e1e",
            bar: "#252526",
            footer: "#1e1e1e",
            card: "#252526",
            card2: "#202020",
            line: "#3a3a3a",
            text: "#e8e8e8",
            muted: "#a6a6a6",
            dim: "#7d7d7d",
            gold: "#f6c452",
            green: "#56d68b",
            toast: "#2d2d2d",
            toast_text: "#e8e8e8",
            rim: "#4a4a4a",
        },
    }
}

impl Palette {
    pub fn css(&self) -> String {
        format!(
            "--bg:{};--bar:{};--footer:{};--card:{};--card2:{};--line:{};--text:{};\
             --muted:{};--dim:{};--gold:{};--green:{};--toast:{};--toast-text:{};--rim:{};",
            self.bg,
            self.bar,
            self.footer,
            self.card,
            self.card2,
            self.line,
            self.text,
            self.muted,
            self.dim,
            self.gold,
            self.green,
            self.toast,
            self.toast_text,
            self.rim
        )
    }

    /// The palette as a JS object literal for the settings theme preview.
    pub fn js(&self) -> String {
        format!(
            "{{bg:'{}',bar:'{}',footer:'{}',card:'{}',card2:'{}',line:'{}',text:'{}',\
             muted:'{}',dim:'{}',gold:'{}',green:'{}',toast:'{}',toastText:'{}',rim:'{}'}}",
            self.bg,
            self.bar,
            self.footer,
            self.card,
            self.card2,
            self.line,
            self.text,
            self.muted,
            self.dim,
            self.gold,
            self.green,
            self.toast,
            self.toast_text,
            self.rim
        )
    }
}
