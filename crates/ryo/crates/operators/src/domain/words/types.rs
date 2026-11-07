//! Borrowed words contracts; no engine authority.
/// Engine-owned symbol collection for words; no row delivery or acquisition is exposed.
pub trait WordOutput {
    fn word(&mut self, text: ryo_values::domain::row_texts::types::CellText<'_>);
}

/// The engine-owned scratch one word is built in; it keeps its capacity between calls.
pub struct WordScratch<'a>(pub &'a mut String);

/// Interns each word as derived text.
pub(crate) struct WordMinting<'a> {
    /// The engine's driver, which interns each word.
    pub driver: &'a mut dyn crate::domain::contract::types::RowDriver,
    /// The interned words, in order.
    pub captured: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
}

impl crate::domain::words::types::WordOutput for WordMinting<'_> {
    fn word(&mut self, text: ryo_values::domain::row_texts::types::CellText<'_>) {
        let made = self.driver.mint(text);
        self.captured.push(made);
    }
}
