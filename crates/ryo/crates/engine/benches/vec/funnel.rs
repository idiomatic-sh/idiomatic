//! Intern the typed word prerequisites and look them up in the provider's token table.
use ryo::languages::Position;
use std::collections::HashMap;

pub(super) struct FRule {
    pub(super) name: String,
    pub(super) cnfs: Vec<Vec<Vec<u16>>>,
}

#[derive(Default)]
pub(super) struct Words {
    pub(super) ids: HashMap<Vec<u8>, u16>,
    pub(super) names: Vec<String>,
}

impl Words {
    fn id(&mut self, w: &str) -> u16 {
        if let Some(i) = self.ids.get(w.as_bytes()) {
            return *i;
        }
        let i = self.names.len() as u16;
        self.names.push(w.to_owned());
        self.ids.insert(w.as_bytes().to_vec(), i);
        i
    }
    fn cnf(&mut self, v: &[Vec<String>]) -> Vec<Vec<u16>> {
        v.iter()
            .map(|c| c.iter().map(|w| self.id(w)).collect())
            .collect()
    }
}

pub(super) fn build(specs: &[super::spec::SpecRule]) -> (Vec<FRule>, Words) {
    let mut words = Words::default();
    let rules = specs
        .iter()
        .map(|rule| FRule {
            name: rule.name.clone(),
            cnfs: rule
                .word_cnfs
                .iter()
                .map(|clauses| words.cnf(clauses))
                .collect(),
        })
        .collect();
    (rules, words)
}

const UNSEEN: u32 = u32::MAX;
/// A memo entry whose text is no required word.
const NO_WORD: u32 = u32::MAX - 1;

/// The high bit of a name code: a fixed text of the language, the same in every file of the run.
pub(super) const FIXED_TEXT: u32 = 1 << 31;

/// The file's required words from its name tokens, in id order: the present bits and the (word, token id) pairs. A token the
/// walk gave a code (`codes`: the file's own tree, in id order) is looked up once a code through `memo` (a fixed text's,
/// once a run through `fixed`); every other (an embedded segment's, or all of a file whose bytes were no text) by its bytes.
#[expect(clippy::too_many_arguments, reason = "throwaway prototype")]
pub(super) fn file_words<'a>(
    positions: impl Iterator<Item = Position>,
    text: impl Fn(usize) -> &'a [u8],
    codes: &[u32],
    words: &Words,
    memo: &mut Vec<u32>,
    fixed: &mut Vec<u32>,
    present: &mut [u64],
    pairs: &mut Vec<(u16, u32)>,
) {
    present.iter_mut().for_each(|w| *w = 0);
    pairs.clear();
    memo.clear();
    for (j, p) in positions.enumerate() {
        let i = p.0 as usize;
        let w = match codes.get(j) {
            Some(&c) => {
                let m = if c & FIXED_TEXT == 0 {
                    &mut *memo
                } else {
                    &mut *fixed
                };
                let c = (c & !FIXED_TEXT) as usize;
                if c >= m.len() {
                    m.resize(c + 1, UNSEEN);
                }
                if m[c] == UNSEEN {
                    m[c] = words.ids.get(text(i)).map_or(NO_WORD, |w| u32::from(*w));
                }
                m[c]
            }
            None => words.ids.get(text(i)).map_or(NO_WORD, |w| u32::from(*w)),
        };
        if w != NO_WORD {
            let w = w as u16;
            present[usize::from(w) / 64] |= 1 << (w % 64);
            pairs.push((w, p.0));
        }
    }
}
