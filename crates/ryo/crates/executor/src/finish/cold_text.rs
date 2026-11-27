//! Bind requested literals once. The file dictionaries use the same decoded text, source
//! spelling and absent text as File::elem_text. A symbol maps directly to its plan-bound slot.
use super::{E, File, Run, cold_word::Test};
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
pub struct Plan {
    pub by_symbol: Vec<u32>,
    pub slots: usize,
    pub by_bytes: FxHashMap<Rc<[u8]>, u32>,
    pub one_byte: Vec<u32>,
    pub longest: usize,
    pub tests: FxHashMap<Vec<super::cold_list::FilterKey>, Rc<Predicate>>,
}
pub fn slots(run: &mut Run, words: &[u32]) -> Vec<usize> {
    words
        .iter()
        .map(|word| {
            let sym = *word as usize;
            let plan = &mut run.cold_text;
            if plan.by_symbol.len() <= sym {
                plan.by_symbol.resize(sym + 1, u32::MAX);
            }
            if plan.by_symbol[sym] == u32::MAX {
                let slot = plan.slots as u32;
                let text = &run.texts[sym];
                if let [byte] = &text[..] {
                    if plan.one_byte.is_empty() {
                        plan.one_byte.resize(256, u32::MAX);
                    }
                    plan.one_byte[usize::from(*byte)] = slot;
                } else {
                    plan.longest = plan.longest.max(text.len());
                    plan.by_bytes.insert(text.clone(), slot);
                }
                plan.by_symbol[sym] = slot;
                plan.slots += 1;
            }
            plan.by_symbol[sym] as usize
        })
        .collect()
}

impl Plan {
    pub fn symbol_slot(&self, symbol: u32) -> u32 {
        self.by_symbol
            .get(symbol as usize)
            .copied()
            .unwrap_or(u32::MAX)
    }

    pub fn spelling_slot(&self, text: &[u8]) -> u32 {
        match text {
            [byte] => self
                .one_byte
                .get(usize::from(*byte))
                .copied()
                .unwrap_or(u32::MAX),
            _ if text.len() <= self.longest => self.by_bytes.get(text).copied().unwrap_or(u32::MAX),
            _ => u32::MAX,
        }
    }
}

#[derive(Default)]
pub struct Dictionary {
    pub postings: RefCell<Vec<Vec<u32>>>,
}

pub struct Predicate {
    pub test: super::Bound<Test>,
}
impl Predicate {
    pub fn compile(run: &mut Run, expr: &E, test: Test) -> Rc<Self> {
        let key = super::cold_list::conjuncts(std::slice::from_ref(expr));
        run.cold_text
            .tests
            .entry(key)
            .or_insert_with(|| Rc::new(Self { test: super::Bound::new(test) }))
            .clone()
    }
    pub fn accepts(&self, f: &File<'_>, run: &mut Run, row: u32) -> bool {
        let symbol = f.elem_text(run, row);

        self.test.answer(symbol, || {

            (self.test.pattern)(run, u64::from(symbol))
        })
    }
}

pub fn candidates(
    extent: super::Extent,
    f: &File<'_>,
    run: &mut Run,
    slots: &[usize],
    out: &mut Vec<u32>,
) -> usize {
    let mut indexed = 0;
    let mut postings = f.cold_text.postings.borrow_mut();
    let previous = postings.len();
    // The run can bind another literal after this file first built its dictionary.
    // Extend only missing lists; their ids and all existing node orders remain stable.
    if previous < run.cold_text.slots && slots.iter().any(|&slot| slot >= previous) {
        postings.resize_with(run.cold_text.slots, Vec::new);
        indexed = f.n;
        let empty = run.cold_text.symbol_slot(run.empty);
        let memo = f.text_memo.borrow();
        let symbols = memo.iter().copied().chain(std::iter::repeat(u32::MAX));
        for (i, (&kind, symbol)) in f.kind.iter().zip(symbols).enumerate() {
            let slot = if symbol != u32::MAX {
                run.cold_text.symbol_slot(symbol)
            } else if File::has_text(run, kind) {
                run.cold_text.spelling_slot(f.text_bytes(i as u32))
            } else {
                empty
            };
            if slot != u32::MAX && slot as usize >= previous {
                postings[slot as usize].push(i as u32);
            }
        }

    }
    let mut streams = 0;
    for &slot in slots {
        let rows = &postings[slot];
        if !rows.is_empty() {
            streams += 1;
            // the postings of a file with embedded segments are cut at the extent: one compare to pick the slice
            let rows = if extent == super::Extent::Own && f.own < f.n { &rows[..rows.partition_point(|r| (*r as usize) < f.own)] } else { &rows[..] };
            out.extend_from_slice(rows);
        }
    }
    if streams > 1 {
        out.sort_unstable();
    }
    indexed
}

