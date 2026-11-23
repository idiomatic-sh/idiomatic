//! Attribute view materializations and fused body drivers to their source plan nodes.
use super::{Exec, File, Native, Provider, R, Run, Table, ViewSpec, kinds_required, step_clock};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Default)]
struct Driver {
    calls: u64,
    rows: u64,
    seconds: f64,
}

thread_local! {
    static TESTS: RefCell<BTreeMap<(usize, bool), u64>> = const { RefCell::new(BTreeMap::new()) };
    static LISTS: RefCell<BTreeMap<(usize, bool), (Driver, u64)>> = const { RefCell::new(BTreeMap::new()) };
    static DRIVERS: RefCell<BTreeMap<(usize, usize), Driver>> = const { RefCell::new(BTreeMap::new()) };
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
) {
    let mut n = 0;
    for test in &v.filter {
        if rows.is_empty() {
            break;
        }
        n += rows.len() as u64;
        let one = ViewSpec::new(v.base, Vec::new(), vec![test.clone()]);
        super::view_cols::retain(f, run, &one, rows);
    }
    TESTS.with_borrow_mut(|all| *all.entry((node, materialize)).or_default() += n);
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
    /// A fused driver may run a join or demand an intermediate table. Attribute its
    /// exclusive time separately so the entire enclosing group is never charged to a leaf.
    #[inline]
    pub(super) fn leaf_driver(
        &mut self,
        f: &mut File<'_>,
        run: &mut Run,
        p: &mut dyn Provider,
        root: usize,
        node: usize,
    ) -> R<Rc<Table>> {
        if !sw!("VEC_LEAF_PROFILE") {
            return self.body(f, run, p, node);
        }
        let inner = self.inner;
        let start = step_clock();
        let out = self.body(f, run, p, node)?;
        let seconds = (step_clock() - start) - (self.inner - inner);
        DRIVERS.with_borrow_mut(|all| {
            let d = all.entry((root, node)).or_default();
            d.calls += 1;
            d.rows += out.n as u64;
            d.seconds += seconds;
        });
        Ok(out)
    }

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
                    buckets(run, &v),
                    current_entries(run, &v)
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
        DRIVERS.with_borrow_mut(|all| {
            for ((root, node), d) in std::mem::take(all) {
                println!(
                    "LEAF_DRIVER\t{root}\t{node}\t{}\t{}\t{:.9}",
                    d.calls, d.rows, d.seconds
                );
            }
        });
    }
}

/// Physical buckets for a shared scan. A multi-value filter can subscribe to
/// several buckets. Fact entries group by family so other filters share its scan.
fn buckets(run: &Run, v: &ViewSpec) -> serde_json::Value {
    let (base, column) = match v.base {
        Native::Tree => ("tree", Some(1)),
        Native::Child => ("child", Some(3)),
        Native::Location => ("location", None),
        Native::Token => ("token", None),
        Native::Fact(i) => return serde_json::json!([["fact", run.fact_rels[usize::from(i)].0]]),
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
                return serde_json::json!(
                    names.iter().map(|n| [base, n.as_ref()]).collect::<Vec<_>>()
                );
            }
        }
    }
    serde_json::json!([[base]])
}

/// Current view_rows entries. Fact column/value lookup follows the conjunct and
/// column order of lit_pick. The child relation presently reads every child.
fn current_entries(run: &mut Run, v: &ViewSpec) -> serde_json::Value {
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
    let (prefix, values) = match v.base {
        Native::Tree => (
            vec!["tree".to_owned()],
            super::kind_set(run, &v.filter, 0, v.base, &[]),
        ),
        Native::Child => return serde_json::json!([["child"]]),
        Native::Fact(i) => {
            let name = run.fact_rels[usize::from(i)].0.clone();
            let (_, names, types) = run.catalog(&name).expect("loaded fact");
            let mut cols = Vec::new();
            let mut prefix = vec!["fact".to_owned(), name];
            let mut found = None;
            'conjuncts: for e in &v.filter {
                cols.clear();
                columns(e, &mut cols);
                for &c in &cols {
                    if types[c] == Ty::Sym
                        && let Some(vals) = kinds_required(e, 0, c)
                    {
                        prefix.push(names[c].clone());
                        found = Some(vals);
                        break 'conjuncts;
                    }
                }
            }
            (prefix, found)
        }
        _ => return buckets(run, v),
    };
    let Some(mut values) = values else {
        return serde_json::json!([prefix]);
    };
    values.sort_unstable();
    values.dedup();
    serde_json::json!(
        values
            .into_iter()
            .map(|sym| {
                let mut key = prefix.clone();
                key.push(String::from_utf8_lossy(run.text(sym)).into_owned());
                key
            })
            .collect::<Vec<_>>()
    )
}
