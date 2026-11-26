//! Borrowed words contracts; no engine authority.
pub use ryo_values::domain::words::types::{WordOutput, WordScratch};

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
