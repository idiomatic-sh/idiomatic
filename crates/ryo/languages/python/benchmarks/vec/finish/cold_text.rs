//! Bind requested literals once. The file dictionaries use the same decoded text, source
//! spelling and absent text as File::elem_text. A symbol maps directly to its plan-bound slot.
use super::{E, File, Run, cold_word::Test};
use rustc_hash::FxHashMap;
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

#[derive(Default)]
pub(super) struct Plan {
    by_symbol: Vec<u32>,
    slots: usize,
    tests: FxHashMap<Vec<String>, Rc<Predicate>>,
}
pub(super) fn slots(run: &mut Run, words: &[u32]) -> Vec<usize> {
    words
        .iter()
        .map(|word| {
            let sym = *word as usize;
            let plan = &mut run.cold_text;
            if plan.by_symbol.len() <= sym {
                plan.by_symbol.resize(sym + 1, u32::MAX);
            }
            if plan.by_symbol[sym] == u32::MAX {
                plan.by_symbol[sym] = plan.slots as u32;
                plan.slots += 1;
            }
            plan.by_symbol[sym] as usize
        })
        .collect()
}

#[derive(Default)]
pub(super) struct Dictionary {
    postings: OnceCell<Vec<Vec<u32>>>,
    answers: RefCell<FxHashMap<(usize, u32), bool>>,
}

pub(super) struct Predicate {
    id: usize,
    test: Test,
}
impl Predicate {
    pub(super) fn compile(run: &mut Run, expr: &E, test: Test) -> Rc<Self> {
        let key = super::cold_list::conjuncts(std::slice::from_ref(expr));
        let id = run.cold_text.tests.len();
        run.cold_text
            .tests
            .entry(key)
            .or_insert_with(|| Rc::new(Self { id, test }))
            .clone()
    }
    pub(super) fn accepts(&self, f: &File<'_>, run: &mut Run, row: u32) -> bool {
        let symbol = f.elem_text(run, row);
        if sw!("VEC_COLD_COUNT") {
            counts(|n| n[4] += 1);
        }
        *f.cold_text
            .answers
            .borrow_mut()
            .entry((self.id, symbol))
            .or_insert_with(|| {
                if sw!("VEC_COLD_COUNT") {
                    counts(|n| n[3] += 1);
                }
                (self.test)(run, u64::from(symbol))
            })
    }
}

pub(super) fn candidates(
    f: &File<'_>,
    run: &mut Run,
    slots: &[usize],
    out: &mut Vec<u32>,
) -> usize {
    let mut indexed = 0;
    let postings = f.cold_text.postings.get_or_init(|| {
        let start = super::cold_pred::phase_clock();
        let mut lists = vec![Vec::new(); run.cold_text.slots];
        indexed = f.n;
        for (i, &kind) in f.kind.iter().enumerate() {
            let symbol = if File::has_text(run, kind) {
                f.elem_text(run, i as u32)
            } else {
                run.empty
            };
            if let Some(&slot) = run.cold_text.by_symbol.get(symbol as usize)
                && slot != u32::MAX
            {
                lists[slot as usize].push(i as u32);
            }
        }
        if sw!("VEC_COLD_COUNT") {
            counts(|n| {
                n[0] += 1;
                n[1] += f.n as u64;
                n[2] += lists.iter().map(|s| s.len() as u64).sum::<u64>();
            });
        }
        super::cold_pred::phase("text postings", start);
        lists
    });
    let mut streams = 0;
    for &slot in slots {
        let rows = &postings[slot];
        if !rows.is_empty() {
            streams += 1;
            out.extend_from_slice(rows);
        }
    }
    if streams > 1 {
        out.sort_unstable();
    }
    indexed
}

thread_local! {
    static TOTALS: std::cell::Cell<[u64; 5]> = const { std::cell::Cell::new([0; 5]) };
}
fn counts(add: impl FnOnce(&mut [u64; 5])) {
    TOTALS.with(|all| {
        let mut n = all.get();
        add(&mut n);
        all.set(n);
    });
}
pub(super) fn report() {
    if sw!("VEC_COLD_COUNT") {
        let [files, indexed, postings, evaluated, requests] = TOTALS.with(|c| c.get());
        println!("COLD_TEXT\t{files}\t{indexed}\t{postings}\t{evaluated}\t{requests}");
    }
}
