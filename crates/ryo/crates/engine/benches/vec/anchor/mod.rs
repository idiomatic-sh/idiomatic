//! File-local access paths compiled from the shared logical plan.
use super::*;
use rustc_hash::FxHashSet;
use smallvec::SmallVec;
use std::collections::BTreeMap;
mod compile;
mod expression;
mod runner;
use expression::Select;
pub(crate) use runner::FileIndexes;

type Row = SmallVec<[u64; 8]>;
type Program = Rc<Route>;
#[derive(Clone, Copy)]
enum Arg {
    Outer(usize),
    First(usize),
}
type Args = Vec<(usize, Arg)>;
struct Edge {
    code: Program,
    args: Vec<Arg>,
}
struct Route {
    cols: Vec<usize>,
    inst: Inst,
}

#[derive(Clone)]
enum Source {
    View(Rc<ViewSpec>),
    Shared,
    Relation(String),
}
struct Leaf {
    id: usize,
    source: Source,
    schema: Rc<Schema>,
}
enum Transform {
    Columns(Vec<usize>),
    Append(Vec<u64>),
    Bounds(usize),
    Span(usize, usize),
}
#[derive(Clone, Copy)]
enum Direction {
    Up,
    Down,
    Nearest,
    Parent,
    Child,
}
struct Axis {
    side: Edge,
    other: Program,
    id: usize,
    domain_id: usize,
    inverse: bool,
    dir: Direction,
    up: Program,
    down: Program,
    parent: usize,
    child: usize,
    domain_check: Program,
}
type OrderedKey = (usize, usize, Vec<usize>);
struct Predecessor {
    index: OrderedKey,
    input: Edge,
    left_order: usize,
    partition: Vec<(usize, usize)>,
    project: Vec<(bool, usize)>,
}
#[derive(Clone, Copy)]
struct Probe {
    column: usize,
    parameter: usize,
}
enum Inst {
    Leaf(Rc<Leaf>, Option<Probe>),
    Filter(Edge, Rc<Select>),
    Join(Edge, Edge, usize, Rc<Select>, bool),
    Union(Program, Program),
    Anti(Program, Program, Vec<usize>),
    Transform(Edge, Transform),
    Axis(Axis),
    Predecessor(Predecessor),
}
struct Choice {
    leaf: Rc<Leaf>,
    path: Program,
}
struct Rule {
    choices: Vec<Choice>,
    schema: Rc<Schema>,
}
#[derive(Default)]
pub(crate) struct Stats {
    diagnostic: bool,
    probe_samples: u64,
    probe_seconds: f64,
    emit: f64,
    teardown: f64,
    rules: usize,
    pairs: u64,
    anchors: u64,
    probes: u64,
    builds: u64,
    rows: u64,
    index: f64,
    choice: f64,
    total: f64,
    compile: f64,
    fills: f64,
    shared: f64,
    shared_rows: u64,
    shared_builds: u64,
    tests: u64,
    visits: u64,
}
#[derive(Default)]
pub(crate) struct Programs {
    rules: Vec<Option<Rule>>,
    stats: Stats,
    detail: Vec<(String, u64, u64, u64, f64)>,
}
impl Programs {
    pub(crate) fn load(exec: &mut Exec<'_>, run: &mut Run) -> Self {
        if !sw!("VEC_ANCHOR") {
            return Self::default();
        }
        let t = super::super::thread_cpu();
        let mut compiler = compile::Compiler::new(exec, run);
        let rules: Vec<_> = (0..compiler.exec.plan.emits.len())
            .map(|i| compiler.rule(i))
            .collect();
        let n = rules.iter().flatten().count();
        let diagnostic = std::env::var_os("VEC_RULE_STATS").is_some();
        let detail = if diagnostic {
            compiler
                .exec
                .plan
                .emits
                .iter()
                .map(|e| (e.0.clone(), 0, 0, 0, 0.0))
                .collect()
        } else {
            Vec::new()
        };
        let ns = super::super::thread_cpu() - t;
        eprintln!(
            "anchor compiler: {} emits, {} routes, {:.6} CPU-s",
            n,
            compiler.cache.len(),
            ns
        );
        Self {
            rules,
            detail,
            stats: Stats {
                rules: n,
                compile: ns,
                diagnostic,
                ..Default::default()
            },
        }
    }
    pub(crate) fn covers(&self, emit: usize) -> bool {
        self.rules.get(emit).is_some_and(Option::is_some)
    }
    pub(crate) fn written(&mut self, started: Option<f64>) {
        if let Some(t) = started {
            self.stats.emit += super::super::thread_cpu() - t;
        }
    }
    pub(crate) fn finish_file(&mut self, indexes: FileIndexes) {
        if self.stats.rules != 0 {
            let t = super::super::thread_cpu();
            drop(indexes);
            self.stats.teardown += super::super::thread_cpu() - t;
        }
    }
    pub(crate) fn report(&self) {
        let s = &self.stats;
        for (i, (name, anchors, probes, visits, seconds)) in self.detail.iter().enumerate() {
            if self.covers(i) {
                eprintln!("ANCHOR_RULE {name} {anchors} {probes} {visits} {seconds:.9}");
            }
        }
        if s.diagnostic {
            // Adjacent thread-clock reads give the empty bracket for the sampled lookup.
            let mut null = 0.0;
            for _ in 0..65536 {
                let t = super::super::thread_cpu();
                null += super::super::thread_cpu() - t;
            }
            eprintln!(
                "ANCHOR_PROFILE samples={} seconds={:.9} null_ns={:.3}",
                s.probe_samples,
                s.probe_seconds,
                null * 1e9 / 65536.0
            );
        }
        if s.rules != 0 {
            eprintln!(
                "ANCHOR rules={} pairs={} anchors={} probes={} visits={} tests={} builds={} index_rows={} index_s={:.9} fills_s={:.9} shared_builds={} shared_rows={} shared_s={:.9} choice_s={:.9} run_s={:.9} compile_s={:.9} emit_s={:.9} teardown_s={:.9}",
                s.rules,
                s.pairs,
                s.anchors,
                s.probes,
                s.visits,
                s.tests,
                s.builds,
                s.rows,
                s.index,
                s.fills,
                s.shared_builds,
                s.shared_rows,
                s.shared,
                s.choice,
                s.total,
                s.compile,
                s.emit,
                s.teardown
            );
        }
    }
}
