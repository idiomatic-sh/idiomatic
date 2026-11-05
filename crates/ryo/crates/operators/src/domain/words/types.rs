//! Borrowed words contracts; no engine authority.
/// Engine-owned symbol collection for words; no row delivery or acquisition is exposed.
pub trait WordOutput {
    fn word(&mut self, text: &str);
}
