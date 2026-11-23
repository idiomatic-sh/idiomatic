//! Intern the query's own constants in the fixed prefix of the file dictionary.
//! Predicate membership then uses a numeric code. Unknown payloads render as
//! the empty string, just as in the engine's dictionary contract.
use super::pair_summary::{Id, Program};
use ryo::languages::{self, CellWord, FileWords, NameText, Walk, facts};

pub(super) fn vocabulary<W: Walk>(_: &W, p: &Program) -> ([FileWords; 2], Vec<Option<Id>>) {
    register_symbols(p, &W::vocabulary().fixed.0)
}

pub(super) fn register_symbols<const N: usize>(p: &Program, names: &[NameText<'_>]) -> ([FileWords; N], Vec<Option<Id>>) {
    let mut text: Vec<_> = p.symbols.iter().map(|(k, &id)| (id, NameText(k))).collect();
    text.sort_unstable_by_key(|x| x.0);
    let selected: Vec<_> = text.iter().map(|x| x.1).collect();
    let fixed = languages::fixed_names::checked(names).expect("declared fixed words");
    let base = facts::base_words(fixed, &selected);
    let mut words = facts::file_words(&base);
    let mut codes = vec![None; facts::base_count(&words).0 as usize];
    for (id, s) in text {
        let n = facts::intern(&mut words, s);
        codes[n.0 as usize] = Some(id);
    }
    (std::array::from_fn(|_| facts::file_words(&base)), codes)
}

#[inline]
pub(super) fn symbol(
    schedule: &super::summary_schedule::Schedule,
    w: &FileWords,
    c: CellWord,
) -> Option<Id> {
    let codes = &schedule.symbols;
    if let Some(id) = usize::try_from(c.0).ok().and_then(|i| codes.get(i)) {
        *id
    } else if c.0 >= facts::words_end(w).0 {
        codes[0]
    } else {
        None
    }
}

#[derive(Default)]
pub(super) struct Matches {
    codes: Vec<Vec<u8>>,
}
impl Matches {
    pub(super) fn reset(&mut self, count: usize, stable: usize) {
        self.codes.resize_with(count, Vec::new);
        for row in &mut self.codes {
            row.truncate(stable);
        }
    }
    pub(super) fn test(&mut self, p: &Program, id: usize, c: CellWord, w: &FileWords) -> bool {
        let at = if c.0 < facts::words_end(w).0 {
            c.0 as usize
        } else {
            0
        };
        let row = &mut self.codes[id];
        if row.len() <= at {
            row.resize(at + 1, 0);
        }
        if row[at] == 0 {
            row[at] = 1 + u8::from(p.regexes[id].is_match(facts::text(w, c).0.as_bytes()));
        }
        row[at] == 2
    }
}
