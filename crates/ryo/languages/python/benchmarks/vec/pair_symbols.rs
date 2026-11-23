//! A filled symbol cell already is an integer. Translate each file word into
//! the query's vocabulary once, including the fixed prefix shared by files.
use super::*;
#[derive(Default)]
pub(super) struct Codes {
    values: Vec<usize>,
}
impl Codes {
    pub(super) fn next_file(&mut self, words: &FileWords) {
        let base = languages::facts::base_count(words).0 as usize;
        self.values.truncate(base);
    }
    pub(super) fn extend(&mut self, words: &FileWords, symbols: &Map<String, Id>) {
        let end = languages::facts::words_end(words).0 as usize;
        for i in self.values.len()..end {
            let t = languages::facts::text(words, languages::CellWord(i as u64));
            self.values.push(symbols.get(t.0).map_or(0, |i| i + 1));
        }
    }
    pub(super) fn symbol(&self, cell: languages::CellWord) -> Option<Id> {
        usize::try_from(cell.0)
            .ok()
            .and_then(|i| self.values.get(i))
            .or_else(|| self.values.first())?
            .checked_sub(1)
    }
}
