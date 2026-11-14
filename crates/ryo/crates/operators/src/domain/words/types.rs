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

/// What a words lowers to: the text column.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WordsParts {
    /// The text column.
    pub(crate) text: ryo_values::domain::plan_parts::types::ColumnIndex,
}

/// The words of a text, one after the other in `words`, each word's place in `places`: kept to be minted once the split is
/// done.
pub(crate) struct WordPlaces<'a> {
    /// The words, one after the other.
    pub(crate) words: &'a mut crate::domain::contract::types::TextScratch,
    /// The place of each word in `words`.
    pub(crate) places: &'a mut crate::domain::contract::types::RangeScratch,
}

impl WordOutput for WordPlaces<'_> {
    fn word(&mut self, text: ryo_values::domain::row_texts::types::CellText<'_>) {
        let start = self.words.0.len();
        self.words.0.push_str(text.0);
        self.places.0.push(start..self.words.0.len());
    }
}
