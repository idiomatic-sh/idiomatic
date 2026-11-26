//! Attribute view materializations to their source plan nodes.
use super::{Exec, File, Native, Run, ViewSpec, kinds_required, step_clock};
use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Default)]
struct Driver {
    calls: u64,
    rows: u64,
    seconds: f64,
}

thread_local! {
    static TESTS: RefCell<BTreeMap<(usize, bool), u64>> = const { RefCell::new(BTreeMap::new()) };
    static LISTS: RefCell<BTreeMap<(usize, bool), (Driver, u64)>> = const { RefCell::new(BTreeMap::new()) };
}

/// A leaf-only CPU clock for the share of an otherwise unclocked second half.
#[inline]
pub(super) fn table_clock() -> f64 {
    if sw!("VEC_LEAF_CLOCK") {
        super::super::thread_cpu()
    } else {
        step_clock()
    }
}

/// Diagnostic only. Keep the existing evaluator and short-circuit order, recording
/// the number of candidate rows each top-level compiled conjunct actually sees.
pub(super) fn count_tests(
    f: &File<'_>,
    run: &mut Run,
    v: &ViewSpec,
    node: usize,
    materialize: bool,
    rows: &mut Vec<u32>,
) -> u64 {
    let mut n = 0;
    for test in &v.filter {
        if rows.is_empty() {
            break;
        }
        n += rows.len() as u64;
        let one = ViewSpec::new(run, v.base, Vec::new(), vec![test.clone()]);
        super::view_cols::retain(f, run, &one, rows);
    }
    if sw!("VEC_LEAF_TESTS") {
        TESTS.with_borrow_mut(|all| *all.entry((node, materialize)).or_default() += n);
    }
    n
}

/// A cold row-list read outside materialization is timed in its consumer's step.
pub(super) fn row_list(node: usize, materialize: bool, entered: usize, kept: usize, start: f64) {
    let seconds = step_clock() - start;
    LISTS.with_borrow_mut(|all| {
        let (d, scanned) = all.entry((node, materialize)).or_default();
        d.calls += 1;
        d.rows += kept as u64;
        d.seconds += seconds;
        *scanned += entered as u64;
    });
}

impl Exec<'_> {
    pub(crate) fn leaf_report(&mut self, run: &mut Run) {
        if sw!("VEC_LEAF_CLOCK") {
            let mut brackets = [0.0; 1001];
            for cost in &mut brackets {
                let begin = table_clock();
                *cost = table_clock() - begin;
            }
            brackets.sort_unstable_by(f64::total_cmp);
            println!("LEAF_CLOCK_NS\t{:.2}", brackets[brackets.len() / 2] * 1e9);
        }
        if !sw!("VEC_LEAF_PROFILE") {
            return;
        }
        for node in 0..self.plan.nodes.len() {
            if let Ok(Some(v)) = self.view(run, node) {
                println!(
                    "LEAF_VIEW\t{node}\t{}\t{}",
                    super::json_output::buckets(&buckets(run, &v)),
                    super::json_output::buckets(&current_entries(run, &v))
                );
            }
        }
        TESTS.with_borrow_mut(|all| {
            for ((node, materialize), tests) in std::mem::take(all) {
                println!("LEAF_TESTS\t{node}\t{materialize}\t{tests}");
            }
        });
        LISTS.with_borrow_mut(|all| {
            for ((node, materialize), (d, scanned)) in std::mem::take(all) {
                println!(
                    "LEAF_LIST\t{node}\t{materialize}\t{}\t{scanned}\t{}\t{:.9}",
                    d.calls, d.rows, d.seconds
                );
            }
        });
    }
}

/// Physical buckets for a shared scan. A multi-value filter can subscribe to
/// several buckets. Fact entries group by family so other filters share its scan.
fn buckets(run: &Run, v: &ViewSpec) -> Vec<Vec<String>> {
    let (base, column) = match v.base {
        Native::Tree(_) => ("tree", Some(1)),
        Native::Child(_) => ("child", Some(3)),
        Native::Location => ("location", None),
        Native::Token => ("token", None),
        Native::Fact(i) => {
            return vec![vec!["fact".into(), run.fact_rels[usize::from(i)].0.clone()]];
        }
    };
    if let Some(column) = column {
        for test in &v.filter {
            if let Some(values) = kinds_required(test, 0, column) {
                let mut names: Vec<_> = values
                    .iter()
                    .map(|v| String::from_utf8_lossy(run.text(*v)))
                    .collect();
                names.sort_unstable();
                names.dedup();
                return names
                    .into_iter()
                    .map(|n| vec![base.into(), n.into_owned()])
                    .collect();
            }
        }
    }
    vec![vec![base.into()]]
}

/// Current view_rows entries. Fact column/value lookup follows the conjunct and
/// column order of lit_pick. The child relation presently reads every child.
pub(super) fn fact_entry(
    run: &mut Run,
    i: u16,
    filter: &[super::E],
) -> Option<(usize, String, Vec<u32>)> {
    use super::{E, Ty};
    fn columns(e: &E, out: &mut Vec<usize>) {
        match e {
            E::Col(0, c, _) => out.push(*c),
            E::Cmp(_, a, b) | E::And(a, b) | E::Or(a, b) | E::In(a, b, _) => {
                columns(a, out);
                columns(b, out);
            }
            _ => {}
        }
    }
    let rel = run.fact_rels[usize::from(i)].0.clone();
    let (_, names, types) = run.catalog(&rel).expect("loaded fact");
    let mut cols = Vec::new();
    for e in filter {
        cols.clear();
        columns(e, &mut cols);
        for &c in &cols {
            if types[c] == Ty::Sym
                && let Some(xs) = kinds_required(e, 0, c)
            {
                return Some((c, names[c].clone(), xs));
            }
        }
    }
    None
}

fn current_entries(run: &mut Run, v: &ViewSpec) -> Vec<Vec<String>> {
    let (prefix, values) = match v.base {
        Native::Tree(_) => (
            vec!["tree".to_owned()],
            super::kind_set(run, &v.filter, 0, v.base, &[]),
        ),
        Native::Child(_) => return vec![vec!["child".into()]],
        Native::Fact(i) => {
            let mut prefix = vec!["fact".to_owned(), run.fact_rels[usize::from(i)].0.clone()];
            let values = fact_entry(run, i, &v.filter).map(|(_, name, xs)| {
                prefix.push(name);
                xs
            });
            (prefix, values)
        }
        _ => return buckets(run, v),
    };
    let Some(mut values) = values else {
        return vec![prefix];
    };
    values.sort_unstable();
    values.dedup();
    values
        .into_iter()
        .map(|sym| {
            let mut key = prefix.clone();
            key.push(String::from_utf8_lossy(run.text(sym)).into_owned());
            key
        })
        .collect()
}
