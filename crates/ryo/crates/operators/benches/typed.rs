//! Spike (ryo-vhuy): today's kernels, planned and run as the engine runs them (demand-driven, one file per iteration, the
//! tail held then released, the file ended), against a hand-written typed and fused executor over the same rows: columns as
//! `Vec<u32>` (ids by position, texts by per-file code), a selection vector instead of copied rows, ids addressed by position
//! in a stamped scratch array instead of a hash index, a chain fused into one loop per input row. Each shape is run at the
//! recorder's p25, median and p90 sizes and at one row (the fixed cost). The outputs are compared row for row.
//! Prints one JSON line per shape and size on stdout and each shape's physical plan on stderr.
//! Run: `RYO_LAZY=1 cargo bench -p idiomatic-sh-ryo-operators --bench typed --features fixtures`.

#![expect(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "Spike (ryo-vhuy): a throwaway microbenchmark; a fault is the failure, and plain indexing is the typed loop measured"
)]

use rustc_hash::{FxHashMap, FxHashSet};
use ryo_operators::domain::physical_plan::fixtures::Step;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use ryo_operators::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use ryo_operators::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeRows, PlanInput, PlanInputList, PlanNodeId,
};
use ryo_operators::inbound::use_cases::plan::interface::PlanUseCase as _;
use ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
use ryo_operators::inbound::use_cases::run::fixtures::{TableInputs, TextResources};
use ryo_operators::inbound::use_cases::run::interface::RunUseCase as _;
use ryo_operators::inbound::use_cases::run::use_case::RunUseCaseImpl;
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{
    FileId, Integer, Offset, Position, Relation, Span, Value,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input, JoinColumn, LiteralText, Operand, PlanComparison,
    QueryLiteral, RepeatedRows, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};
use std::hint::black_box;
use std::time::{Duration, Instant};

/// The texts of the synthetic file, each one of its symbols.
const TEXTS: &[&str] = &[
    "zzz", "name", "call", "attribute", "string", "kind", "function", "class", "body", "value",
    "func", "args", "target", "k0", "k1", "k2", "k3", "k4", "k5", "k6", "k7", "k8", "k9", "n0",
    "n1", "n2", "n3", "n4", "n5", "n6", "n7", "n8", "n9", "n10", "n11", "n12", "n13", "n14",
    "n15", "n16", "n17", "n18", "n19",
];
/// The field texts of `semantic_child`.
const FIELDS: &[&str] = &["body", "value", "func", "args", "target"];
/// The names of `defines`.
const NAMES: usize = 20;
/// No position: the end of a chain.
const NONE: u32 = u32::MAX;

/// A small deterministic generator.
struct Gen(u64);
impl Gen {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % (n.max(1) as u64)) as usize
    }
    fn chance(&mut self, p: f64) -> bool {
        (self.next() % 1_000_000) as f64 / 1_000_000.0 < p
    }
}

/// The open file's texts and their codes.
struct Texts {
    resources: TextResources,
    code: FxHashMap<&'static str, u32>,
}
impl Texts {
    fn new() -> Self {
        let resources = TextResources::new(TEXTS);
        let code = TEXTS
            .iter()
            .map(|text| match resources.symbol(text) {
                Value::Symbol(symbol) => (*text, symbol.0),
                _ => unreachable!("a text is a symbol"),
            })
            .collect();
        Self { resources, code }
    }
    fn sym(&self, text: &str) -> Value {
        Value::Symbol(ryo_relations::domain::terms::types::Symbol(self.code[text]))
    }
}

fn node(position: usize) -> Value {
    Value::Node(node_id(FileId(0), Position(position as u32)))
}
fn span(position: usize) -> Value {
    Value::Span(Span {
        start: Offset(position as u32),
        end: Offset(position as u32 + 1),
    })
}

/// A value as the typed executor holds it, and as both outputs are compared.
fn canon(value: Value) -> u64 {
    match value {
        Value::Node(id) => id.0 & 0xffff_ffff,
        Value::Symbol(symbol) => u64::from(symbol.0),
        Value::Name(_) => u64::MAX,
        Value::File(file) => u64::from(file.0),
        Value::Span(span) => (u64::from(span.start.0) << 32) | u64::from(span.end.0),
        Value::Number(number) => number.0 as u64,
        Value::Flag(_) => 0,
    }
}

/// One relation's rows: as the engine's table, as canonical rows, and as the typed executor's `u32` columns.
struct Rows {
    relation: Relation,
    table: Table,
    canon: Vec<Vec<u64>>,
    cols: Vec<Vec<u32>>,
}
fn rows_of(relation: Relation, rows: Vec<Vec<Value>>) -> Rows {
    let width = kinds(relation).len();
    let canon: Vec<Vec<u64>> = rows.iter().map(|row| row.iter().map(|v| canon(*v)).collect()).collect();
    let cols = (0..width)
        .map(|c| canon.iter().map(|row| row[c] as u32).collect())
        .collect();
    let count = rows.len();
    Rows {
        relation,
        table: Table {
            kinds: kinds(relation).to_vec(),
            cells: rows.into_iter().flatten().collect(),
            count: RowCount(count),
        },
        canon,
        cols,
    }
}

/// `semantic_syntax`: `rows` rows over positions drawn from `0..file` (distinct when `distinct`), kind `name` with
/// probability `named`, else another kind.
fn syntax(texts: &Texts, g: &mut Gen, rows: usize, file: usize, named: f64, kind: &str) -> Rows {
    let positions = positions(g, rows, file);
    let others = ["call", "attribute", "string", "kind"];
    let rows = positions
        .into_iter()
        .map(|p| {
            let k = if g.chance(named) { kind } else { others[g.below(others.len())] };
            vec![node(p), texts.sym(k), texts.sym(&format!("k{}", g.below(10))), Value::File(FileId(0)), span(p)]
        })
        .collect();
    rows_of(Relation::SemanticSyntax, rows)
}

/// `rows` distinct positions of `0..file` in file order (all of them when `rows >= file`).
fn positions(g: &mut Gen, rows: usize, file: usize) -> Vec<usize> {
    if rows >= file {
        return (0..rows).collect();
    }
    let mut picked: Vec<usize> = (0..file).collect();
    for at in 0..rows {
        let other = at + g.below(file - at);
        picked.swap(at, other);
    }
    picked.truncate(rows);
    picked.sort_unstable();
    picked
}

/// `semantic_child`: `rows` rows whose children are distinct positions of `0..file`, each with a random earlier parent.
fn children(texts: &Texts, g: &mut Gen, rows: usize, file: usize) -> Rows {
    let rows = positions(g, rows, file)
        .into_iter()
        .enumerate()
        .map(|(at, child)| {
            vec![
                node(g.below(child.max(1))),
                node(child),
                Value::Number(Integer((at % 5) as i64)),
                texts.sym(FIELDS[g.below(FIELDS.len())]),
                Value::File(FileId(0)),
            ]
        })
        .collect();
    rows_of(Relation::SemanticChild, rows)
}

/// `defines`: `rows` rows, kind `function` with probability `function`, column 4 the row's own node with probability
/// `own`, name one of `NAMES`.
fn defines(texts: &Texts, g: &mut Gen, rows: usize, function: f64, own: f64) -> Rows {
    let rows = (0..rows)
        .map(|at| {
            let kind = if g.chance(function) { "function" } else { ["class", "kind", "name"][g.below(3)] };
            let other = if g.chance(own) { at } else { g.below(rows) };
            vec![
                node(at),
                texts.sym(kind),
                texts.sym("kind"),
                texts.sym(&format!("n{}", g.below(NAMES))),
                node(other),
                Value::File(FileId(0)),
                span(at),
            ]
        })
        .collect();
    rows_of(Relation::Defines, rows)
}

// ---- the logical program's parts ----

fn literal(test: Test, column: u32, text: &str) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ComparisonOrigin::Unrecorded,
    }
}
fn columns(test: Test, left: u32, right: u32) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(left)),
        right: Operand::Column(ColumnIndex(right)),
        origin: ComparisonOrigin::Unrecorded,
    }
}
#[derive(Clone, Copy)]
enum From {
    Relation(Relation),
    Step(u32),
}
impl From {
    fn input(self) -> Input {
        match self {
            Self::Relation(r) => Input::Relation(r),
            Self::Step(s) => Input::Step(StepId(s)),
        }
    }
    fn plan(self) -> PlanInput {
        match self {
            Self::Relation(r) => PlanInput::Relation(r),
            Self::Step(s) => PlanInput::Node(PlanNodeId(s)),
        }
    }
}
fn select(from: From, comparisons: Vec<PlanComparison>) -> Step {
    Step {
        operation: Operation::Select {
            from: from.input(),
            comparisons,
            kept: Vec::new(),
        },
        inputs: vec![from.plan()],
    }
}
/// Every row of `relation`, held: a select whose comparison every row passes.
fn every(relation: Relation, text_column: u32) -> Step {
    select(From::Relation(relation), vec![literal(Test::NotEqual, text_column, "zzz")])
}
fn project(from: From, cols: &[u32], distinct: RepeatedRows) -> Step {
    Step {
        operation: Operation::Project {
            from: from.input(),
            columns: cols.iter().map(|c| ColumnIndex(*c)).collect(),
            distinct,
        },
        inputs: vec![from.plan()],
    }
}
fn join(left: From, right: From, left_on: u32, right_on: u32, output: &[(Side, u32)]) -> Step {
    Step {
        operation: Operation::Join {
            left: left.input(),
            right: right.input(),
            left_on: vec![ColumnIndex(left_on)],
            right_on: vec![ColumnIndex(right_on)],
            output: output
                .iter()
                .map(|(side, column)| JoinColumn {
                    side: *side,
                    column: ColumnIndex(*column),
                })
                .collect(),
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        },
        inputs: vec![left.plan(), right.plan()],
    }
}
fn antijoin(left: From, right: From, left_on: u32, right_on: u32) -> Step {
    Step {
        operation: Operation::Antijoin {
            left: left.input(),
            right: right.input(),
            left_on: vec![ColumnIndex(left_on)],
            right_on: vec![ColumnIndex(right_on)],
        },
        inputs: vec![left.plan(), right.plan()],
    }
}
fn union(left: From, right: From) -> Step {
    Step {
        operation: Operation::Union {
            left: left.input(),
            right: right.input(),
        },
        inputs: vec![left.plan(), right.plan()],
    }
}

// ---- copies of a file's rows ----

/// One file's tables, in the order its shape names them.
type Data = Vec<Rows>;

/// Which copy of the file's tables the next run reads: `span` copies in turn (one: warm, every row in cache; many: cold,
/// each run's input rows evicted since its last read, as a file's rows are in the scan), `last` the one read last.
#[derive(Default)]
struct Turn {
    at: usize,
    last: usize,
    span: usize,
}
impl Turn {
    #[inline]
    fn next(&mut self) -> usize {
        self.last = self.at;
        self.at += 1;
        if self.at >= self.span.max(1) {
            self.at = 0;
        }
        self.last
    }
    fn spanning(&mut self, span: usize) {
        self.span = span;
        self.at = 0;
        self.last = 0;
    }
}

/// How many copies of a file's tables make its runs cold: about 128 MiB of cells, at most 4,096 copies.
fn copies_for(data: &Data) -> usize {
    let bytes: usize = data.iter().map(|rows| rows.table.cells.len() * 16 + 1).sum();
    ((128 << 20) / bytes).clamp(1, 4096)
}

/// `make` run once per copy, each with its own seed; the copies after the first keep only their columns and the inputs.
fn copies(texts: &Texts, seed: u64, make: &dyn Fn(&Texts, &mut Gen) -> Data) -> Vec<Data> {
    let first = make(texts, &mut Gen(seed));
    let count = copies_for(&first);
    let mut all = vec![first];
    all.extend((1..count).map(|at| make(texts, &mut Gen(seed.wrapping_mul(7919).wrapping_add(at as u64 * 104_729)))));
    all
}

// ---- today's path ----

/// One shape's program, planned once, run once per file as the engine runs it.
struct Engine {
    plan: PhysicalPlan,
    run: RunUseCaseImpl,
    buffers: FileBuffers,
    inputs: Vec<TableInputs>,
    setup: Vec<PlanNodeId>,
    timed: Vec<PlanNodeId>,
    turn: Turn,
}
impl Engine {
    fn new(steps: &[Step], setup: &[u32], timed: &[u32], data: &[Data]) -> Self {
        let mut program = LogicalProgram::new();
        for step in steps {
            let _id = program.node(LogicalNode {
                operation: &step.operation,
                inputs: PlanInputList(&step.inputs),
                bindings: NodeBindings(&[]),
                scope: StepScope::PerFile,
            });
        }
        for (rule, node) in setup.iter().chain(timed).enumerate() {
            program.emit(RuleOrdinal(rule as u32), PlanNodeId(*node));
        }
        let plan = PlanUseCaseImpl.plan(&program).expect("the shape plans");
        let inputs = data
            .iter()
            .map(|tables| {
                let held: Vec<(Relation, &Table)> = tables.iter().map(|t| (t.relation, &t.table)).collect();
                TableInputs::new(&held)
            })
            .collect();
        Self {
            plan,
            run: RunUseCaseImpl::new(std::sync::Arc::new(
                ryo_operators::outbound::matching::factory::use_case(),
            )),
            buffers: FileBuffers::default(),
            inputs,
            setup: setup.iter().map(|n| PlanNodeId(*n)).collect(),
            timed: timed.iter().map(|n| PlanNodeId(*n)).collect(),
            turn: Turn::default(),
        }
    }
    /// One file: the setup nodes made, then the timed nodes made and released; the file ended. Answers the timed nanoseconds.
    fn once(&mut self, texts: &mut Texts) -> u64 {
        let inputs = &self.inputs[self.turn.next()];
        for node in &self.setup {
            self.run
                .run(&self.plan, *node, &mut self.buffers, inputs, &mut texts.resources)
                .expect("a setup node runs");
        }
        let started = Instant::now();
        for node in &self.timed {
            self.run
                .run(&self.plan, *node, &mut self.buffers, inputs, &mut texts.resources)
                .expect("a timed node runs");
        }
        for node in &self.timed {
            self.buffers.release(*node);
        }
        let took = started.elapsed().as_nanos() as u64;
        self.buffers.ended();
        took
    }
    /// The timed nodes' rows over the first copy, canonical, for one file.
    fn rows(&mut self, texts: &mut Texts) -> Vec<Vec<Vec<u64>>> {
        for node in self.setup.iter().chain(&self.timed) {
            self.run
                .run(&self.plan, *node, &mut self.buffers, &self.inputs[0], &mut texts.resources)
                .expect("a node runs");
        }
        let answered = self
            .timed
            .iter()
            .map(|node| match self.buffers.rows(*node) {
                NodeRows::Materialized(table) => {
                    let width = table.kinds.len().max(1);
                    table.cells.chunks(width).take(table.count.0).map(|row| row.iter().map(|v| canon(*v)).collect()).collect()
                }
                NodeRows::Absent => Vec::new(),
            })
            .collect();
        self.buffers.ended();
        answered
    }
}

// ---- the typed path ----

/// A position-addressed index: for each key position, its rows in row order, valid only where `stamp` holds this run's
/// generation. Kept by the worker across files, never cleared.
#[derive(Default)]
struct Stamped {
    generation: u32,
    stamp: Vec<u32>,
    head: Vec<u32>,
    next: Vec<u32>,
}
impl Stamped {
    fn build(&mut self, keys: &[u32]) {
        self.generation = self.generation.wrapping_add(1);
        if self.next.len() < keys.len() {
            self.next.resize(keys.len(), NONE);
        }
        for (row, key) in keys.iter().enumerate().rev() {
            let key = *key as usize;
            if key >= self.stamp.len() {
                self.stamp.resize(key + 1, 0);
                self.head.resize(key + 1, NONE);
            }
            if self.stamp[key] != self.generation {
                self.stamp[key] = self.generation;
                self.head[key] = NONE;
            }
            self.next[row] = self.head[key];
            self.head[key] = row as u32;
        }
    }
    #[inline]
    fn first(&self, key: u32) -> u32 {
        let key = key as usize;
        if key < self.stamp.len() && self.stamp[key] == self.generation { self.head[key] } else { NONE }
    }
}

/// One shape's typed executor and its output.
trait Typed {
    /// One file's run over the next copy.
    fn run(&mut self);
    /// The output of the last run, canonical (only over the first copy, which keeps its canonical rows).
    fn rows(&self) -> Vec<Vec<Vec<u64>>>;
    fn turn(&mut self) -> &mut Turn;
}

fn gather(source: &[Vec<u64>], selection: &[u32]) -> Vec<Vec<u64>> {
    selection.iter().map(|at| source[*at as usize].clone()).collect()
}
fn pairs(left: &[Vec<u64>], right: &[Vec<u64>], l: &[u32], r: &[u32], output: &[(Side, u32)]) -> Vec<Vec<u64>> {
    l.iter()
        .zip(r)
        .map(|(a, b)| {
            output
                .iter()
                .map(|(side, c)| match side {
                    Side::Left => left[*a as usize][*c as usize],
                    Side::Right => right[*b as usize][*c as usize],
                })
                .collect()
        })
        .collect()
}
fn cols_rows(cols: &[Vec<u32>]) -> Vec<Vec<u64>> {
    (0..cols.first().map_or(0, Vec::len)).map(|r| cols.iter().map(|c| u64::from(c[r])).collect()).collect()
}

/// The typed executors' common state: the copies, the turn, the per-file text codes.
struct Base<'a> {
    data: &'a [Data],
    code: &'a FxHashMap<&'static str, u32>,
    turn: Turn,
}
impl<'a> Base<'a> {
    fn new(data: &'a [Data], code: &'a FxHashMap<&'static str, u32>) -> Self {
        Self { data, code, turn: Turn::default() }
    }
    fn last(&self) -> &'a Data {
        &self.data[self.turn.last]
    }
}

/// (1) Two filters over `defines`, fused: kind is `function`, and column 0 is column 4.
struct FilterChain<'a> {
    base: Base<'a>,
    sel: Vec<u32>,
}
impl Typed for FilterChain<'_> {
    fn run(&mut self) {
        let function = self.base.code["function"];
        let rel = &self.base.data[self.base.turn.next()][0];
        let (kind, a, b) = (&rel.cols[1], &rel.cols[0], &rel.cols[4]);
        self.sel.clear();
        for row in 0..kind.len() {
            if kind[row] == function && a[row] == b[row] {
                self.sel.push(row as u32);
            }
        }
    }
    fn rows(&self) -> Vec<Vec<Vec<u64>>> {
        vec![gather(&self.base.last()[0].canon, &self.sel)]
    }
    fn turn(&mut self) -> &mut Turn {
        &mut self.base.turn
    }
}

/// (1b) A filter over a held table: kind is `name`.
struct FilterNode<'a> {
    base: Base<'a>,
    sel: Vec<u32>,
}
impl Typed for FilterNode<'_> {
    fn run(&mut self) {
        let name = self.base.code["name"];
        let kind = &self.base.data[self.base.turn.next()][0].cols[1];
        self.sel.clear();
        for (row, k) in kind.iter().enumerate() {
            if *k == name {
                self.sel.push(row as u32);
            }
        }
    }
    fn rows(&self) -> Vec<Vec<Vec<u64>>> {
        vec![gather(&self.base.last()[0].canon, &self.sel)]
    }
    fn turn(&mut self) -> &mut Turn {
        &mut self.base.turn
    }
}

/// Distinct output columns of two `u32` keys: a hash set of the packed pair, the worker's, cleared per run.
#[derive(Default)]
struct Distinct2 {
    seen: FxHashSet<u64>,
    out: [Vec<u32>; 2],
}
impl Distinct2 {
    fn start(&mut self) {
        self.seen.clear();
        self.out[0].clear();
        self.out[1].clear();
    }
    #[inline]
    fn add(&mut self, a: u32, b: u32) {
        if self.seen.insert((u64::from(a) << 32) | u64::from(b)) {
            self.out[0].push(a);
            self.out[1].push(b);
        }
    }
}

/// (2) A projection of a held table on two columns, repeated rows dropped.
struct ProjectDropped<'a> {
    base: Base<'a>,
    cols: [usize; 2],
    distinct: Distinct2,
}
impl Typed for ProjectDropped<'_> {
    fn run(&mut self) {
        let rel = &self.base.data[self.base.turn.next()][0];
        let (a, b) = (&rel.cols[self.cols[0]], &rel.cols[self.cols[1]]);
        self.distinct.start();
        for row in 0..a.len() {
            self.distinct.add(a[row], b[row]);
        }
    }
    fn rows(&self) -> Vec<Vec<Vec<u64>>> {
        vec![cols_rows(&self.distinct.out)]
    }
    fn turn(&mut self) -> &mut Turn {
        &mut self.base.turn
    }
}

/// (2w) A projection of a held table on three columns, repeated rows dropped: the recorder's median width.
#[derive(Default)]
struct Distinct3 {
    seen: FxHashSet<u128>,
    out: [Vec<u32>; 3],
}
struct ProjectDropped3<'a> {
    base: Base<'a>,
    cols: [usize; 3],
    distinct: Distinct3,
}
impl Typed for ProjectDropped3<'_> {
    fn run(&mut self) {
        let rel = &self.base.data[self.base.turn.next()][0];
        let (a, b, c) = (&rel.cols[self.cols[0]], &rel.cols[self.cols[1]], &rel.cols[self.cols[2]]);
        let d = &mut self.distinct;
        d.seen.clear();
        for out in &mut d.out {
            out.clear();
        }
        for row in 0..a.len() {
            let key = (u128::from(a[row]) << 64) | (u128::from(b[row]) << 32) | u128::from(c[row]);
            if d.seen.insert(key) {
                d.out[0].push(a[row]);
                d.out[1].push(b[row]);
                d.out[2].push(c[row]);
            }
        }
    }
    fn rows(&self) -> Vec<Vec<Vec<u64>>> {
        vec![cols_rows(&self.distinct.out)]
    }
    fn turn(&mut self) -> &mut Turn {
        &mut self.base.turn
    }
}

/// (3) An inner probe of table 0 into table 1 on one id key; the output is the matched pairs (a selection on each side).
struct Probe<'a> {
    base: Base<'a>,
    left_on: usize,
    right_on: usize,
    output: Vec<(Side, u32)>,
    index: Stamped,
    l: Vec<u32>,
    r: Vec<u32>,
}
impl Typed for Probe<'_> {
    fn run(&mut self) {
        let data = &self.base.data[self.base.turn.next()];
        self.index.build(&data[1].cols[self.right_on]);
        self.l.clear();
        self.r.clear();
        for (row, key) in data[0].cols[self.left_on].iter().enumerate() {
            let mut at = self.index.first(*key);
            while at != NONE {
                self.l.push(row as u32);
                self.r.push(at);
                at = self.index.next[at as usize];
            }
        }
    }
    fn rows(&self) -> Vec<Vec<Vec<u64>>> {
        let data = self.base.last();
        vec![pairs(&data[0].canon, &data[1].canon, &self.l, &self.r, &self.output)]
    }
    fn turn(&mut self) -> &mut Turn {
        &mut self.base.turn
    }
}

/// (4) An anti probe of table 0 against table 1 on one id key: the left rows whose key no right row holds.
struct Anti<'a> {
    base: Base<'a>,
    left_on: usize,
    right_on: usize,
    index: Stamped,
    sel: Vec<u32>,
}
impl Typed for Anti<'_> {
    fn run(&mut self) {
        let data = &self.base.data[self.base.turn.next()];
        self.index.build(&data[1].cols[self.right_on]);
        self.sel.clear();
        for (row, key) in data[0].cols[self.left_on].iter().enumerate() {
            if self.index.first(*key) == NONE {
                self.sel.push(row as u32);
            }
        }
    }
    fn rows(&self) -> Vec<Vec<Vec<u64>>> {
        vec![gather(&self.base.last()[0].canon, &self.sel)]
    }
    fn turn(&mut self) -> &mut Turn {
        &mut self.base.turn
    }
}

/// (5) filter -> project -> probe -> project (repeats dropped), fused: one loop over the head relation's rows (table 0),
/// probing table 1 by position.
struct Chain<'a> {
    base: Base<'a>,
    index: Stamped,
    distinct: Distinct2,
}
impl Typed for Chain<'_> {
    fn run(&mut self) {
        let name = self.base.code["name"];
        let data = &self.base.data[self.base.turn.next()];
        let (head, right) = (&data[0], &data[1]);
        self.index.build(&right.cols[1]);
        self.distinct.start();
        let (id, kind) = (&head.cols[0], &head.cols[1]);
        let (parent, field) = (&right.cols[0], &right.cols[3]);
        for row in 0..kind.len() {
            if kind[row] != name {
                continue;
            }
            let mut at = self.index.first(id[row]);
            while at != NONE {
                self.distinct.add(parent[at as usize], field[at as usize]);
                at = self.index.next[at as usize];
            }
        }
    }
    fn rows(&self) -> Vec<Vec<Vec<u64>>> {
        vec![cols_rows(&self.distinct.out)]
    }
    fn turn(&mut self) -> &mut Turn {
        &mut self.base.turn
    }
}

/// (6) Distinct over the union of two tables of two id columns.
struct Union2<'a> {
    base: Base<'a>,
    a_cols: [usize; 2],
    b_cols: [usize; 2],
    distinct: Distinct2,
}
impl Typed for Union2<'_> {
    fn run(&mut self) {
        let data = &self.base.data[self.base.turn.next()];
        self.distinct.start();
        for (rows, cols) in [(&data[0], self.a_cols), (&data[1], self.b_cols)] {
            let (x, y) = (&rows.cols[cols[0]], &rows.cols[cols[1]]);
            for row in 0..x.len() {
                self.distinct.add(x[row], y[row]);
            }
        }
    }
    fn rows(&self) -> Vec<Vec<Vec<u64>>> {
        vec![cols_rows(&self.distinct.out)]
    }
    fn turn(&mut self) -> &mut Turn {
        &mut self.base.turn
    }
}

/// (7) Lookups of several names in one relation: one index by text code, built once per file, read once per lookup.
struct Lookups<'a> {
    base: Base<'a>,
    names: Vec<&'static str>,
    index: Stamped,
    sel: Vec<Vec<u32>>,
}
impl Typed for Lookups<'_> {
    fn run(&mut self) {
        let rel = &self.base.data[self.base.turn.next()][0];
        self.index.build(&rel.cols[3]);
        for (at, name) in self.names.iter().enumerate() {
            let out = &mut self.sel[at];
            out.clear();
            let mut row = self.index.first(self.base.code[name]);
            while row != NONE {
                out.push(row);
                row = self.index.next[row as usize];
            }
        }
    }
    fn rows(&self) -> Vec<Vec<Vec<u64>>> {
        self.sel.iter().map(|sel| gather(&self.base.last()[0].canon, sel)).collect()
    }
    fn turn(&mut self) -> &mut Turn {
        &mut self.base.turn
    }
}

// ---- measuring ----

/// The mean nanoseconds of `once` with the slowest 1% of calls left out (interrupts), after a warm-up, over at least 0.4 s
/// or 400,000 calls. A mean, not a median: the clock ticks every 41.7 ns, so one call's reading is quantised, and only the
/// mean of many readings is not.
fn timed(mut once: impl FnMut() -> u64) -> f64 {
    for _ in 0..30 {
        let _ns = black_box(once());
    }
    let mut samples = Vec::new();
    let started = Instant::now();
    while started.elapsed() < Duration::from_millis(400) && samples.len() < 400_000 {
        samples.push(once());
    }
    samples.sort_unstable();
    let kept = &samples[..samples.len() * 99 / 100 + 1];
    kept.iter().sum::<u64>() as f64 / kept.len() as f64
}

fn typed_timed(typed: &mut dyn Typed) -> f64 {
    timed(|| {
        let started = Instant::now();
        typed.run();
        let took = started.elapsed().as_nanos() as u64;
        let _kept = black_box(&typed);
        took
    })
}

/// What one shape at one size measured, warm and cold.
struct Measured {
    shape: &'static str,
    size: &'static str,
    rows_in: usize,
    rows_out: usize,
    runs: usize,
    copies: usize,
    today_ns: f64,
    typed_ns: f64,
    today_cold_ns: f64,
    typed_cold_ns: f64,
    rows_equal: bool,
    order_equal: bool,
}

fn measure(
    shape: &'static str,
    size: &'static str,
    rows_in: usize,
    copies: usize,
    engine: &mut Engine,
    texts: &mut Texts,
    typed: &mut dyn Typed,
) -> Measured {
    let today = engine.rows(texts);
    typed.turn().spanning(1);
    typed.run();
    let mine = typed.rows();
    let order_equal = today == mine;
    let sorted = |mut v: Vec<Vec<Vec<u64>>>| {
        for node in &mut v {
            node.sort_unstable();
        }
        v
    };
    let rows_equal = sorted(today.clone()) == sorted(mine.clone());
    if !rows_equal {
        eprintln!("{shape} {size}: today {} rows, typed {} rows; first today {:?} typed {:?}",
            today.iter().map(Vec::len).sum::<usize>(), mine.iter().map(Vec::len).sum::<usize>(),
            today.first().and_then(|n| n.first()), mine.first().and_then(|n| n.first()));
    }
    // The clock's own cost, read the same way, comes off every reading.
    let clock = timed(|| {
        let started = Instant::now();
        black_box(started.elapsed().as_nanos() as u64)
    });
    let mut pair = |span: usize, engine: &mut Engine, texts: &mut Texts| {
        engine.turn.spanning(span);
        typed.turn().spanning(span);
        let today = (timed(|| engine.once(texts)) - clock).max(0.0);
        let mine = (typed_timed(typed) - clock).max(0.0);
        (today, mine)
    };
    let (today_ns, typed_ns) = pair(1, engine, texts);
    let (today_cold_ns, typed_cold_ns) = pair(copies, engine, texts);
    Measured {
        shape,
        size,
        rows_in,
        rows_out: today.iter().map(Vec::len).sum(),
        runs: engine.timed.len(),
        copies,
        today_ns,
        typed_ns,
        today_cold_ns,
        typed_cold_ns,
        rows_equal,
        order_equal,
    }
}

/// One shape's sizes: label and its two parameters.
type Sizes = [(&'static str, [usize; 2]); 4];

/// One shape at each size: its copies made by `make`, its program (steps, setup nodes, timed nodes, rows in) by `build`, its
/// typed executor over the copies by `typed`.
fn shape(
    name: &'static str,
    sizes: &Sizes,
    texts: &mut Texts,
    seed: u64,
    make: &dyn Fn(&Texts, &mut Gen, [usize; 2]) -> Data,
    build: &dyn Fn(&[Data], [usize; 2]) -> (Vec<Step>, Vec<u32>, Vec<u32>, usize),
    typed: &dyn for<'d> Fn(&'d [Data]) -> Box<dyn Typed + 'd>,
) -> Vec<Measured> {
    sizes
        .iter()
        .map(|(label, size)| {
            let mut data = copies(texts, seed, &|texts, g| make(texts, g, *size));
            let (steps, setup, timed, rows_in) = build(&data, *size);
            let mut engine = Engine::new(&steps, &setup, &timed, &data);
            // The engine holds its own columns of every copy; past the first, only the typed columns stay.
            for copy in data.iter_mut().skip(1) {
                for rows in copy {
                    rows.canon = Vec::new();
                    rows.table.cells = Vec::new();
                }
            }
            if *label == "p50" {
                eprintln!("{name} plan: {:?}", engine.plan);
            }
            let mut executor = typed(&data);
            let measured = measure(name, label, rows_in, data.len(), &mut engine, texts, executor.as_mut());
            eprintln!("{name} {label}: {} copies, today {:.0}/{:.0} ns, typed {:.0}/{:.0} ns (warm/cold)", data.len(),
                measured.today_ns, measured.today_cold_ns, measured.typed_ns, measured.typed_cold_ns);
            measured
        })
        .collect()
}

// ---- batched steps (ryo-qs8s cost model) ----

/// One file's per-file buckets for the batched steps: `n` nodes by position, each with a kind code, a parent and a field
/// code; the children of each node as an offsets-plus-ids edge list (with each edge's field code); a position-addressed
/// index over a right id column (half the nodes, `NONE` where no row); the candidates, every node in a random order.
struct Bucket {
    kind: Vec<u32>,
    parent: Vec<u32>,
    field: Vec<u32>,
    offsets: Vec<u32>,
    child: Vec<u32>,
    child_field: Vec<u32>,
    right: Vec<u32>,
    head: Vec<u32>,
    candidates: Vec<u32>,
}

fn bucket(g: &mut Gen, n: usize) -> Bucket {
    let kind: Vec<u32> = (0..n).map(|_| g.below(8) as u32).collect();
    let parent: Vec<u32> = (0..n).map(|at| g.below(at.max(1)) as u32).collect();
    let field: Vec<u32> = (0..n).map(|_| g.below(5) as u32).collect();
    let (offsets, child, child_field) = edges(&parent, &field);
    let right: Vec<u32> = (0..n / 2 + 1).map(|_| g.below(n) as u32).collect();
    let mut head = vec![NONE; n];
    for (row, key) in right.iter().enumerate().rev() {
        head[*key as usize] = row as u32;
    }
    let mut candidates: Vec<u32> = (0..n as u32).collect();
    for at in (1..n).rev() {
        candidates.swap(at, g.below(at + 1));
    }
    Bucket { kind, parent, field, offsets, child, child_field, right, head, candidates }
}

/// The children of each node as an edge list by a counting sort over the parent column: offsets, child ids, field codes.
fn edges(parent: &[u32], field: &[u32]) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let n = parent.len();
    let mut offsets = vec![0_u32; n + 1];
    for p in parent.iter().skip(1) {
        offsets[*p as usize + 1] += 1;
    }
    for at in 0..n {
        offsets[at + 1] += offsets[at];
    }
    let mut fill = offsets.clone();
    let mut child = vec![0_u32; n.saturating_sub(1)];
    let mut child_field = vec![0_u32; n.saturating_sub(1)];
    for c in 1..n {
        let p = parent[c] as usize;
        let at = fill[p] as usize;
        child[at] = c as u32;
        child_field[at] = field[c];
        fill[p] += 1;
    }
    (offsets, child, child_field)
}

/// The batch steps' scratch, the worker's: hot across batches.
#[derive(Default)]
struct BatchScratch {
    values: Vec<u32>,
    sel: Vec<u32>,
    pairs: Vec<(u32, u32)>,
    out: Vec<u32>,
    index: Stamped,
}

/// The steps named for the cost model, in order.
const STEPS: [&str; 7] = ["gather", "compare_literal", "probe_position", "exists", "absent", "axis_children_by_field", "build"];

/// One pass over a file's candidates in batches of 1,024, every step timed on its own; answers nanoseconds per step.
fn batched_pass(b: &Bucket, s: &mut BatchScratch, steps: &mut [u64; 7]) -> usize {
    let mut kept = 0;
    for batch in b.candidates.chunks(1024) {
        let t0 = Instant::now();
        s.values.clear();
        s.values.extend(batch.iter().map(|id| b.kind[*id as usize]));
        let t1 = Instant::now();
        s.sel.clear();
        for (at, v) in s.values.iter().enumerate() {
            s.sel.push(at as u32);
            let len = s.sel.len();
            s.sel.truncate(len - usize::from(*v != 3));
        }
        let t2 = Instant::now();
        s.pairs.clear();
        for id in batch {
            let r = b.head[*id as usize];
            if r != NONE {
                s.pairs.push((*id, r));
            }
        }
        let t3 = Instant::now();
        s.out.clear();
        for id in batch {
            if b.head[*id as usize] != NONE {
                s.out.push(*id);
            }
        }
        let t4 = Instant::now();
        s.out.clear();
        for id in batch {
            if b.head[*id as usize] == NONE {
                s.out.push(*id);
            }
        }
        let t5 = Instant::now();
        s.out.clear();
        for id in batch {
            let p = *id as usize;
            for at in b.offsets[p] as usize..b.offsets[p + 1] as usize {
                if b.child_field[at] == 1 {
                    s.out.push(b.child[at]);
                }
            }
        }
        let t6 = Instant::now();
        kept += s.sel.len() + s.pairs.len() + s.out.len();
        for (step, (from, to)) in [(t0, t1), (t1, t2), (t2, t3), (t3, t4), (t4, t5), (t5, t6)].into_iter().enumerate() {
            steps[step] += to.duration_since(from).as_nanos() as u64;
        }
    }
    // The per-file index over an id column: built once per file, ns per row of the column.
    let t = Instant::now();
    s.index.build(&b.right);
    steps[6] += t.elapsed().as_nanos() as u64;
    black_box(kept)
}

/// The batched steps at one file size, warm (one copy) and cold (copies cycled past the caches); one JSON line each.
fn batched(label: &str, n: usize) {
    let mut g = Gen(31 + n as u64);
    let first = bucket(&mut g, n);
    let bytes = n * 4 * 10 + 64;
    let count = ((128 << 20) / bytes).clamp(1, 4096);
    let mut all = vec![first];
    all.extend((1..count).map(|_| bucket(&mut g, n)));
    // The edge list's build, per row: a counting sort of the parent column.
    let started = Instant::now();
    let mut builds = 0_usize;
    while started.elapsed() < Duration::from_millis(300) {
        let b = &all[builds % all.len()];
        let _e = black_box(edges(&b.parent, &b.field));
        builds += 1;
    }
    let edges_ns_per_row = started.elapsed().as_nanos() as f64 / (builds * n) as f64;
    let mut s = BatchScratch::default();
    for (mode, span) in [("warm", 1), ("cold", all.len())] {
        let mut steps = [0_u64; 7];
        let mut passes = 0_usize;
        for at in 0..30 {
            let mut skip = [0_u64; 7];
            let _k = batched_pass(&all[at % span], &mut s, &mut skip);
        }
        let started = Instant::now();
        while started.elapsed() < Duration::from_millis(400) {
            let _k = batched_pass(&all[passes % span], &mut s, &mut steps);
            passes += 1;
        }
        let per: Vec<String> = STEPS
            .iter()
            .zip(steps)
            .enumerate()
            .map(|(at, (name, ns))| {
                let rows = if at == 6 { all[0].right.len() } else { n };
                format!("\"{name}\":{:.2}", ns as f64 / (passes * rows) as f64)
            })
            .collect();
        println!(
            "{{\"batched\":\"{label}\",\"mode\":\"{mode}\",\"nodes\":{n},\"copies\":{},\"passes\":{passes},\"ns_per_candidate\":{{{}}},\"edge_list_build_ns_per_row\":{edges_ns_per_row:.2}}}",
            all.len(),
            per.join(",")
        );
    }
}

fn main() {
    // Spike (ryo-vhuy): `TYPED_ONLY=batched` runs only the batched steps.
    let only_batched = std::env::var_os("TYPED_ONLY").is_some_and(|v| v == "batched");
    for (label, n) in [("small", 60), ("median", 530), ("p90", 7000)] {
        batched(label, n);
    }
    if only_batched {
        return;
    }
    let mut texts = Texts::new();
    // The codes live as long as the bench: every typed executor reads them.
    let code: &'static FxHashMap<&'static str, u32> = Box::leak(Box::new(texts.code.clone()));
    let mut all: Vec<Measured> = Vec::new();
    // Sizes from the recorder (`sizes.json`): one row (the fixed cost), then p25, median and p90 rows in per non-empty run;
    // a base relation's rows per file scale its mean (about 2,900 for the syntax relations) by the Django files' sizes.

    // (1) filter chain over a provider relation: text equality, then id equality.
    all.extend(shape(
        "filter_chain_base",
        &[("one", [1, 0]), ("p25", [60, 0]), ("p50", [530, 0]), ("p90", [7000, 0])],
        &mut texts,
        17,
        &|texts, g, [n, _]| vec![defines(texts, g, n, 0.15, 0.5)],
        &|_, [n, _]| {
            let steps = vec![
                select(From::Relation(Relation::Defines), vec![literal(Test::Equal, 1, "function")]),
                select(From::Step(0), vec![columns(Test::Equal, 0, 4)]),
            ];
            (steps, vec![], vec![1], n)
        },
        &|data| Box::new(FilterChain { base: Base::new(data, code), sel: Vec::new() }),
    ));

    // (1b) filter over a held table: text equality.
    all.extend(shape(
        "filter_node",
        &[("one", [1, 0]), ("p25", [47, 0]), ("p50", [167, 0]), ("p90", [2248, 0])],
        &mut texts,
        18,
        &|texts, g, [n, _]| vec![syntax(texts, g, n, n, 0.5, "name")],
        &|_, [n, _]| {
            let steps = vec![every(Relation::SemanticSyntax, 1), select(From::Step(0), vec![literal(Test::Equal, 1, "name")])];
            (steps, vec![0], vec![1], n)
        },
        &|data| Box::new(FilterNode { base: Base::new(data, code), sel: Vec::new() }),
    ));

    // (2) projection of a held table on (parent, field), repeated rows dropped.
    all.extend(shape(
        "project_dropped",
        &[("one", [1, 0]), ("p25", [14, 0]), ("p50", [82, 0]), ("p90", [507, 0])],
        &mut texts,
        19,
        &|texts, g, [n, _]| vec![children(texts, g, n, n)],
        &|_, [n, _]| {
            let steps = vec![every(Relation::SemanticChild, 3), project(From::Step(0), &[0, 3], RepeatedRows::Dropped)];
            (steps, vec![0], vec![1], n)
        },
        &|data| Box::new(ProjectDropped { base: Base::new(data, code), cols: [0, 3], distinct: Distinct2::default() }),
    ));

    // (2w) the same at the recorder's median width: three columns (parent, ordinal, field).
    all.extend(shape(
        "project_dropped_w3",
        &[("one", [1, 0]), ("p25", [14, 0]), ("p50", [82, 0]), ("p90", [507, 0])],
        &mut texts,
        26,
        &|texts, g, [n, _]| vec![children(texts, g, n, n)],
        &|_, [n, _]| {
            let steps = vec![every(Relation::SemanticChild, 3), project(From::Step(0), &[0, 2, 3], RepeatedRows::Dropped)];
            (steps, vec![0], vec![1], n)
        },
        &|data| Box::new(ProjectDropped3 { base: Base::new(data, code), cols: [0, 2, 3], distinct: Distinct3::default() }),
    ));

    // (3) inner probe of a held table into a provider relation on one id key (unique on the right).
    let output = vec![(Side::Left, 0), (Side::Left, 1), (Side::Right, 0), (Side::Right, 3)];
    let output = &output;
    all.extend(shape(
        "probe_inner_facts",
        &[("one", [1, 1]), ("p25", [9, 60]), ("p50", [30, 530]), ("p90", [344, 7000])],
        &mut texts,
        20,
        &|texts, g, [l, n]| vec![syntax(texts, g, l, n, 1.0, "call"), children(texts, g, n, n)],
        &|_, [l, _]| {
            let steps = vec![every(Relation::SemanticSyntax, 1), join(From::Step(0), From::Relation(Relation::SemanticChild), 0, 1, output)];
            (steps, vec![0], vec![1], l)
        },
        &|data| Box::new(Probe { base: Base::new(data, code), left_on: 0, right_on: 1, output: output.clone(), index: Stamped::default(), l: Vec::new(), r: Vec::new() }),
    ));

    // (3n) inner probe of a held table into a held table on one id key; about a third of the left rows match.
    all.extend(shape(
        "probe_inner_node",
        &[("one", [1, 1]), ("p25", [9, 9]), ("p50", [35, 47]), ("p90", [262, 398])],
        &mut texts,
        21,
        &|texts, g, [l, r]| {
            let file = 3 * l.max(r);
            vec![syntax(texts, g, l, file, 1.0, "call"), children(texts, g, r, file)]
        },
        &|_, [l, _]| {
            let steps = vec![every(Relation::SemanticSyntax, 1), every(Relation::SemanticChild, 3), join(From::Step(0), From::Step(1), 0, 1, output)];
            (steps, vec![0, 1], vec![2], l)
        },
        &|data| Box::new(Probe { base: Base::new(data, code), left_on: 0, right_on: 1, output: output.clone(), index: Stamped::default(), l: Vec::new(), r: Vec::new() }),
    ));

    // (4) anti probe on one id key: held left, held right.
    all.extend(shape(
        "probe_anti_node",
        &[("one", [1, 1]), ("p25", [8, 1]), ("p50", [69, 2]), ("p90", [375, 110])],
        &mut texts,
        22,
        &|texts, g, [l, r]| {
            let file = 3 * l.max(r);
            vec![syntax(texts, g, l, file, 1.0, "call"), children(texts, g, r, file)]
        },
        &|_, [l, _]| {
            let steps = vec![every(Relation::SemanticSyntax, 1), every(Relation::SemanticChild, 3), antijoin(From::Step(0), From::Step(1), 0, 1)];
            (steps, vec![0, 1], vec![2], l)
        },
        &|data| Box::new(Anti { base: Base::new(data, code), left_on: 0, right_on: 1, index: Stamped::default(), sel: Vec::new() }),
    ));

    // (5) the fused chain: filter (text) -> project (kept) -> inner probe into a held table -> project (repeats dropped).
    all.extend(shape(
        "chain_filter_project_probe_project",
        &[("one", [1, 1]), ("p25", [60, 10]), ("p50", [530, 118]), ("p90", [7000, 496])],
        &mut texts,
        23,
        &|texts, g, [n, r]| vec![syntax(texts, g, n, n, 0.2, "name"), children(texts, g, r, n)],
        &|_, [n, _]| {
            let steps = vec![
                every(Relation::SemanticChild, 3),
                select(From::Relation(Relation::SemanticSyntax), vec![literal(Test::Equal, 1, "name")]),
                project(From::Step(1), &[0, 1], RepeatedRows::Kept),
                join(From::Step(2), From::Step(0), 0, 1, &[(Side::Left, 0), (Side::Right, 0), (Side::Right, 3)]),
                project(From::Step(3), &[1, 2], RepeatedRows::Dropped),
            ];
            (steps, vec![0], vec![4], n)
        },
        &|data| Box::new(Chain { base: Base::new(data, code), index: Stamped::default(), distinct: Distinct2::default() }),
    ));

    // (6) distinct over the union of two held tables of two id columns; half of the second's pairs repeat the first's.
    all.extend(shape(
        "distinct_two_ids",
        &[("one", [1, 0]), ("p25", [3, 0]), ("p50", [26, 0]), ("p90", [282, 0])],
        &mut texts,
        24,
        &|texts, g, [n, _]| {
            let a = children(texts, g, n, n);
            let b = defines(texts, g, n, 0.15, 0.0);
            let mut rows: Vec<Vec<Value>> = b.table.cells.chunks(7).map(<[Value]>::to_vec).collect();
            for row in &mut rows {
                if g.chance(0.5) {
                    let from = &a.table.cells[g.below(n) * 5..];
                    row[0] = from[0];
                    row[4] = from[1];
                }
            }
            vec![a, rows_of(Relation::Defines, rows)]
        },
        &|_, [n, _]| {
            let steps = vec![
                project(From::Relation(Relation::SemanticChild), &[0, 1], RepeatedRows::Kept),
                project(From::Relation(Relation::Defines), &[0, 4], RepeatedRows::Kept),
                union(From::Step(0), From::Step(1)),
            ];
            (steps, vec![0, 1], vec![2], 2 * n)
        },
        &|data| Box::new(Union2 { base: Base::new(data, code), a_cols: [0, 1], b_cols: [0, 4], distinct: Distinct2::default() }),
    ));

    // (7) four lookups of a name in a provider relation: the planner's shared text index and its lookups.
    let names = vec!["n0", "n1", "n2", "n3"];
    let names = &names;
    all.extend(shape(
        "lookup_text",
        &[("one", [1, 0]), ("p25", [60, 0]), ("p50", [530, 0]), ("p90", [7000, 0])],
        &mut texts,
        25,
        &|texts, g, [n, _]| vec![defines(texts, g, n, 0.15, 0.5)],
        &|_, [n, _]| {
            let steps: Vec<Step> = names
                .iter()
                .map(|name| select(From::Relation(Relation::Defines), vec![literal(Test::Equal, 3, name)]))
                .collect();
            (steps, vec![], vec![0, 1, 2, 3], n)
        },
        &|data| Box::new(Lookups { base: Base::new(data, code), names: names.clone(), index: Stamped::default(), sel: vec![Vec::new(); 4] }),
    ));

    for m in &all {
        println!(
            "{{\"shape\":\"{}\",\"size\":\"{}\",\"rows_in\":{},\"rows_out\":{},\"runs\":{},\"copies\":{},\"today_ns\":{:.0},\"typed_ns\":{:.0},\"today_cold_ns\":{:.0},\"typed_cold_ns\":{:.0},\"rows_equal\":{},\"order_equal\":{}}}",
            m.shape, m.size, m.rows_in, m.rows_out, m.runs, m.copies, m.today_ns, m.typed_ns, m.today_cold_ns, m.typed_cold_ns, m.rows_equal, m.order_equal
        );
    }
}
