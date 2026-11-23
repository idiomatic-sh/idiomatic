//! Compile a base-view field test once for the run's provider vocabulary.
use super::{Exec, File, Native, Run, ViewSpec};
use rustc_hash::FxHashMap;
use std::cell::Cell;
use std::rc::Rc;

pub(super) enum Kernel {
    One(u16),
    Set { allowed: Box<[bool]>, unknown: bool },
}

impl Kernel {
    fn fields(run: &Run, members: &[u32]) -> Self {
        let unknown = members.binary_search(&run.empty).is_ok();
        let allowed: Box<[bool]> = run
            .field_sym
            .iter()
            .map(|word| members.binary_search(word).is_ok())
            .collect();
        let mut yes = allowed.iter().enumerate().filter(|(_, accept)| **accept);
        if !unknown
            && let Some((code, _)) = yes.next()
            && yes.next().is_none()
            && let Ok(code) = u16::try_from(code)
        {
            return Self::One(code);
        }
        Self::Set { allowed, unknown }
    }

    fn retain(&self, f: &File<'_>, rows: &mut Vec<u32>) {
        let field = &f.field;
        match self {
            Self::One(code) => rows.retain(|r| field[*r as usize] == *code),
            Self::Set { allowed, unknown } => rows.retain(|r| {
                allowed
                    .get(usize::from(field[*r as usize]))
                    .copied()
                    .unwrap_or(*unknown)
            }),
        }
    }
}

// Positive literals in the same column of a conjunction intersect. Provider codes
// with the same symbol (and out-of-vocabulary fields) keep their original meaning.
fn field_members(v: &ViewSpec) -> Option<Vec<u32>> {
    if v.base != Native::Child {
        return None;
    }
    let mut exprs = v.filter.iter();
    let mut allowed = super::cold_list::members(exprs.next()?, 3)?;
    for e in exprs {
        let other = super::cold_list::members(e, 3)?;
        allowed.retain(|x| other.contains(x));
    }
    allowed.sort_unstable();
    allowed.dedup();
    Some(allowed)
}

impl Exec<'_> {
    pub(crate) fn cold_test_load(&mut self, run: &mut Run) {
        if sw!("VEC_NO_COLD_TYPED") {
            return;
        }
        // Only unary projections descending from the child base can carry a field filter.
        let mut child = vec![false; self.plan.nodes.len()];
        let mut kernels = FxHashMap::<Vec<u32>, Rc<Kernel>>::default();
        let mut slots = 0;
        for (node, pn) in self.plan.nodes.iter().enumerate() {
            if pn.op == "base" {
                child[node] =
                    pn.j["rel"].as_str().and_then(super::native_of) == Some(Native::Child);
                continue;
            }
            let Some(src) = pn.j.get("src").and_then(|s| s.as_array()) else {
                continue;
            };
            if pn.op != "select"
                || src.len() != 1
                || !src[0][0]
                    .as_u64()
                    .and_then(|n| child.get(n as usize))
                    .copied()
                    .unwrap_or(false)
            {
                continue;
            }
            let Ok(Some(v)) = self.view(run, node) else {
                continue;
            };
            child[node] = true;
            let Some(members) = field_members(&v) else {
                continue;
            };
            let kernel = kernels
                .entry(members)
                .or_insert_with_key(|words| Rc::new(Kernel::fields(run, words)));
            v.cold_test
                .set(kernel.clone())
                .unwrap_or_else(|_| panic!("cold test set twice"));
            slots += 1;
        }
        if sw!("VEC_COLD_COUNT") {
            println!("COLD_COMPILED\tchild-field\t{slots}\t{}", kernels.len());
        }
    }
}

thread_local! { static COUNTS: Cell<(u64,u64,u64)> = const { Cell::new((0,0,0)) }; }

/// Gate runs apply the original filter to a copy and compare all row IDs, in order.
pub(super) fn retain(
    f: &File<'_>,
    run: &mut Run,
    v: &ViewSpec,
    node: usize,
    leaf: bool,
    rows: &mut Vec<u32>,
) -> u64 {
    let counting = sw!("VEC_COLD_COUNT") || sw!("VEC_LEAF_TESTS");
    match v.cold_test.get() {
        Some(kernel) if counting => {
            let mut original = rows.clone();
            let tests = super::leaf_scan::count_tests(f, run, v, node, leaf, &mut original);
            let entered = rows.len() as u64;
            kernel.retain(f, rows);
            assert_eq!(
                *rows, original,
                "typed field rows at file {}, node {node}",
                f.file
            );
            COUNTS.with(|s| {
                let (calls, ins, outs) = s.get();
                s.set((calls + 1, ins + entered, outs + rows.len() as u64));
            });
            tests
        }
        Some(kernel) => {
            kernel.retain(f, rows);
            0
        }
        None if counting => super::leaf_scan::count_tests(f, run, v, node, leaf, rows),
        None => {
            if !v.filter.is_empty() {
                super::view_cols::retain(f, run, v, rows);
            }
            0
        }
    }
}

pub(super) fn report() {
    if sw!("VEC_COLD_COUNT") {
        let (builds, rows, kept) = COUNTS.with(|s| s.replace((0, 0, 0)));
        println!("COLD_TYPED\tchild-field\t{builds}\t{rows}\t{kept}");
    }
}
