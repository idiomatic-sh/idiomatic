//! Compile a base-view field test once for the run's provider vocabulary.
use super::{Exec, File, Native, Run, ViewSpec};
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

pub(super) enum Kernel {
    Columns(&'static str, super::cold_pred::Program),
    One(u16),
    FieldWords(Box<[u32]>),
    Set { allowed: Box<[bool]>, unknown: bool },
}

impl Kernel {
    fn fields(run: &Run, members: &[u32]) -> Self {
        if run.file_field_codes {
            return Self::FieldWords(members.into());
        }
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

    fn retain(&self, f: &File<'_>, run: &mut Run, rows: &mut Vec<u32>) {
        let field = &f.field;
        match self {
            Self::Columns(_, program) => program.retain(f, run, rows),
            Self::FieldWords(words) => rows.retain(|r| {
                let word = run
                    .field_sym
                    .get(usize::from(field[*r as usize]))
                    .copied()
                    .unwrap_or(run.empty);
                words.binary_search(&word).is_ok()
            }),
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

/// Filters shared by views first reached during this run. SDK field codes are resolved per file.
#[derive(Default)]
pub(super) struct Compiler {
    fields: FxHashMap<Vec<u32>, Rc<Kernel>>,
    programs: FxHashMap<(Native, Vec<super::cold_list::FilterKey>), Option<Rc<Kernel>>>,
    inventory: BTreeMap<(&'static str, bool), (usize, usize)>,
}

impl Compiler {
    pub(super) fn bind(&mut self, run: &mut Run, node: usize, v: &ViewSpec) {
        if v.filter.is_empty() {
            return;
        }
        if let Some(entry) = super::cold_list::Handoff::compile(run, v) {
            assert!(v.cold_bucket.set(entry).is_ok());
        }
        let form = super::cold_list::form(v);
        let mut first = false;
        let kernel = if let Some(words) = field_members(v) {
            Some(
                self.fields
                    .entry(words)
                    .or_insert_with_key(|words| {
                        first = true;
                        Rc::new(Kernel::fields(run, words))
                    })
                    .clone(),
            )
        } else {
            let key = (v.base, super::cold_list::conjuncts(&v.filter));
            self.programs
                .entry(key)
                .or_insert_with(|| {
                    first = true;
                    super::cold_pred::Program::compile(run, v)
                        .map(|p| Rc::new(Kernel::Columns(form, p)))
                })
                .clone()
        };
        if sw!("VEC_COLD_COUNT") {
            let n = self.inventory.entry((form, kernel.is_some())).or_default();
            n.0 += 1;
            n.1 += usize::from(first);
        }
        if kernel.is_none() && sw!("VEC_COLD_KEYS") {
            println!("COLD_UNCOMPILED\t{node}\t{form}\t{:?}", v.filter);
        }
        if let Some(k) = kernel {
            v.cold_test
                .set(k)
                .unwrap_or_else(|_| panic!("cold test set twice"));
        }
    }
}
impl Exec<'_> {
    // The optional audit reports inventory for the entire plan, including unreachable views.
    pub(crate) fn cold_test_load(&mut self, run: &mut Run) {
        if !(sw!("VEC_COLD_COUNT") || sw!("VEC_COLD_KEYS")) {
            return;
        }
        for (node, pn) in self.plan.nodes.iter().enumerate() {
            if pn.op == "select" && pn.inputs.len() == 1 {
                let _ = self.view(run, node);
            }
        }
        for ((form, compiled), (views, distinct)) in &self.cold_compiler.inventory {
            println!("COLD_COMPILED\t{form}\t{views}\t{distinct}\t{compiled}");
        }
    }
}

thread_local! { static COUNTS: RefCell<BTreeMap<(bool,&'static str),[u64;3]>> = const { RefCell::new(BTreeMap::new()) }; }
fn count(kernel: &Kernel, entry: bool, entered: usize, kept: usize) {
    let form = match kernel {
        Kernel::Columns(form, _) => *form,
        _ => "child-field",
    };
    COUNTS.with_borrow_mut(|m| {
        let n = m.entry((entry, form)).or_default();
        n[0] += 1;
        n[1] += entered as u64;
        n[2] += kept as u64;
    });
}

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
            let entered = rows.len();
            kernel.retain(f, run, rows);
            assert_eq!(
                *rows, original,
                "typed rows at file {}, node {node}",
                f.file
            );
            count(kernel, false, entered, rows.len());
            tests
        }
        Some(kernel) => {
            kernel.retain(f, run, rows);
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

/// Read a typed base column or its already indexed literal rows directly.
pub(super) fn column_rows(
    f: &File<'_>,
    run: &mut Run,
    v: &ViewSpec,
    node: usize,
) -> Option<(Rc<Vec<u32>>, usize, u64)> {
    let Kernel::Columns(_, program) = &**v.cold_test.get()? else {
        return None;
    };
    let (rows, entered, tests) = program.rows(f, run)?;
    if sw!("VEC_COLD_COUNT") {
        let mut original = match super::kind_set(run, &v.filter, 0, v.base, &[]) {
            Some(ks) => super::bucket(f, run, &ks),
            None => super::lit_rows(f, &v.filter, 0, v.base, &[])
                .unwrap_or_else(|| f.native_rows(run, v.base)),
        };
        super::view_cols::retain_original(f, run, v, &mut original);
        assert_eq!(*rows, original, "column entry file {}, node {node}", f.file);
    }
    Some((rows, entered, tests))
}

/// The select front can enter a narrower base bucket and apply the very same view filter.
pub(super) fn entry(f: &File<'_>, run: &mut Run, v: &ViewSpec, rows: &mut Vec<u32>) -> bool {
    let Some(kernel) = v.cold_test.get() else {
        return false;
    };
    if sw!("VEC_COLD_COUNT") {
        let entered = rows.len();
        let mut original = rows.clone();
        super::view_cols::retain_original(f, run, v, &mut original);
        kernel.retain(f, run, rows);
        assert_eq!(*rows, original, "typed entry rows at file {}", f.file);
        count(kernel, true, entered, rows.len());
    } else {
        kernel.retain(f, run, rows);
    }
    true
}

pub(super) fn report() {
    if sw!("VEC_COLD_COUNT") {
        COUNTS.with_borrow(|all| {
            for ((entry, form), [calls, rows, kept]) in all {
                let label = if *entry {
                    "COLD_TYPED_ENTRY"
                } else {
                    "COLD_TYPED"
                };
                println!("{label}\t{form}\t{calls}\t{rows}\t{kept}");
            }
        });
    }
}
