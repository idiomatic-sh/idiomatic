//! Borrowed source matches contracts; no engine authority.
/// The engine lends exactly one checked source interval and its acquired matcher.
pub struct SourceMatchesInput<'a> {
    pub row: &'a [ryo_relations::domain::relations::types::Value],
    pub source: &'a [u8],
    pub base: usize,
    pub text: &'a str,
    pub line: Option<i64>,
    pub regex: &'a regex_automata::meta::Regex,
    pub capture: Option<&'a str>,
    pub capture_text: bool,
    pub capture_line: bool,
}

/// Engine-owned symbol storage and row delivery. No acquisition or publication API is exposed.
pub trait TextRowDriver {
    fn intern(&mut self, text: &str) -> ryo_relations::domain::relations::types::Value;
    fn emit(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<(), SemanticInterrupted>;
}

use crate::domain::contract::types::SemanticInterrupted;
