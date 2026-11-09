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
            bg: "#0b1420",
            bar: "#0e1928",
            footer: "#0b1420",
            card: "#111d2e",
            card2: "#0e1827",
            line: "#203047",
            text: "#e6effc",
            muted: "#8fa4c0",
            dim: "#617691",
            gold: "#f6c452",
            green: "#56d68b",
            toast: "#1b2c40",
            toast_text: "#e6effc",
            rim: "#2a3d56",
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
}
