//! Borrowed words contracts; no engine authority.
/// Engine-owned symbol collection for words; no row delivery or acquisition is exposed.
pub trait WordOutput {
    fn word(&mut self, text: &str);
}

/// The engine-owned scratch one word is built in; it keeps its capacity between calls.
pub(crate) struct WordScratch<'a>(pub &'a mut String);
