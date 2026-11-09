#[derive(Clone, Copy, Debug)]
pub struct Lang {
    pub de: bool,
}

impl Lang {
    pub fn new(de: bool) -> Self {
        Self { de }
    }

    pub fn t<'a>(&self, en: &'a str, de: &'a str) -> &'a str {
        if self.de { de } else { en }
    }
}
