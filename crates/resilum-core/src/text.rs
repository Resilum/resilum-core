use std::fmt::Display;

pub trait Text {
    fn line(&mut self, text: impl Display);
    fn block(&mut self, text: impl Display);
}

impl Text for String {
    fn line(&mut self, text: impl Display) {
        self.block(text);
        self.push('\n');
    }

    fn block(&mut self, text: impl Display) {
        self.push_str(&text.to_string());
    }
}

#[cfg(test)]
mod tests;
