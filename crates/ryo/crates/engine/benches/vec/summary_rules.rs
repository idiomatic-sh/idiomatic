//! Invert the file-level conjunctions. A failed shared requirement clears
//! every still-live rule that needs it with a few machine-word operations.
use super::pair_summary::{Id, Need, Program};
use rustc_hash::FxHashMap;

struct Test {
    need: Id,
    uses: std::ops::Range<usize>,
}
pub(super) struct Rules {
    tests: Vec<Test>,
    uses: Vec<(usize, u64)>,
}
impl Rules {
    pub(super) fn new(p: &Program, needs: &[Vec<Id>], price: &[u8]) -> Self {
        let mut dependents = FxHashMap::<Id, Vec<(usize, u64)>>::default();
        for (root, (_, need)) in p.roots.iter().enumerate() {
            let checks = if matches!(p.needs[*need], Need::Both(..)) {
                needs[*need].as_slice()
            } else {
                std::slice::from_ref(need)
            };
            for &id in checks {
                if matches!(p.needs[id], Need::Any) {
                    continue;
                }
                let uses = dependents.entry(id).or_default();
                let word = root / 64;
                if uses.last().is_none_or(|r| r.0 != word) {
                    uses.push((word, 0));
                }
                uses.last_mut().unwrap().1 |= 1 << (root % 64);
            }
        }
        let mut ordered: Vec<_> = dependents.into_iter().collect();
        ordered.sort_unstable_by_key(|(id, blocks)| {
            let fanout: usize = blocks.iter().map(|x| x.1.count_ones() as usize).sum();
            (price[*id], std::cmp::Reverse(fanout), *id)
        });
        let mut tests = Vec::with_capacity(ordered.len());
        let mut uses = Vec::new();
        for (need, mut bits) in ordered {
            let start = uses.len();
            uses.append(&mut bits);
            tests.push(Test {
                need,
                uses: start..uses.len(),
            });
        }
        Self { tests, uses }
    }
    pub(super) fn apply(
        &self,
        alive: &mut [bool],
        bits: &mut Vec<u64>,
        mut test: impl FnMut(Id) -> bool,
    ) {
        bits.clear();
        bits.resize(alive.len().div_ceil(64), 0);
        for (chunk, word) in alive.chunks(64).zip(bits.iter_mut()) {
            for (i, &live) in chunk.iter().enumerate() {
                *word |= u64::from(live) << i;
            }
        }
        for t in &self.tests {
            let rules = &self.uses[t.uses.clone()];
            if rules.iter().any(|&(i, m)| bits[i] & m != 0) && !test(t.need) {
                for &(i, m) in rules {
                    bits[i] &= !m;
                }
            }
        }
        for (i, value) in alive.iter_mut().enumerate() {
            *value = bits[i / 64] & (1 << (i % 64)) != 0;
        }
    }
}
