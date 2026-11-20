//! Spike (ryo-qz0d): a set-at-a-time engine over sorted, packed relations, evaluated over batches of files.
//!
//! A relation is a flat vector of fixed-width tuples of `u64` cells whose first column is the file's ordinal in the batch; an
//! index is a sort order (a sorted vector of key tuples ending in a row number); a join is a merge of two sorted indexes, an
//! anti-join a merge with skip, distinct a sort and a scan. The merged logical program is lowered at load time, rule by rule:
//! a rule whose plan uses an operation this engine does not run is refused and reported, never approximated.
#![allow(
    dead_code,
    missing_docs,
    missing_debug_implementations,
    unreachable_pub,
    unused_results,
    clippy::all,
    reason = "Spike (ryo-qz0d): throwaway prototype for the engine bake-off"
)]

use crate::domain::comparing::collections::comparisons::affix::item::AffixItem;
use crate::domain::comparing::collections::comparisons::equality::item::EqualityItem;
use crate::domain::comparing::collections::comparisons::float_text::item::FloatTextItem;
use crate::domain::comparing::collections::comparisons::glob::item::GlobItem;
use crate::domain::comparing::collections::comparisons::interface::ComparisonsItem;
use crate::domain::comparing::collections::comparisons::membership::item::MembershipItem;
use crate::domain::comparing::collections::comparisons::numeric::item::NumericItem;
use crate::domain::comparing::collections::comparisons::ordering::item::OrderingItem;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::functions::types::MadeText;
use crate::domain::patterns::types::PatternMatcher;
use crate::domain::physical_plan::types::PlanInput;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::table_scratch::types::Patterns;
use crate::domain::table_texts::types::{Datum, TextView};
use ryo_language::domain::elements::service::positions;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::service::text_of;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FileId, Relation, Value};
use ryo_values::domain::plan_parts::service::family_of;
use ryo_values::domain::plan_parts::types::{
    Family, FlagState, Function, Operand, PlanComparison, QueryLiteral, RepeatedRows, Side,
    StepScope, Test, WholeNumber,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

// ------------------------------------------------------------------------------------------------------------------------
// Switches, read once.

struct Switches {
    on: bool,
    batch: usize,
    out: Option<std::path::PathBuf>,
    stats: Option<std::path::PathBuf>,
    cover: Option<std::path::PathBuf>,
    no_guards: bool,
    fill_only: bool,
    fill_all: bool,
}

fn switches() -> &'static Switches {
    static S: OnceLock<Switches> = OnceLock::new();
    S.get_or_init(|| Switches {
        on: std::env::var_os("RYO_SORTED").is_some(),
        batch: std::env::var("RYO_SORTED_BATCH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(256)
            .max(1),
        out: std::env::var_os("RYO_SORTED_OUT").map(Into::into),
        stats: std::env::var_os("RYO_SORTED_STATS").map(Into::into),
        cover: std::env::var_os("RYO_SORTED_COVER").map(Into::into),
        no_guards: std::env::var_os("RYO_SORTED_NO_GUARDS").is_some(),
        fill_only: std::env::var_os("RYO_SORTED_FILL_ONLY").is_some() || std::env::var_os("RYO_SORTED_FILL_ALL").is_some(),
        fill_all: std::env::var_os("RYO_SORTED_FILL_ALL").is_some(),
    })
}

/// Whether the sorted engine replaces the rule pass.
pub fn on() -> bool {
    switches().on
}

// ------------------------------------------------------------------------------------------------------------------------
// Cells: a tag in the top byte, a payload in the low 56 bits.

const TAG_NODE: u64 = 1;
const TAG_TEXT: u64 = 2;
const TAG_FILE: u64 = 4;
const TAG_SPAN: u64 = 5;
const TAG_NUM: u64 = 6;
const TAG_FLAG: u64 = 7;
const PAY: u64 = (1 << 56) - 1;
const SPAN_NONE: u64 = PAY;
const NUM_BIAS: i64 = 1 << 55;

#[inline]
fn tag(cell: u64) -> u64 {
    cell >> 56
}
#[inline]
fn mk(t: u64, payload: u64) -> u64 {
    (t << 56) | (payload & PAY)
}
#[inline]
fn num_cell(n: i64) -> u64 {
    let biased = n.saturating_add(NUM_BIAS).clamp(0, PAY as i64);
    mk(TAG_NUM, biased as u64)
}
#[inline]
fn num_of(cell: u64) -> i64 {
    (cell & PAY) as i64 - NUM_BIAS
}
#[inline]
fn span_cell(start: u32, end: u32) -> u64 {
    if start == u32::MAX && end == 0 {
        return mk(TAG_SPAN, SPAN_NONE);
    }
    debug_assert!(start < (1 << 28) && end < (1 << 28));
    mk(TAG_SPAN, (u64::from(start) << 28) | u64::from(end))
}
#[inline]
fn span_of(cell: u64) -> Option<(u32, u32)> {
    let p = cell & PAY;
    if p == SPAN_NONE {
        None
    } else {
        Some(((p >> 28) as u32, (p & ((1 << 28) - 1)) as u32))
    }
}

// ------------------------------------------------------------------------------------------------------------------------
// The lowered program.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Src {
    Node(u32),
    Rel(usize),
}

#[derive(Clone, Debug)]
enum Opd {
    Col(usize),
    Num(i64),
    Text(u32),
    Flag(FlagState),
    Nums(Vec<WholeNumber>),
    Texts(Vec<ryo_values::domain::plan_parts::types::LiteralText>),
    Func(Function, Vec<Opd>),
}

#[derive(Clone, Debug)]
struct Cmp {
    test: Test,
    l: Opd,
    r: Opd,
    /// Equal / not equal of a column with a literal text: decided on codes when the cell is text.
    fast: Option<(usize, u32, bool)>,
    /// The memo slot when the verdict depends only on one column's text.
    memo: Option<(usize, usize)>,
}

#[derive(Clone, Debug)]
enum Val {
    Col(usize),
    Const(u64),
    Func(Function, Vec<Opd>),
}

/// A closure fused into a join: the right rows are the closure of the right input's edges, each logical right column one of
/// (partition keys..., source, destination).
#[derive(Clone, Debug)]
struct Reach {
    keys: Vec<usize>,
    from: usize,
    to: usize,
    columns: Vec<usize>,
}

#[derive(Clone, Debug)]
enum Op {
    Select { src: Src, cmps: Vec<Cmp>, kept: Vec<usize> },
    Project { src: Src, cols: Vec<usize>, dedup: bool },
    Extend { src: Src, vals: Vec<Val> },
    Join { l: Src, r: Src, lk: Vec<usize>, rk: Vec<usize>, out: Vec<(bool, usize)>, rfilter: Vec<Cmp>, reach: Option<Reach> },
    Union { l: Src, r: Src },
    Inter { l: Src, r: Src },
    Diff { l: Src, r: Src },
    Anti { l: Src, r: Src, lk: Vec<usize>, rk: Vec<usize> },
    Group { src: Src, keys: Vec<usize>, distinct: Option<usize> },
    Bounds { src: Src, col: usize },
    TextMap { src: Src, map: crate::domain::physical_plan::types::RowMap },
    Ordered { l: Src, r: Src, lk: Vec<usize>, rk: Vec<usize>, lo: usize, ro: usize, out: Vec<(bool, usize)> },
    Closure { src: Src, keys: Vec<usize>, from: usize, to: usize },
    /// project(cols of left) over select(left.a TEST right.b) over join(left, right on keys), as a per-group bound test.
    SemiBand { l: Src, r: Src, lk: Vec<usize>, rk: Vec<usize>, a: usize, b: usize, test: Test, cols: Vec<usize>, dedup: bool },
    Axis { starts: Src, edges: Src, domain: Src, dir: ryo_values::domain::plan_parts::types::Direction, id: usize, parent: usize, child: usize, domain_id: usize },
}

impl Op {
    fn class(&self) -> usize {
        match self {
            Op::Select { .. } => 0,
            Op::Project { .. } => 1,
            Op::Extend { .. } => 2,
            Op::Join { .. } => 3,
            Op::Union { .. } => 4,
            Op::Inter { .. } | Op::Diff { .. } => 5,
            Op::Anti { .. } => 6,
            Op::Group { .. } => 7,
            Op::Bounds { .. } => 8,
            Op::TextMap { .. } => 10,
            Op::Ordered { .. } => 11,
            Op::Closure { .. } => 12,
            Op::Axis { .. } => 13,
            Op::SemiBand { .. } => 3,
        }
    }
    fn sources(&self) -> Vec<Src> {
        match self {
            Op::Select { src, .. }
            | Op::Project { src, .. }
            | Op::Extend { src, .. }
            | Op::Group { src, .. }
            | Op::Bounds { src, .. }
            | Op::TextMap { src, .. }
            | Op::Closure { src, .. } => vec![*src],
            Op::Axis { starts, edges, domain, .. } => vec![*starts, *edges, *domain],
            Op::Ordered { l, r, .. } | Op::SemiBand { l, r, .. } => vec![*l, *r],
            Op::Join { l, r, .. }
            | Op::Union { l, r }
            | Op::Inter { l, r }
            | Op::Diff { l, r }
            | Op::Anti { l, r, .. } => vec![*l, *r],
        }
    }
}

const NCLASS: usize = 14;
const CLASSES: [&str; NCLASS] = [
    "select", "project", "extend", "join", "union", "set", "antijoin", "group", "span_bounds", "fill", "text_map",
    "ordered_match", "closure", "axis",
];

struct Rule {
    id: Box<str>,
    ordinal: u32,
    node: u32,
    span: usize,
}

pub struct Program {
    ops: Vec<Option<Op>>,
    widths: Vec<usize>,
    needed: Vec<u32>,
    readers: Vec<u32>,
    rules: Vec<Rule>,
    rels: Vec<Relation>,
    rel_widths: Vec<usize>,
    literals: Vec<Box<str>>,
    memos: usize,
    /// For each node, the rules that need it (covered rule indexes), for pruning files a rule cannot emit in.
    owners: Vec<Vec<u32>>,
    /// Per covered rule: its necessary guard sources (relation slots or nodes read directly under joins / selects on its spine).
    guards: Vec<Vec<u32>>,
    needs_source: bool,
    rel_kinds: Vec<Vec<CellKind>>,
    /// Per node, the rules whose findings in a file need a row of it in that file.
    necessary_for: Vec<Vec<u32>>,
    /// Closure nodes computed at their joins, from the keys the other side holds.
    virtual_closure: Vec<bool>,
    /// Every relation any node of the program reads (the fill floor's input).
    all_rels: Vec<Relation>,
}

static PROGRAM: OnceLock<Program> = OnceLock::new();

pub(crate) fn program() -> Option<&'static Program> {
    PROGRAM.get()
}

struct Lowerer {
    rels: Vec<Relation>,
    literals: Vec<Box<str>>,
    lit_index: FxHashMap<Box<str>, u32>,
    memos: usize,
    widths: Vec<Option<usize>>,
}

impl Lowerer {
    fn lit(&mut self, text: &str) -> u32 {
        if let Some(code) = self.lit_index.get(text) {
            return *code;
        }
        let code = self.literals.len() as u32;
        self.literals.push(text.into());
        self.lit_index.insert(text.into(), code);
        code
    }
    fn src(&mut self, input: Option<&PlanInput>) -> Result<Src, String> {
        match input {
            Some(PlanInput::Node(node)) => Ok(Src::Node(node.0)),
            Some(PlanInput::Relation(relation)) => {
                let at = match self.rels.iter().position(|r| r == relation) {
                    Some(at) => at,
                    None => {
                        self.rels.push(*relation);
                        self.rels.len() - 1
                    }
                };
                Ok(Src::Rel(at))
            }
            Some(other) => Err(format!("input {other:?}")),
            None => Err("missing input".into()),
        }
    }
    fn width(&self, src: Src) -> Result<usize, String> {
        match src {
            Src::Node(node) => self
                .widths
                .get(node as usize)
                .copied()
                .flatten()
                .ok_or_else(|| format!("unlowered input node {node}")),
            Src::Rel(slot) => Ok(rel_width(self.rels[slot])),
        }
    }
    fn opd(&mut self, operand: &Operand, bindings: &[ParameterBinding]) -> Result<Opd, String> {
        Ok(match operand {
            Operand::Column(column) => Opd::Col(column.0 as usize + 1),
            Operand::Literal(QueryLiteral::Number(number)) => Opd::Num(number.0),
            Operand::Literal(QueryLiteral::Text(text)) => Opd::Text(self.lit(&text.0)),
            Operand::Parameter(id) => match bindings.get(id.0 as usize) {
                Some(ParameterBinding::Number(number)) => Opd::Num(number.0),
                Some(ParameterBinding::Flag(flag)) => Opd::Flag(*flag),
                Some(ParameterBinding::Text(text)) => Opd::Text(self.lit(&text.0)),
                Some(ParameterBinding::Numbers(numbers)) => Opd::Nums(numbers.clone()),
                Some(ParameterBinding::Texts(texts)) => Opd::Texts(texts.clone()),
                other => return Err(format!("parameter binding {other:?}")),
            },
            Operand::Function(applied) => Opd::Func(
                applied.function,
                applied
                    .arguments
                    .iter()
                    .map(|argument| self.opd(argument, bindings))
                    .collect::<Result<_, _>>()?,
            ),
        })
    }
    fn cmp(&mut self, comparison: &PlanComparison, bindings: &[ParameterBinding]) -> Result<Cmp, String> {
        let l = self.opd(&comparison.left, bindings)?;
        let r = self.opd(&comparison.right, bindings)?;
        let fast = match (comparison.test, &l, &r) {
            (Test::Equal | Test::NotEqual, Opd::Col(c), Opd::Text(code))
            | (Test::Equal | Test::NotEqual, Opd::Text(code), Opd::Col(c)) => {
                Some((*c, *code, comparison.test == Test::Equal))
            }
            _ => None,
        };
        let fixed = |o: &Opd| matches!(o, Opd::Num(_) | Opd::Text(_) | Opd::Flag(_) | Opd::Nums(_) | Opd::Texts(_));
        let memo = match (&l, &r) {
            (Opd::Col(c), other) | (other, Opd::Col(c)) if fixed(other) => {
                self.memos += 1;
                Some((self.memos - 1, *c))
            }
            _ => None,
        };
        Ok(Cmp { test: comparison.test, l, r, fast, memo })
    }
}

fn remap(op: &mut Op, f: &mut dyn FnMut(usize) -> usize) {
    let mut fix = |src: &mut Src| {
        if let Src::Rel(slot) = src {
            *slot = f(*slot);
        }
    };
    match op {
        Op::Select { src, .. } | Op::Project { src, .. } | Op::Extend { src, .. } | Op::Group { src, .. } | Op::Bounds { src, .. }
        | Op::TextMap { src, .. } | Op::Closure { src, .. } => fix(src),
        Op::Axis { starts, edges, domain, .. } => {
            fix(starts);
            fix(edges);
            fix(domain);
        }
        Op::Ordered { l, r, .. } | Op::SemiBand { l, r, .. } => {
            fix(l);
            fix(r);
        }
        Op::Join { l, r, .. } | Op::Union { l, r } | Op::Inter { l, r } | Op::Diff { l, r } | Op::Anti { l, r, .. } => {
            fix(l);
            fix(r);
        }
    }
}

fn rel_kinds(relation: Relation) -> Vec<CellKind> {
    match crate::domain::tree_cells::service::native_form(relation) {
        Some(crate::domain::tree_cells::types::TreeForm::Elements) => {
            vec![CellKind::Node, CellKind::Symbol, CellKind::Symbol, CellKind::File, CellKind::Span]
        }
        Some(crate::domain::tree_cells::types::TreeForm::Edges) => {
            vec![CellKind::Node, CellKind::Node, CellKind::Number, CellKind::Symbol, CellKind::File]
        }
        None => ryo_relations::domain::relations::service::kinds(relation).to_vec(),
    }
}

fn rel_width(relation: Relation) -> usize {
    match crate::domain::tree_cells::service::native_form(relation) {
        Some(_) => 6,
        None => 1 + ryo_relations::domain::relations::service::kinds(relation).len(),
    }
}

fn lower_node(
    low: &mut Lowerer,
    operation: &Operation,
    inputs: &[PlanInput],
    bindings: &[ParameterBinding],
) -> Result<(Op, usize), String> {
    let first = || inputs.first();
    Ok(match operation {
        Operation::Select { comparisons, kept, .. } => {
            let src = low.src(first())?;
            let w = low.width(src)?;
            let cmps = comparisons.iter().map(|c| low.cmp(c, bindings)).collect::<Result<Vec<_>, _>>()?;
            let kept: Vec<usize> = kept.iter().map(|c| c.0 as usize + 1).collect();
            let out = if kept.is_empty() { w } else { kept.len() + 1 };
            (Op::Select { src, cmps, kept }, out)
        }
        Operation::Project { columns, distinct, .. } => {
            let src = low.src(first())?;
            let cols: Vec<usize> = columns.iter().map(|c| c.0 as usize + 1).collect();
            let n = cols.len() + 1;
            (Op::Project { src, cols, dedup: *distinct == RepeatedRows::Dropped }, n)
        }
        Operation::Extend { values, .. } => {
            let src = low.src(first())?;
            let w = low.width(src)?;
            let mut vals = Vec::new();
            for value in values {
                vals.push(match low.opd(value, bindings)? {
                    Opd::Col(c) => Val::Col(c),
                    Opd::Num(n) => Val::Const(num_cell(n)),
                    Opd::Text(code) => Val::Const(mk(TAG_TEXT, u64::from(code))),
                    Opd::Flag(flag) => Val::Const(mk(TAG_FLAG, u64::from(flag == FlagState::Raised))),
                    Opd::Func(function, arguments) => Val::Func(function, arguments),
                    other => return Err(format!("extend value {other:?}")),
                });
            }
            let n = w + vals.len();
            (Op::Extend { src, vals }, n)
        }
        Operation::Join { left_on, right_on, output, filter, view, reach, .. } => {
            let l = low.src(inputs.first())?;
            let r = low.src(inputs.get(1))?;
            let placed = |c: usize| view.get(c).map_or(c, |p| p.0 as usize);
            let lk = left_on.iter().map(|c| c.0 as usize + 1).collect();
            let rk = right_on.iter().map(|c| placed(c.0 as usize) + 1).collect();
            let out: Vec<(bool, usize)> = output
                .iter()
                .map(|column| match column.side {
                    Side::Left => (false, column.column.0 as usize + 1),
                    Side::Right => (true, placed(column.column.0 as usize) + 1),
                })
                .collect();
            let rfilter = filter.iter().map(|c| low.cmp(c, bindings)).collect::<Result<Vec<_>, _>>()?;
            let reach = reach.as_ref().map(|reach| Reach {
                keys: reach.keys.iter().map(|c| c.0 as usize + 1).collect(),
                from: reach.source.0 as usize + 1,
                to: reach.destination.0 as usize + 1,
                columns: reach.columns.iter().map(|c| c.0 as usize).collect(),
            });
            if reach.is_some() && (!view.is_empty() || !filter.is_empty()) {
                return Err("operation Join (reach with view or filter)".into());
            }
            let n = out.len() + 1;
            (Op::Join { l, r, lk, rk, out, rfilter, reach }, n)
        }
        Operation::Union { .. } => {
            let l = low.src(inputs.first())?;
            let r = low.src(inputs.get(1))?;
            let w = low.width(l)?;
            if low.width(r)? != w {
                return Err("union widths".into());
            }
            (Op::Union { l, r }, w)
        }
        Operation::Intersection { .. } => {
            let l = low.src(inputs.first())?;
            let r = low.src(inputs.get(1))?;
            (Op::Inter { l, r }, low.width(l)?)
        }
        Operation::Difference { .. } => {
            let l = low.src(inputs.first())?;
            let r = low.src(inputs.get(1))?;
            (Op::Diff { l, r }, low.width(l)?)
        }
        Operation::Antijoin { left_on, right_on, .. } => {
            let l = low.src(inputs.first())?;
            let r = low.src(inputs.get(1))?;
            let lk = left_on.iter().map(|c| c.0 as usize + 1).collect();
            let rk = right_on.iter().map(|c| c.0 as usize + 1).collect();
            (Op::Anti { l, r, lk, rk }, low.width(l)?)
        }
        Operation::GroupCount { keys, distinct, .. } => {
            let src = low.src(first())?;
            let keys: Vec<usize> = keys.iter().map(|c| c.0 as usize + 1).collect();
            let n = keys.len() + 2;
            (Op::Group { src, keys, distinct: distinct.map(|c| c.0 as usize + 1) }, n)
        }
        Operation::SpanBounds { span, .. } => {
            let src = low.src(first())?;
            let w = low.width(src)?;
            (Op::Bounds { src, col: span.0 as usize + 1 }, w + 2)
        }
        Operation::Capture { .. }
        | Operation::Replace { .. }
        | Operation::SourceMatches { .. }
        | Operation::SpanBetween { .. }
        | Operation::Words { .. } => {
            let src = low.src(first())?;
            let w = low.width(src)?;
            match crate::domain::physical_plan::service::lowering::service::lowered(operation, inputs, bindings) {
                crate::domain::physical_plan::types::lowering::value::Lowering::TextMap { map, .. } => {
                    let kinds = vec![CellKind::Node; w - 1];
                    let n = crate::domain::physical_plan::collections::kernels::text_map::item::mapped_kinds(&map, &kinds).len() + 1;
                    (Op::TextMap { src, map }, n)
                }
                _ => return Err("text map lowering".into()),
            }
        }
        Operation::OrderedMatch { left_on, right_on, left_order, right_order, output, .. } => {
            let l = low.src(inputs.first())?;
            let r = low.src(inputs.get(1))?;
            let out: Vec<(bool, usize)> = output
                .iter()
                .map(|c| (c.side == Side::Right, c.column.0 as usize + 1))
                .collect();
            let n = out.len() + 1;
            (
                Op::Ordered {
                    l,
                    r,
                    lk: left_on.iter().map(|c| c.0 as usize + 1).collect(),
                    rk: right_on.iter().map(|c| c.0 as usize + 1).collect(),
                    lo: left_order.0 as usize + 1,
                    ro: right_order.0 as usize + 1,
                    out,
                },
                n,
            )
        }
        Operation::Closure { keys, source, destination, .. } => {
            let src = low.src(first())?;
            let keys: Vec<usize> = keys.iter().map(|c| c.0 as usize + 1).collect();
            let n = keys.len() + 3;
            (Op::Closure { src, keys, from: source.0 as usize + 1, to: destination.0 as usize + 1 }, n)
        }
        Operation::Axis { direction, id, parent, child, domain_id, domain_filter, domain_view, output, .. } => {
            use ryo_values::domain::plan_parts::types::Direction;
            if !domain_filter.is_empty() || !domain_view.is_empty() || !output.is_empty() {
                return Err("operation Axis with domain filter/view/output".into());
            }
            if !matches!(direction, Direction::Parent | Direction::Child | Direction::Ancestor | Direction::NearestAncestor | Direction::Descendant) {
                return Err(format!("operation Axis {direction:?}"));
            }
            let starts = low.src(inputs.first())?;
            let edges = low.src(inputs.get(1))?;
            let domain = low.src(inputs.get(2))?;
            let w = low.width(starts)?;
            (
                Op::Axis {
                    starts,
                    edges,
                    domain,
                    dir: *direction,
                    id: id.0 as usize + 1,
                    parent: parent.0 as usize + 1,
                    child: child.0 as usize + 1,
                    domain_id: domain_id.0 as usize + 1,
                },
                w + 1,
            )
        }
        other => {
            let text = format!("{other:?}");
            let name: String = text.chars().take_while(|c| c.is_alphanumeric()).collect();
            return Err(format!("operation {name}"));
        }
    })
}

/// Lowers the rules of `program` this engine can run. `rule_ids` names each rule ordinal; `spans` gives the column of each rule's
/// emitted rows named `span`.
pub fn install(program: &LogicalProgram<'_>, rule_ids: &[Box<str>], spans: &[Option<usize>]) {
    if PROGRAM.get().is_some() {
        return;
    }
    let nodes = program.nodes();
    let mut widths: Vec<Option<usize>> = vec![None; nodes.len()];
    let mut ops: Vec<Option<Op>> = vec![None; nodes.len()];
    let mut errors: Vec<Option<String>> = vec![None; nodes.len()];
    let mut low = Lowerer {
        rels: Vec::new(),
        literals: Vec::new(),
        lit_index: FxHashMap::default(),
        memos: 0,
        widths: vec![None; nodes.len()],
    };
    // Literals first so they are numbered the same in every worker.
    for (at, node) in nodes.iter().enumerate() {
        let result = if node.scope != StepScope::PerFile {
            Err("cross-file scope".to_string())
        } else {
            lower_node(&mut low, node.operation, node.inputs.0, node.bindings.0)
        };
        match result {
            Ok((op, width)) => {
                widths[at] = Some(width);
                low.widths[at] = Some(width);
                ops[at] = Some(op);
            }
            Err(error) => errors[at] = Some(error),
        }
    }
    // Semi band fusion: project(left columns) <- select(one order test, left column vs right column) <- join, where the join and
    // the select each have exactly one reader in the whole program.
    {
        let mut program_readers = vec![0u32; nodes.len()];
        for node in nodes {
            for input in node.inputs.0 {
                if let PlanInput::Node(read) = input {
                    program_readers[read.0 as usize] += 1;
                }
            }
        }
        for (at, node) in program.emitted_rules() {
            let _ = at;
            program_readers[node.0 as usize] += 1;
        }
        for at in 0..ops.len() {
            let Some(Op::Project { src: Src::Node(sel), cols, dedup }) = &ops[at] else { continue };
            let sel = *sel as usize;
            let Some(Op::Select { src: Src::Node(join), cmps, kept }) = &ops[sel] else { continue };
            let join = *join as usize;
            if !kept.is_empty() || cmps.len() != 1 || program_readers[sel] != 1 || program_readers[join] != 1 {
                continue;
            }
            let Some(Op::Join { l, r, lk, rk, out, rfilter, reach: None }) = &ops[join] else { continue };
            if !rfilter.is_empty() {
                continue;
            }
            let cmp = &cmps[0];
            let (Opd::Col(x), Opd::Col(y)) = (&cmp.l, &cmp.r) else { continue };
            let side = |c: usize| out.get(c.wrapping_sub(1)).copied();
            let flipped = |test: Test| match test {
                Test::LessThan => Some(Test::GreaterThan),
                Test::LessThanOrEqual => Some(Test::GreaterThanOrEqual),
                Test::GreaterThan => Some(Test::LessThan),
                Test::GreaterThanOrEqual => Some(Test::LessThanOrEqual),
                _ => None,
            };
            let (a, b, test) = match (side(*x), side(*y)) {
                (Some((false, a)), Some((true, b))) if flipped(cmp.test).is_some() => (a, b, cmp.test),
                (Some((true, b)), Some((false, a))) => match flipped(cmp.test) {
                    Some(test) => (a, b, test),
                    None => continue,
                },
                _ => continue,
            };
            let mut left_cols = Vec::new();
            for c in cols {
                match side(*c) {
                    Some((false, lc)) => left_cols.push(lc),
                    _ => break,
                }
            }
            if left_cols.len() != cols.len() {
                continue;
            }
            ops[at] = Some(Op::SemiBand { l: *l, r: *r, lk: lk.clone(), rk: rk.clone(), a, b, test, cols: left_cols, dedup: *dedup });
        }
    }
    // A rule is covered when every node its emitted node depends on lowered.
    let mut rules = Vec::new();
    let mut needed = vec![false; nodes.len()];
    let mut readers = vec![0u32; nodes.len()];
    let mut owners: Vec<Vec<u32>> = vec![Vec::new(); nodes.len()];
    let mut report = String::new();
    for (ordinal, node) in program.emitted_rules() {
        let id: Box<str> = rule_ids.get(ordinal as usize).cloned().unwrap_or_else(|| "?".into());
        let Some(span) = spans.get(ordinal as usize).copied().flatten() else {
            report.push_str(&format!("REFUSED\t{id}\tno span column\n"));
            continue;
        };
        let mut stack = vec![node.0];
        let mut seen = std::collections::BTreeSet::new();
        let mut refusal = None;
        while let Some(at) = stack.pop() {
            if !seen.insert(at) {
                continue;
            }
            if errors[at as usize].is_some() {
                let mut root = at as usize;
                while let Some(next) = errors[root]
                    .as_deref()
                    .and_then(|e| e.strip_prefix("unlowered input node "))
                    .and_then(|n| n.parse::<usize>().ok())
                {
                    if next == root {
                        break;
                    }
                    root = next;
                }
                refusal = Some(format!("{} (node {root})", errors[root].clone().unwrap_or_default()));
                break;
            }
            if let Some(op) = &ops[at as usize] {
                for source in op.sources() {
                    if let Src::Node(read) = source {
                        stack.push(read);
                    }
                }
            }
        }
        if let Some(refusal) = refusal {
            report.push_str(&format!("REFUSED\t{id}\t{refusal}\n"));
            continue;
        }
        let index = rules.len() as u32;
        for at in &seen {
            needed[*at as usize] = true;
            owners[*at as usize].push(index);
        }
        report.push_str(&format!("COVERED\t{id}\t{} nodes\n", seen.len()));
        rules.push(Rule { id, ordinal, node: node.0, span: span + 1 });
    }
    for (at, op) in ops.iter().enumerate() {
        if !needed[at] {
            continue;
        }
        if let Some(op) = op {
            for source in op.sources() {
                if let Src::Node(read) = source {
                    readers[read as usize] += 1;
                }
            }
        }
    }
    for rule in &rules {
        readers[rule.node as usize] += 1;
    }
    // Only the relations the needed nodes read are filled.
    let mut map: Vec<Option<usize>> = vec![None; low.rels.len()];
    let mut used: Vec<Relation> = Vec::new();
    for (at, op) in ops.iter_mut().enumerate() {
        if !needed[at] {
            *op = None;
            continue;
        }
        if let Some(op) = op {
            remap(op, &mut |slot| {
                *map[slot].get_or_insert_with(|| {
                    used.push(low.rels[slot]);
                    used.len() - 1
                })
            });
        }
    }
    low.rels = used;
    // Guards: for each rule, the relation slots and nodes on its emitted node's spine whose emptiness in a file empties the rule
    // there: the first input of select/project/extend/bounds/group, both inputs of an inner join and intersection, the left of an
    // antijoin or difference. A union empties only when both sides do, so it stops the walk.
    let mut guards = Vec::new();
    let mut necessary_for: Vec<Vec<u32>> = vec![Vec::new(); nodes.len()];
    for (rule_index, rule) in rules.iter().enumerate() {
        let mut found: Vec<u32> = Vec::new();
        let mut stack = vec![rule.node];
        let mut visited = std::collections::BTreeSet::new();
        while let Some(at) = stack.pop() {
            if !visited.insert(at) {
                continue;
            }
            necessary_for[at as usize].push(rule_index as u32);
            let Some(op) = &ops[at as usize] else { continue };
            let necessary: Vec<Src> = match op {
                Op::Select { src, .. } | Op::Project { src, .. } | Op::Extend { src, .. } | Op::Group { src, .. } | Op::Bounds { src, .. }
                | Op::TextMap { src, .. } | Op::Closure { src, .. } => vec![*src],
                Op::Axis { starts, edges, domain, .. } => vec![*starts, *edges, *domain],
                Op::Join { l, r, .. } | Op::Inter { l, r } | Op::Ordered { l, r, .. } | Op::SemiBand { l, r, .. } => vec![*l, *r],
                Op::Anti { l, .. } | Op::Diff { l, .. } => vec![*l],
                Op::Union { .. } => vec![],
            };
            for source in necessary {
                match source {
                    Src::Rel(slot) => {
                        let mark = slot as u32 | (1 << 31);
                        if !found.contains(&mark) {
                            found.push(mark);
                        }
                    }
                    Src::Node(read) => stack.push(read),
                }
            }
        }
        guards.push(found);
    }
    let rel_widths = low.rels.iter().map(|r| rel_width(*r)).collect();
    let rel_kinds = low.rels.iter().map(|r| rel_kinds(*r)).collect();
    // A closure every reader of which joins on its source column is computed at each reader, from that reader's keys.
    let mut virtual_closure = vec![false; nodes.len()];
    for (at, op) in ops.iter().enumerate() {
        let Some(Op::Closure { keys, .. }) = op else { continue };
        let source_at = keys.len() + 1;
        let me = Src::Node(at as u32);
        let mut all = true;
        let mut any = false;
        for (reader, other) in ops.iter().enumerate() {
            let Some(other) = other else { continue };
            if !needed[reader] || !other.sources().contains(&me) {
                continue;
            }
            any = true;
            let fits = match other {
                Op::Join { l, r, lk, rk, reach: None, .. } if l != r => {
                    let end_at = source_at + 1;
                    (*l == me && (lk.contains(&source_at) || lk.contains(&end_at)))
                        || (*r == me && (rk.contains(&source_at) || rk.contains(&end_at)))
                }
                _ => false,
            };
            all &= fits;
        }
        virtual_closure[at] = any && all && !rules.iter().any(|rule| rule.node == at as u32);
    }
    let needed_list: Vec<u32> = (0..nodes.len() as u32).filter(|at| needed[*at as usize]).collect();
    if let Some(path) = &switches().cover {
        report.push_str(&format!("NODES\t{}\nRELATIONS\t{:?}\n", needed_list.len(), low.rels));
        drop(std::fs::write(path, report));
    }
    let needs_source = needed_list.iter().any(|at| matches!(ops[*at as usize], Some(Op::TextMap { .. })));
    let program = Program {
        necessary_for,
        ops,
        widths: widths.iter().map(|w| w.unwrap_or(0)).collect(),
        needed: needed_list,
        readers,
        rules,
        rel_widths,
        rels: low.rels,
        literals: low.literals,
        memos: low.memos,
        all_rels: {
            let mut all = Vec::new();
            for node in nodes {
                for input in node.inputs.0 {
                    if let PlanInput::Relation(relation) = input
                        && !all.contains(relation)
                    {
                        all.push(*relation);
                    }
                }
            }
            all
        },
        needs_source,
        rel_kinds,
        virtual_closure,
        owners,
        guards,
    };
    drop(PROGRAM.set(program));
}

/// The relations the covered rules read.
pub fn relations() -> Vec<Relation> {
    program().map(|p| p.fill_rels().to_vec()).unwrap_or_default()
}

impl Program {
    fn fill_rels(&self) -> &[Relation] {
        if switches().fill_all { &self.all_rels } else { &self.rels }
    }
}

// ------------------------------------------------------------------------------------------------------------------------
// Workers: one per thread, each with its own text codes (literals first, so they agree), its batch and its findings.

#[derive(Default)]
struct Strings {
    map: FxHashMap<Box<str>, u32>,
    texts: Vec<Box<str>>,
    /// The bridge to the engine's own kernels: a symbol table holding the texts a bridged row names.
    syms: Symbols,
    code_sym: Vec<u32>,
    sym_code: FxHashMap<u32, u32>,
}

impl Strings {
    fn intern(&mut self, text: &str) -> u32 {
        if let Some(code) = self.map.get(text) {
            return *code;
        }
        let code = self.texts.len() as u32;
        self.texts.push(text.into());
        self.map.insert(text.into(), code);
        code
    }
    fn sym(&mut self, code: u32) -> ryo_relations::domain::terms::types::Symbol {
        let at = code as usize;
        if self.code_sym.len() <= at {
            self.code_sym.resize(at + 1, u32::MAX);
        }
        if self.code_sym[at] == u32::MAX {
            let symbol = ryo_language::domain::symbols::service::intern_derived(
                &mut self.syms,
                ByteSlice(&[]),
                ryo_relations::domain::terms::types::NameText(&self.texts[at]),
            );
            self.code_sym[at] = symbol.0;
            self.sym_code.insert(symbol.0, code);
        }
        ryo_relations::domain::terms::types::Symbol(self.code_sym[at])
    }
    fn from_sym(&mut self, symbol: ryo_relations::domain::terms::types::Symbol, source: &[u8]) -> u32 {
        if let Some(code) = self.sym_code.get(&symbol.0) {
            return *code;
        }
        let text: String = text_of(&self.syms, ByteSlice(source), symbol).map(|t| t.0.to_owned()).unwrap_or_default();
        let code = self.intern(&text);
        self.sym_code.insert(symbol.0, code);
        code
    }
}

struct FileMeta {
    id: u32,
    path: Arc<str>,
    runs: Vec<u64>,
    source: Vec<u8>,
}

#[derive(Default, Clone)]
struct Stats {
    nanos: [u64; NCLASS],
    tuples: [u64; NCLASS],
    index_nanos: u64,
    index_tuples: u64,
    sort_nanos: u64,
    files: u64,
    batches: u64,
    pruned_rule_files: u64,
    run_rule_files: u64,
    dyn_pruned: u64,
    peak_batch_cells: u64,
    node_nanos: Vec<u64>,
    node_tuples: Vec<u64>,
}

struct Worker {
    strings: Strings,
    files: Vec<FileMeta>,
    rels: Vec<Vec<u64>>,
    memos: Vec<Vec<u8>>,
    regexes: Patterns,
    findings: Vec<(u32, Arc<str>, u32, u32)>,
    stats: Stats,
    symbol_codes: FxHashMap<u32, u32>,
    cache: FxHashMap<(usize, Vec<usize>), Arc<Vec<u64>>>,
    pair_cache: FxHashMap<(usize, usize, usize), Arc<Vec<u64>>>,
}

impl Worker {
    fn new(program: &Program) -> Self {
        let mut strings = Strings::default();
        for literal in &program.literals {
            strings.intern(literal);
        }
        Worker {
            strings,
            files: Vec::new(),
            rels: vec![Vec::new(); program.fill_rels().len()],
            memos: vec![Vec::new(); program.memos],
            regexes: Patterns::default(),
            findings: Vec::new(),
            stats: Stats::default(),
            symbol_codes: FxHashMap::default(),
            cache: FxHashMap::default(),
            pair_cache: FxHashMap::default(),
        }
    }
}

static WORKERS: Mutex<Vec<Arc<Mutex<Worker>>>> = Mutex::new(Vec::new());

thread_local! {
    static MINE: RefCell<Option<Arc<Mutex<Worker>>>> = const { RefCell::new(None) };
}

fn with_worker<T>(program: &Program, f: impl FnOnce(&mut Worker) -> T) -> T {
    MINE.with(|mine| {
        let mut mine = mine.borrow_mut();
        let arc = mine.get_or_insert_with(|| {
            let made = Arc::new(Mutex::new(Worker::new(program)));
            WORKERS.lock().unwrap_or_else(|e| e.into_inner()).push(Arc::clone(&made));
            made
        });
        let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)
    })
}

// ------------------------------------------------------------------------------------------------------------------------
// Fill.

fn encode(w: &mut Worker, value: Value, symbols: &Symbols, source: &[u8]) -> u64 {
    match value {
        Value::Node(node) => mk(TAG_NODE, node.0 & 0xffff_ffff),
        Value::Symbol(symbol) => {
            let code = match w.symbol_codes.get(&symbol.0) {
                Some(code) => *code,
                None => {
                    let text = text_of(symbols, ByteSlice(source), symbol).map(|t| t.0).unwrap_or("");
                    let code = w.strings.intern(text);
                    w.symbol_codes.insert(symbol.0, code);
                    code
                }
            };
            mk(TAG_TEXT, u64::from(code))
        }
        Value::Name(name) => {
            let hash = name.hash;
            let hash = hash.0;
            mk(TAG_TEXT, u64::from(w.strings.intern(&format!("\u{0}name{hash}"))))
        }
        Value::File(file) => mk(TAG_FILE, u64::from(file.0)),
        Value::Span(span) => span_cell(span.start.0, span.end.0),
        Value::Number(number) => num_cell(number.0),
        Value::Flag(flag) => mk(TAG_FLAG, u64::from(flag == ryo_relations::domain::relations::types::Flag::Raised)),
    }
}

/// Copies one file's relations into this thread's batch, and evaluates the batch when it is full.
pub fn fill_file<'f>(
    file: FileId,
    path: &str,
    runs: &dyn Fn(u32) -> bool,
    facts: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    elements: &Elements,
    symbols: &Symbols,
    source: &[u8],
) {
    let Some(program) = program() else { return };
    with_worker(program, |w| {
        let started = Instant::now();
        let ordinal = w.files.len() as u64;
        let mut runs_bits = vec![0u64; program.rules.len().div_ceil(64)];
        for (index, rule) in program.rules.iter().enumerate() {
            if runs(rule.ordinal) {
                runs_bits[index / 64] |= 1 << (index % 64);
            }
        }
        w.files.push(FileMeta { id: file.0, path: path.into(), runs: runs_bits, source: if program.needs_source { source.to_vec() } else { Vec::new() } });
        w.symbol_codes.clear();
        let mut filled = 0u64;
        for (slot, relation) in program.fill_rels().iter().enumerate() {
            let mut rows = std::mem::take(&mut w.rels[slot]);
            match crate::domain::tree_cells::service::native_form(*relation) {
                Some(form) => {
                    let tree = crate::domain::tree_cells::types::TreeRows { form, elements, file };
                    let edges = form == crate::domain::tree_cells::types::TreeForm::Edges;
                    for at in positions(elements) {
                        if edges && at.0 == 0 {
                            continue;
                        }
                        let cells = if edges {
                            crate::domain::tree_cells::service::edge_of(&tree, at)
                        } else {
                            crate::domain::tree_cells::service::element_of(&tree, at)
                        };
                        rows.push(ordinal);
                        for cell in cells {
                            let encoded = encode(w, cell, symbols, source);
                            rows.push(encoded);
                        }
                        filled += 1;
                    }
                }
                None => {
                    if let Some(view) = facts(*relation) {
                        let count = view.columns.first().map_or(0, |c| c.0.len());
                        rows.reserve(count * (view.kinds.len() + 1));
                        for row in 0..count {
                            rows.push(ordinal);
                            for (kind, column) in view.kinds.iter().zip(view.columns) {
                                let bits = column.0[row];
                                let cell = match kind {
                                    CellKind::Node => mk(TAG_NODE, bits & 0xffff_ffff),
                                    CellKind::Symbol => {
                                        let value = ryo_relations::domain::relations::service::from_bits(
                                            *kind,
                                            ryo_relations::domain::relations::types::Bits(bits),
                                        );
                                        encode(w, value, symbols, source)
                                    }
                                    CellKind::File => mk(TAG_FILE, bits & 0xffff_ffff),
                                    CellKind::Span => span_cell((bits >> 32) as u32, (bits & 0xffff_ffff) as u32),
                                    CellKind::Number => num_cell(bits as i64),
                                    CellKind::Flag => mk(TAG_FLAG, u64::from(bits != 0)),
                                };
                                rows.push(cell);
                            }
                            filled += 1;
                        }
                    }
                }
            }
            w.rels[slot] = rows;
        }
        w.stats.nanos[9] += started.elapsed().as_nanos() as u64;
        w.stats.tuples[9] += filled;
        w.stats.files += 1;
        if w.files.len() >= switches().batch {
            evaluate(program, w);
        }
    });
}

// ------------------------------------------------------------------------------------------------------------------------
// Sorting and indexes.

macro_rules! by_width {
    ($w:expr, $n:ident, $body:block, $fallback:block) => {
        match $w {
            1 => { const $n: usize = 1; $body }
            2 => { const $n: usize = 2; $body }
            3 => { const $n: usize = 3; $body }
            4 => { const $n: usize = 4; $body }
            5 => { const $n: usize = 5; $body }
            6 => { const $n: usize = 6; $body }
            7 => { const $n: usize = 7; $body }
            8 => { const $n: usize = 8; $body }
            9 => { const $n: usize = 9; $body }
            10 => { const $n: usize = 10; $body }
            11 => { const $n: usize = 11; $body }
            12 => { const $n: usize = 12; $body }
            _ => $fallback,
        }
    };
}

fn sort_rows(data: &mut Vec<u64>, w: usize, dedup: bool) {
    if data.len() <= w {
        return;
    }
    by_width!(w, N, {
        let (chunks, _) = data.as_chunks_mut::<N>();
        chunks.sort_unstable();
        if dedup {
            let mut keep = 0usize;
            for at in 0..chunks.len() {
                if at == 0 || chunks[at] != chunks[keep - 1] {
                    chunks[keep] = chunks[at];
                    keep += 1;
                }
            }
            data.truncate(keep * N);
        }
    }, {
        let mut rows: Vec<Vec<u64>> = data.chunks_exact(w).map(<[u64]>::to_vec).collect();
        rows.sort_unstable();
        if dedup {
            rows.dedup();
        }
        data.clear();
        for row in rows {
            data.extend_from_slice(&row);
        }
    });
}

/// An index of `rows` (width `w`) over `keys` (the file column first): tuples of the keys and the row number, sorted.
fn index(rows: &[u64], w: usize, keys: &[usize], only: Option<&[u32]>, stats: &mut Stats) -> Vec<u64> {
    let mut cols = Vec::with_capacity(keys.len() + 1);
    cols.push(0);
    cols.extend_from_slice(keys);
    index_cols(rows, w, &cols, only, stats)
}

/// An index of a base relation over `cols`, built once per batch and kept for every reader.
fn cached_index(
    cache: &mut FxHashMap<(usize, Vec<usize>), Arc<Vec<u64>>>,
    slot: usize,
    rows: &[u64],
    w: usize,
    cols: Vec<usize>,
    stats: &mut Stats,
) -> Arc<Vec<u64>> {
    if let Some(found) = cache.get(&(slot, cols.clone())) {
        return Arc::clone(found);
    }
    let made = Arc::new(index_cols(rows, w, &cols, None, stats));
    cache.insert((slot, cols), Arc::clone(&made));
    made
}

/// An index over exactly `cols` (physical positions, 0 is the file) followed by the row number, sorted.
fn index_cols(rows: &[u64], w: usize, keys: &[usize], only: Option<&[u32]>, stats: &mut Stats) -> Vec<u64> {
    let started = Instant::now();
    let iw = keys.len() + 1;
    let mut out = Vec::with_capacity((rows.len() / w.max(1)) * iw);
    let mut push = |row: usize| {
        let base = row * w;
        for key in keys {
            out.push(rows[base + key]);
        }
        out.push(row as u64);
    };
    match only {
        Some(ids) => ids.iter().for_each(|id| push(*id as usize)),
        None => (0..rows.len() / w).for_each(&mut push),
    }
    sort_rows(&mut out, iw, false);
    stats.index_nanos += started.elapsed().as_nanos() as u64;
    stats.index_tuples += (out.len() / iw) as u64;
    out
}

/// The first position at or after `from` (in tuples) whose prefix is not less than `key`, by galloping.
fn gallop(data: &[u64], w: usize, k: usize, from: usize, key: &[u64]) -> usize {
    let n = data.len() / w;
    let less = |at: usize| data[at * w..at * w + k] < *key;
    if from >= n || !less(from) {
        return from;
    }
    let mut step = 1;
    let mut lo = from;
    while lo + step < n && less(lo + step) {
        lo += step;
        step <<= 1;
    }
    let mut hi = (lo + step).min(n);
    lo += 1;
    while lo < hi {
        let mid = (lo + hi) / 2;
        if less(mid) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Merges two sorted indexes on their first `k` cells; calls `pair` with the row numbers of each match.
fn merge(a: &[u64], b: &[u64], k: usize, mut pair: impl FnMut(&[u64], usize, usize)) {
    let w = k + 1;
    let (na, nb) = (a.len() / w, b.len() / w);
    let (mut i, mut j) = (0usize, 0usize);
    while i < na && j < nb {
        let ka = &a[i * w..i * w + k];
        let kb = &b[j * w..j * w + k];
        match ka.cmp(kb) {
            std::cmp::Ordering::Less => i = gallop(a, w, k, i, kb),
            std::cmp::Ordering::Greater => j = gallop(b, w, k, j, ka),
            std::cmp::Ordering::Equal => {
                let mut i2 = i + 1;
                while i2 < na && a[i2 * w..i2 * w + k] == *ka {
                    i2 += 1;
                }
                let mut j2 = j + 1;
                while j2 < nb && b[j2 * w..j2 * w + k] == *kb {
                    j2 += 1;
                }
                for x in i..i2 {
                    for y in j..j2 {
                        pair(ka, a[x * w + k] as usize, b[y * w + k] as usize);
                    }
                }
                i = i2;
                j = j2;
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------------------------------
// Comparisons.

struct Ctx<'a> {
    strings: &'a mut Strings,
    memos: &'a mut [Vec<u8>],
    regexes: &'a mut Patterns,
    files: &'a [FileMeta],
}

fn datum_of<'s>(cell: u64, ctx_strings: &'s Strings, files: &'s [FileMeta], file_ordinal: u64) -> Datum<'s> {
    match tag(cell) {
        TAG_NODE => {
            let file = files.get(file_ordinal as usize).map_or(0, |f| f.id);
            Datum::Number(WholeNumber(((u64::from(file) << 32) | (cell & 0xffff_ffff)) as i64))
        }
        TAG_TEXT => Datum::Text(TextView(&ctx_strings.texts[(cell & PAY) as usize])),
        TAG_FILE => Datum::Text(TextView(files.get(file_ordinal as usize).map_or("", |f| &f.path))),
        TAG_SPAN => match span_of(cell) {
            Some((start, _)) => Datum::Number(WholeNumber(i64::from(start))),
            None => Datum::Unlocated,
        },
        TAG_NUM => Datum::Number(WholeNumber(num_of(cell))),
        _ => Datum::Flag(if cell & 1 == 1 { FlagState::Raised } else { FlagState::Lowered }),
    }
}

fn func_text(function: Function, arguments: &[Opd], row: &[u64], strings: &Strings, files: &[FileMeta]) -> Option<String> {
    let mut made_args = Vec::with_capacity(arguments.len());
    for argument in arguments {
        let text = match argument {
            Opd::Col(c) => match datum_of(row[*c], strings, files, row[0]) {
                Datum::Text(t) => t.0.to_owned(),
                _ => return None,
            },
            Opd::Text(code) => strings.texts[*code as usize].to_string(),
            Opd::Func(f, a) => func_text(*f, a, row, strings, files)?,
            _ => return None,
        };
        made_args.push(MadeText(text));
    }
    let mut made = MadeText::default();
    crate::domain::functions::service::apply(function, &made_args, &mut made);
    Some(made.0)
}

fn resolve<'s>(opd: &'s Opd, row: &[u64], strings: &'s Strings, files: &'s [FileMeta], made: &'s mut Option<String>) -> Datum<'s> {
    match opd {
        Opd::Col(c) => datum_of(row[*c], strings, files, row[0]),
        Opd::Num(n) => Datum::Number(WholeNumber(*n)),
        Opd::Text(code) => Datum::Text(TextView(&strings.texts[*code as usize])),
        Opd::Flag(flag) => Datum::Flag(*flag),
        Opd::Nums(numbers) => Datum::Numbers(numbers),
        Opd::Texts(texts) => Datum::Texts(texts),
        Opd::Func(function, arguments) => {
            *made = func_text(*function, arguments, row, strings, files);
            match made {
                Some(text) => Datum::Text(TextView(text)),
                None => Datum::Unlocated,
            }
        }
    }
}

fn decide(test: Test, l: &Datum<'_>, r: &Datum<'_>, regexes: &mut Patterns) -> bool {
    let matching = ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
    let semantics = crate::application::matching::types::PortSemantics(&matching);
    let matcher = PatternMatcher(&semantics);
    let outcome = match family_of(test) {
        Family::Equality => EqualityItem.holds(test, l, r).ok(),
        Family::Ordering => OrderingItem.holds(test, l, r).ok(),
        Family::FloatText => FloatTextItem.holds(test, l, r).ok(),
        Family::Numeric => NumericItem.holds(test, l, r).ok(),
        Family::Membership => MembershipItem.holds(test, l, r).ok(),
        Family::Affix => AffixItem.holds(test, l, r).ok(),
        Family::Regex => crate::domain::patterns::service::matches_prepared(l, r, None, regexes, matcher).ok(),
        Family::Glob => GlobItem(matcher).holds(test, l, r).ok(),
    };
    outcome == Some(PredicateOutcome::Holds)
}

fn holds(cmp: &Cmp, row: &[u64], ctx: &mut Ctx<'_>) -> bool {
    if let Some((col, code, equal)) = cmp.fast {
        let cell = row[col];
        if tag(cell) == TAG_TEXT {
            return ((cell & PAY) == u64::from(code)) == equal;
        }
    }
    if let Some((slot, col)) = cmp.memo {
        let cell = row[col];
        if tag(cell) == TAG_TEXT {
            let code = (cell & PAY) as usize;
            let memo = &mut ctx.memos[slot];
            if let Some(v) = memo.get(code)
                && *v != 0
            {
                return *v == 1;
            }
            let verdict = general(cmp, row, ctx);
            let memo = &mut ctx.memos[slot];
            if memo.len() <= code {
                memo.resize(code + 1, 0);
            }
            memo[code] = if verdict { 1 } else { 2 };
            return verdict;
        }
    }
    general(cmp, row, ctx)
}

fn general(cmp: &Cmp, row: &[u64], ctx: &mut Ctx<'_>) -> bool {
    let (mut ml, mut mr) = (None, None);
    let l = resolve(&cmp.l, row, ctx.strings, ctx.files, &mut ml);
    let r = resolve(&cmp.r, row, ctx.strings, ctx.files, &mut mr);
    decide(cmp.test, &l, &r, ctx.regexes)
}

// ------------------------------------------------------------------------------------------------------------------------
// Evaluation of one batch.

fn evaluate(program: &Program, w: &mut Worker) {
    if w.files.is_empty() {
        return;
    }
    let fill_only = switches().fill_only;
    let nfiles = w.files.len();
    let mut cells = 0u64;
    for rel in &w.rels {
        cells += rel.len() as u64;
    }
    w.stats.peak_batch_cells = w.stats.peak_batch_cells.max(cells);
    w.stats.batches += 1;
    if fill_only {
        let started = Instant::now();
        for (slot, relation) in program.fill_rels().iter().enumerate() {
            let width = rel_width(*relation);
            let mut rows = std::mem::take(&mut w.rels[slot]);
            sort_rows(&mut rows, width, false);
            w.rels[slot] = rows;
        }
        w.stats.sort_nanos += started.elapsed().as_nanos() as u64;
    }
    if !fill_only {
        // Guards: a file where a necessary relation of the rule has no row is dropped from the rule.
        let mut alive: Vec<Vec<u64>> = w.files.iter().map(|f| f.runs.clone()).collect();
        if !switches().no_guards {
            let mut present: Vec<Vec<bool>> = vec![vec![false; nfiles]; program.rels.len()];
            for (slot, rows) in w.rels.iter().enumerate() {
                let width = program.rel_widths[slot];
                for row in rows.chunks_exact(width) {
                    present[slot][row[0] as usize] = true;
                }
            }
            for (index, guards) in program.guards.iter().enumerate() {
                for (file, bits) in alive.iter_mut().enumerate() {
                    if bits[index / 64] & (1 << (index % 64)) == 0 {
                        continue;
                    }
                    if guards.iter().any(|g| !present[(*g & !(1 << 31)) as usize][file]) {
                        bits[index / 64] &= !(1 << (index % 64));
                        w.stats.pruned_rule_files += 1;
                    } else {
                        w.stats.run_rule_files += 1;
                    }
                }
            }
        }
        // Each node's files: the union of the files alive for the rules that own it. The base relations are restricted per
        // node read by filtering on the file column only when it pays (here: a node with no live file is skipped).
        let mut results: Vec<Option<Vec<u64>>> = vec![None; program.ops.len()];
        let mut left = program.readers.clone();
        let rels = std::mem::take(&mut w.rels);
        for &node in &program.needed {
            let Some(op) = &program.ops[node as usize] else { continue };
            if program.virtual_closure[node as usize] {
                continue;
            }
            let live_files: Vec<bool> = {
                let mut live = vec![false; nfiles];
                for owner in &program.owners[node as usize] {
                    for (file, bits) in alive.iter().enumerate() {
                        if bits[*owner as usize / 64] & (1 << (*owner % 64)) != 0 {
                            live[file] = true;
                        }
                    }
                }
                live
            };
            let started = Instant::now();
            let out = if live_files.iter().any(|l| *l) {
                run_op(program, op, node, &results, &rels, w, &live_files)
            } else {
                Vec::new()
            };
            let class = op.class();
            let spent = started.elapsed().as_nanos() as u64;
            w.stats.nanos[class] += spent;
            if w.stats.node_nanos.len() < program.ops.len() {
                w.stats.node_nanos.resize(program.ops.len(), 0);
                w.stats.node_tuples.resize(program.ops.len(), 0);
            }
            w.stats.node_nanos[node as usize] += spent;
            w.stats.node_tuples[node as usize] += (out.len() / program.widths[node as usize].max(1)) as u64;
            w.stats.tuples[class] += (out.len() / program.widths[node as usize].max(1)) as u64;
            // Semi-join reduction: a necessary node with no row in a file drops the file from every rule that needs it.
            let needs = &program.necessary_for[node as usize];
            if !needs.is_empty() {
                let width = program.widths[node as usize].max(1);
                let mut present = vec![false; nfiles];
                for row in out.chunks_exact(width) {
                    present[row[0] as usize] = true;
                }
                for rule in needs {
                    let (word, bit) = (*rule as usize / 64, 1u64 << (*rule % 64));
                    for (file, bits) in alive.iter_mut().enumerate() {
                        if !present[file] && bits[word] & bit != 0 {
                            bits[word] &= !bit;
                            w.stats.dyn_pruned += 1;
                        }
                    }
                }
            }
            results[node as usize] = Some(out);
            for source in op.sources() {
                if let Src::Node(read) = source {
                    left[read as usize] -= 1;
                    if left[read as usize] == 0 {
                        results[read as usize] = None;
                    }
                }
            }
        }
        for (index, rule) in program.rules.iter().enumerate() {
            let width = program.widths[rule.node as usize];
            if let Some(rows) = &results[rule.node as usize] {
                let mut rows = rows.clone();
                sort_rows(&mut rows, width, true);
                for row in rows.chunks_exact(width) {
                    let file = &w.files[row[0] as usize];
                    if file.runs[index / 64] & (1 << (index % 64)) == 0 {
                        continue;
                    }
                    let cell = row[rule.span];
                    if tag(cell) != TAG_SPAN {
                        continue;
                    }
                    if let Some((start, end)) = span_of(cell) {
                        w.findings.push((index as u32, Arc::clone(&file.path), start, end));
                    }
                }
            }
        }
        w.rels = rels;
    }
    for rel in &mut w.rels {
        rel.clear();
    }
    w.cache.clear();
    w.pair_cache.clear();
    w.files.clear();
}

fn source<'r>(src: Src, results: &'r [Option<Vec<u64>>], rels: &'r [Vec<u64>]) -> &'r [u64] {
    match src {
        Src::Node(node) => results[node as usize].as_deref().unwrap_or(&[]),
        Src::Rel(slot) => &rels[slot],
    }
}

fn width_of(program: &Program, src: Src) -> usize {
    match src {
        Src::Node(node) => program.widths[node as usize],
        Src::Rel(slot) => program.rel_widths[slot],
    }
}

/// The closure of `edges` (width `width`): one row (file, keys..., source, destination) for each destination reached by one or
/// more edges from each source, within each (file, keys) partition. With `seeds` (sorted (file, node) pairs), only from those.
fn closure_rows(
    edges_rows: &[u64],
    width: usize,
    keys: &[usize],
    from: usize,
    to: usize,
    live: &[bool],
    seeds: Option<&[u64]>,
) -> Vec<u64> {
    closure_dir(edges_rows, width, keys, from, to, live, seeds, false)
}

/// `closure_rows`, walked from the destinations when `backward` (the seeds are destinations); rows keep (source, destination).
fn closure_dir(
    edges_rows: &[u64],
    width: usize,
    keys: &[usize],
    from: usize,
    to: usize,
    live: &[bool],
    seeds: Option<&[u64]>,
    backward: bool,
) -> Vec<u64> {
    let (from, to) = if backward { (to, from) } else { (from, to) };
    let gw = keys.len() + 1;
    let ew = gw + 2;
    let mut edges = Vec::new();
    for row in edges_rows.chunks_exact(width) {
        if !live[row[0] as usize] {
            continue;
        }
        edges.push(row[0]);
        edges.extend(keys.iter().map(|k| row[*k]));
        edges.push(row[from]);
        edges.push(row[to]);
    }
    sort_rows(&mut edges, ew, true);
    let mut out = Vec::new();
    let n = edges.len() / ew;
    let mut at = 0;
    let mut seen = rustc_hash::FxHashSet::default();
    let mut stack = Vec::new();
    let seeded = |file: u64, node: u64| match seeds {
        None => true,
        Some(pairs) => {
            let at = gallop(pairs, 2, 2, 0, &[file, node]);
            at < pairs.len() / 2 && pairs[at * 2] == file && pairs[at * 2 + 1] == node
        }
    };
    while at < n {
        let group = &edges[at * ew..at * ew + gw];
        let mut end = at + 1;
        while end < n && edges[end * ew..end * ew + gw] == *group {
            end += 1;
        }
        let slice = &edges[at * ew..end * ew];
        let m = end - at;
        let targets = |node: u64| {
            let (mut lo, mut hi) = (0usize, m);
            while lo < hi {
                let mid = (lo + hi) / 2;
                if slice[mid * ew + gw] < node {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            let mut end = lo;
            while end < m && slice[end * ew + gw] == node {
                end += 1;
            }
            (lo, end)
        };
        let mut i = 0;
        while i < m {
            let start = slice[i * ew + gw];
            if seeded(group[0], start) {
                seen.clear();
                stack.clear();
                stack.push(start);
                while let Some(node) = stack.pop() {
                    let (lo, hi) = targets(node);
                    for e in lo..hi {
                        let next = slice[e * ew + gw + 1];
                        if seen.insert(next) {
                            out.extend_from_slice(group);
                            if backward {
                                out.push(next);
                                out.push(start);
                            } else {
                                out.push(start);
                                out.push(next);
                            }
                            stack.push(next);
                        }
                    }
                }
            }
            while i < m && slice[i * ew + gw] == start {
                i += 1;
            }
        }
        at = end;
    }
    out
}

/// The sorted distinct (file, value) pairs of column `col` of the live rows.
fn seeds_of(rows: &[u64], width: usize, col: usize, live: &[bool]) -> Vec<u64> {
    let mut pairs = Vec::new();
    for row in rows.chunks_exact(width) {
        if live[row[0] as usize] {
            pairs.push(row[0]);
            pairs.push(row[col]);
        }
    }
    sort_rows(&mut pairs, 2, true);
    pairs
}

/// The sorted (file, from, to) triples of an edge input: built once per batch for a base relation (every file), per call for a
/// node (its live files).
fn edge_pairs(
    cache: &mut FxHashMap<(usize, usize, usize), Arc<Vec<u64>>>,
    src: Src,
    rows: &[u64],
    width: usize,
    from: usize,
    to: usize,
    live: &[bool],
    stats: &mut Stats,
) -> Arc<Vec<u64>> {
    if let Src::Rel(slot) = src
        && let Some(found) = cache.get(&(slot, from, to))
    {
        return Arc::clone(found);
    }
    let started = Instant::now();
    let base = matches!(src, Src::Rel(_));
    let mut pairs = Vec::new();
    for row in rows.chunks_exact(width) {
        if base || live[row[0] as usize] {
            pairs.extend_from_slice(&[row[0], row[from], row[to]]);
        }
    }
    sort_rows(&mut pairs, 3, false);
    stats.index_nanos += started.elapsed().as_nanos() as u64;
    stats.index_tuples += (pairs.len() / 3) as u64;
    let made = Arc::new(pairs);
    if let Src::Rel(slot) = src {
        cache.insert((slot, from, to), Arc::clone(&made));
    }
    made
}

struct BridgeRes<'a> {
    strings: &'a mut Strings,
    source: &'a [u8],
    elements: &'a Elements,
}

impl crate::domain::physical_plan::types::Resources for BridgeRes<'_> {
    fn texts(&self) -> crate::domain::table_texts::types::Texts<'_> {
        crate::domain::table_texts::types::Texts {
            symbols: &self.strings.syms,
            source: ryo_values::domain::files::types::SourceView(self.source),
            paths: &[],
            arenas: &[],
        }
    }
    fn intern(&mut self, text: ryo_values::domain::row_texts::types::CellText<'_>) -> Value {
        Value::Symbol(ryo_language::domain::symbols::service::intern_derived(
            &mut self.strings.syms,
            ByteSlice(&[]),
            ryo_relations::domain::terms::types::NameText(text.0),
        ))
    }
    fn mint(&mut self, text: ryo_values::domain::row_texts::types::CellText<'_>) -> Value {
        self.intern(text)
    }
    fn source(&self) -> crate::domain::physical_plan::types::OpenFileBytes<'_> {
        crate::domain::physical_plan::types::OpenFileBytes(self.source)
    }
    fn tree(&self) -> &Elements {
        self.elements
    }
    fn render(
        &mut self,
        _requests: crate::domain::physical_plan::types::RenderRequests<'_>,
    ) -> Result<crate::domain::physical_plan::types::RenderOutcomes, crate::domain::contract::types::PortableOperatorFault> {
        Err(crate::domain::contract::types::PortableOperatorFault::ShapeMismatch)
    }
}

fn to_value(cell: u64, file: u32, strings: &mut Strings) -> Value {
    use ryo_relations::domain::relations::types::{Flag, Integer, NodeId, Offset, Span};
    match tag(cell) {
        TAG_NODE => Value::Node(NodeId((u64::from(file) << 32) | (cell & 0xffff_ffff))),
        TAG_TEXT => Value::Symbol(strings.sym((cell & PAY) as u32)),
        TAG_FILE => Value::File(FileId((cell & 0xffff_ffff) as u32)),
        TAG_SPAN => match span_of(cell) {
            Some((start, end)) => Value::Span(Span { start: Offset(start), end: Offset(end) }),
            None => Value::Span(ryo_relations::domain::relations::constants::NO_SPAN),
        },
        TAG_NUM => Value::Number(Integer(num_of(cell))),
        _ => Value::Flag(if cell & 1 == 1 { Flag::Raised } else { Flag::Lowered }),
    }
}

fn from_value(value: Value, strings: &mut Strings, source: &[u8]) -> u64 {
    match value {
        Value::Node(node) => mk(TAG_NODE, node.0 & 0xffff_ffff),
        Value::Symbol(symbol) => mk(TAG_TEXT, u64::from(strings.from_sym(symbol, source))),
        Value::Name(_) => mk(TAG_TEXT, u64::from(strings.intern(""))),
        Value::File(file) => mk(TAG_FILE, u64::from(file.0)),
        Value::Span(span) => span_cell(span.start.0, span.end.0),
        Value::Number(number) => num_cell(number.0),
        Value::Flag(flag) => mk(TAG_FLAG, u64::from(flag == ryo_relations::domain::relations::types::Flag::Raised)),
    }
}

fn run_op(
    program: &Program,
    op: &Op,
    node: u32,
    results: &[Option<Vec<u64>>],
    rels: &[Vec<u64>],
    w: &mut Worker,
    live: &[bool],
) -> Vec<u64> {
    let mut ctx = Ctx { strings: &mut w.strings, memos: &mut w.memos, regexes: &mut w.regexes, files: &w.files };
    let mut out = Vec::new();
    match op {
        Op::Select { src, cmps, kept } => {
            let width = width_of(program, *src);
            let keyed = match src {
                Src::Rel(slot) => cmps.iter().find_map(|c| match c.fast {
                    Some((col, code, true)) if program.rel_kinds[*slot].get(col - 1) == Some(&CellKind::Symbol) => Some((*slot, col, code)),
                    _ => None,
                }),
                Src::Node(_) => None,
            };
            if let Some((slot, col, code)) = keyed {
                let rows = &rels[slot];
                let idx = cached_index(&mut w.cache, slot, rows, width, vec![col, 0], &mut w.stats);
                let cell = mk(TAG_TEXT, u64::from(code));
                let n = idx.len() / 3;
                let mut at = gallop(&idx, 3, 1, 0, &[cell]);
                while at < n && idx[at * 3] == cell {
                    let id = idx[at * 3 + 2] as usize;
                    at += 1;
                    let row = &rows[id * width..id * width + width];
                    if !live[row[0] as usize] {
                        continue;
                    }
                    if cmps.iter().all(|c| holds(c, row, &mut ctx)) {
                        if kept.is_empty() {
                            out.extend_from_slice(row);
                        } else {
                            out.push(row[0]);
                            out.extend(kept.iter().map(|k| row[*k]));
                        }
                    }
                }
                return out;
            }
            for row in source(*src, results, rels).chunks_exact(width) {
                if !live[row[0] as usize] {
                    continue;
                }
                if cmps.iter().all(|c| holds(c, row, &mut ctx)) {
                    if kept.is_empty() {
                        out.extend_from_slice(row);
                    } else {
                        out.push(row[0]);
                        out.extend(kept.iter().map(|k| row[*k]));
                    }
                }
            }
        }
        Op::Project { src, cols, dedup } => {
            let width = width_of(program, *src);
            for row in source(*src, results, rels).chunks_exact(width) {
                if !live[row[0] as usize] {
                    continue;
                }
                out.push(row[0]);
                out.extend(cols.iter().map(|k| row[*k]));
            }
            if *dedup {
                let started = Instant::now();
                sort_rows(&mut out, cols.len() + 1, true);
                w.stats.sort_nanos += started.elapsed().as_nanos() as u64;
            }
        }
        Op::Extend { src, vals } => {
            let width = width_of(program, *src);
            for row in source(*src, results, rels).chunks_exact(width) {
                if !live[row[0] as usize] {
                    continue;
                }
                out.extend_from_slice(row);
                for val in vals {
                    let cell = match val {
                        Val::Col(c) => row[*c],
                        Val::Const(cell) => *cell,
                        Val::Func(function, arguments) => {
                            match func_text(*function, arguments, row, ctx.strings, ctx.files) {
                                Some(text) => mk(TAG_TEXT, u64::from(ctx.strings.intern(&text))),
                                None => mk(TAG_SPAN, SPAN_NONE),
                            }
                        }
                    };
                    out.push(cell);
                }
            }
        }
        Op::Join { l, r, lk, rk, out: cols, rfilter, reach } => {
            // A side that is a closure computed here, or the closure a reach join fuses, is made from the other side's keys.
            let side_rows = |side: Src, keys: &[usize], other: Src, other_keys: &[usize], fused: Option<&Reach>| -> Option<(Vec<u64>, usize)> {
                let (ckeys, cfrom, cto, columns, edges) = match fused {
                    Some(reach) => (reach.keys.clone(), reach.from, reach.to, Some(reach.columns.clone()), side),
                    None => match side {
                        Src::Node(n) if program.virtual_closure[n as usize] => match &program.ops[n as usize] {
                            Some(Op::Closure { src, keys, from, to }) => (keys.clone(), *from, *to, None, *src),
                            _ => return None,
                        },
                        _ => return None,
                    },
                };
                // The logical position of the closure's source among the side's columns.
                let logical_of = |at: usize| match &columns {
                    Some(columns) => columns.iter().position(|c| *c == at).map(|p| p + 1),
                    None => Some(at + 1),
                };
                let ow = width_of(program, other);
                let other_rows = source(other, results, rels);
                let seeded_at = |logical: Option<usize>| {
                    logical
                        .and_then(|sl| keys.iter().position(|k| *k == sl))
                        .map(|i| seeds_of(other_rows, ow, other_keys[i], live))
                };
                let (seeds, backward) = match seeded_at(logical_of(ckeys.len())) {
                    Some(seeds) => (Some(seeds), false),
                    None => match seeded_at(logical_of(ckeys.len() + 1)) {
                        Some(seeds) => (Some(seeds), true),
                        None => (None, false),
                    },
                };
                let ew = width_of(program, edges);
                let rows = closure_dir(source(edges, results, rels), ew, &ckeys, cfrom, cto, live, seeds.as_deref(), backward);
                match columns {
                    None => Some((rows, ckeys.len() + 3)),
                    Some(columns) => {
                        let tw = ckeys.len() + 3;
                        let mut mapped = Vec::with_capacity(rows.len() / tw * (columns.len() + 1));
                        for t in rows.chunks_exact(tw) {
                            mapped.push(t[0]);
                            mapped.extend(columns.iter().map(|c| t[1 + c]));
                        }
                        Some((mapped, columns.len() + 1))
                    }
                }
            };
            let lmade = side_rows(*l, lk, *r, rk, None);
            let rmade = side_rows(*r, rk, *l, lk, reach.as_ref());
            let (lrows, lw): (&[u64], usize) = match &lmade {
                Some((rows, width)) => (rows, *width),
                None => (source(*l, results, rels), width_of(program, *l)),
            };
            let (rrows, rw): (&[u64], usize) = match &rmade {
                Some((rows, width)) => (rows, *width),
                None => (source(*r, results, rels), width_of(program, *r)),
            };
            if lrows.is_empty() || rrows.is_empty() {
                return out;
            }
            let lids: Vec<u32> = (0..lrows.len() / lw).filter(|at| live[lrows[at * lw] as usize]).map(|at| at as u32).collect();
            if lids.is_empty() {
                return out;
            }
            let li: Arc<Vec<u64>> = match (l, &lmade) {
                (Src::Rel(slot), None) => {
                    let mut icols = vec![0];
                    icols.extend_from_slice(lk);
                    cached_index(&mut w.cache, *slot, lrows, lw, icols, &mut w.stats)
                }
                _ => Arc::new(index(lrows, lw, lk, Some(&lids), &mut w.stats)),
            };
            let base_right = match (r, &rmade) {
                (Src::Rel(slot), None) => Some(*slot),
                _ => None,
            };
            if let Some(slot) = base_right {
                // A base relation's index is built once per batch; the right filter is tested on the rows that match.
                let mut icols = vec![0];
                icols.extend_from_slice(rk);
                let ri = cached_index(&mut w.cache, slot, rrows, rw, icols, &mut w.stats);
                merge(&li, &ri, lk.len() + 1, |key, a, b| {
                    let row = &rrows[b * rw..b * rw + rw];
                    if live[key[0] as usize] && rfilter.iter().all(|c| holds(c, row, &mut ctx)) {
                        out.push(key[0]);
                        for (right, c) in cols.iter() {
                            out.push(if *right { row[*c] } else { lrows[a * lw + c] });
                        }
                    }
                });
            } else {
                let rids: Vec<u32> = (0..rrows.len() / rw)
                    .filter(|at| {
                        let row = &rrows[at * rw..at * rw + rw];
                        live[row[0] as usize] && rfilter.iter().all(|c| holds(c, row, &mut ctx))
                    })
                    .map(|at| at as u32)
                    .collect();
                if rids.is_empty() {
                    return out;
                }
                let ri = index(rrows, rw, rk, Some(&rids), &mut w.stats);
                merge(&li, &ri, lk.len() + 1, |key, a, b| {
                    if !live[key[0] as usize] {
                        return;
                    }
                    out.push(key[0]);
                    for (right, c) in cols.iter() {
                        out.push(if *right { rrows[b * rw + c] } else { lrows[a * lw + c] });
                    }
                });
            }
        }
        Op::Union { l, r } => {
            let width = width_of(program, *l);
            for src in [l, r] {
                for row in source(*src, results, rels).chunks_exact(width) {
                    if live[row[0] as usize] {
                        out.extend_from_slice(row);
                    }
                }
            }
            let started = Instant::now();
            sort_rows(&mut out, width, true);
            w.stats.sort_nanos += started.elapsed().as_nanos() as u64;
        }
        Op::Inter { l, r } | Op::Diff { l, r } => {
            let keep_common = matches!(op, Op::Inter { .. });
            let width = width_of(program, *l);
            let mut a: Vec<u64> = source(*l, results, rels).to_vec();
            let mut b: Vec<u64> = source(*r, results, rels).to_vec();
            sort_rows(&mut a, width, true);
            sort_rows(&mut b, width, true);
            let mut j = 0;
            let nb = b.len() / width;
            for row in a.chunks_exact(width) {
                if !live[row[0] as usize] {
                    continue;
                }
                while j < nb && b[j * width..j * width + width] < *row {
                    j += 1;
                }
                let found = j < nb && b[j * width..j * width + width] == *row;
                if found == keep_common {
                    out.extend_from_slice(row);
                }
            }
        }
        Op::Anti { l, r, lk, rk } => {
            let (lw, rw) = (width_of(program, *l), width_of(program, *r));
            let (lrows, rrows) = (source(*l, results, rels), source(*r, results, rels));
            let lids: Vec<u32> = (0..lrows.len() / lw).filter(|at| live[lrows[at * lw] as usize]).map(|at| at as u32).collect();
            if lids.is_empty() {
                return out;
            }
            if rrows.is_empty() {
                for id in lids {
                    let base = id as usize * lw;
                    out.extend_from_slice(&lrows[base..base + lw]);
                }
                return out;
            }
            let li = index(lrows, lw, lk, Some(&lids), &mut w.stats);
            let ri = index(rrows, rw, rk, None, &mut w.stats);
            let k = lk.len() + 1;
            let iw = k + 1;
            let nr = ri.len() / iw;
            let mut j = 0;
            for entry in li.chunks_exact(iw) {
                let key = &entry[..k];
                j = gallop(&ri, iw, k, j, key);
                let found = j < nr && ri[j * iw..j * iw + k] == *key;
                if !found {
                    let base = entry[k] as usize * lw;
                    out.extend_from_slice(&lrows[base..base + lw]);
                }
            }
        }
        Op::Group { src, keys, distinct } => {
            let width = width_of(program, *src);
            let kw = keys.len() + 1 + usize::from(distinct.is_some());
            let mut tuples = Vec::new();
            for row in source(*src, results, rels).chunks_exact(width) {
                if !live[row[0] as usize] {
                    continue;
                }
                tuples.push(row[0]);
                tuples.extend(keys.iter().map(|k| row[*k]));
                if let Some(d) = distinct {
                    tuples.push(row[*d]);
                }
            }
            sort_rows(&mut tuples, kw, distinct.is_some());
            let gk = keys.len() + 1;
            let mut at = 0;
            let n = tuples.len() / kw;
            while at < n {
                let key = &tuples[at * kw..at * kw + gk];
                let mut end = at + 1;
                while end < n && tuples[end * kw..end * kw + gk] == *key {
                    end += 1;
                }
                out.extend_from_slice(key);
                out.push(num_cell((end - at) as i64));
                at = end;
            }
        }
        Op::TextMap { src, map } => {
            use crate::domain::physical_plan::collections::kernels::text_map::item::{mapped_kinds, row_mapped, RowMapRun};
            let width = width_of(program, *src);
            let kinds = mapped_kinds(map, &vec![CellKind::Node; width - 1]);
            let ow = kinds.len();
            debug_assert_eq!(ow + 1, program.widths[node as usize]);
            let matching = ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
            let semantics = crate::application::matching::types::PortSemantics(&matching);
            let mut decisions = crate::domain::physical_plan::types::file_buffers::value::NodeDecisions::default();
            let mut scratch = crate::domain::contract::types::RowScratch::default();
            let mut places = crate::domain::contract::types::RangeScratch::default();
            let elements = Elements::default();
            let mut table = ryo_values::domain::rows::types::Table { kinds, ..Default::default() };
            let mut values = Vec::with_capacity(width);
            let files = ctx.files;
            for row in source(*src, results, rels).chunks_exact(width) {
                let f = row[0] as usize;
                if !live[f] {
                    continue;
                }
                let file = &files[f];
                values.clear();
                for cell in &row[1..] {
                    values.push(to_value(*cell, file.id, ctx.strings));
                }
                table.cells.clear();
                table.count = ryo_values::domain::rows::types::RowCount(0);
                let mut res = BridgeRes { strings: ctx.strings, source: &file.source, elements: &elements };
                let mut run = RowMapRun {
                    resources: &mut res,
                    matcher: PatternMatcher(&semantics),
                    decisions: &mut decisions,
                    scratch: &mut scratch,
                    places: &mut places,
                    output: &mut table,
                };
                if row_mapped(map, &values, &mut run).is_err() {
                    continue;
                }
                for orow in table.cells.chunks_exact(ow) {
                    out.push(row[0]);
                    for value in orow {
                        let cell = from_value(*value, ctx.strings, &file.source);
                        out.push(cell);
                    }
                }
            }
        }
        Op::SemiBand { l, r, lk, rk, a, b, test, cols, dedup } => {
            let (lw, rw) = (width_of(program, *l), width_of(program, *r));
            let (lrows, rrows) = (source(*l, results, rels), source(*r, results, rels));
            let files = ctx.files;
            let number = |cell: u64, f: u64| -> Option<i64> {
                match tag(cell) {
                    TAG_NODE => Some((((files[f as usize].id as u64) << 32) | (cell & 0xffff_ffff)) as i64),
                    TAG_NUM => Some(num_of(cell)),
                    TAG_SPAN => span_of(cell).map(|(start, _)| i64::from(start)),
                    _ => None,
                }
            };
            // Per right group (file, keys): the max and min of b.
            let rids: Vec<u32> = (0..rrows.len() / rw).filter(|at| live[rrows[at * rw] as usize]).map(|at| at as u32).collect();
            let lids: Vec<u32> = (0..lrows.len() / lw).filter(|at| live[lrows[at * lw] as usize]).map(|at| at as u32).collect();
            if rids.is_empty() || lids.is_empty() {
                return out;
            }
            let ri = index(rrows, rw, rk, Some(&rids), &mut w.stats);
            let k = rk.len() + 1;
            let iw = k + 1;
            let mut groups: Vec<u64> = Vec::new();
            let mut bounds: Vec<(i64, i64)> = Vec::new();
            let mut numeric = true;
            for entry in ri.chunks_exact(iw) {
                let row_at = entry[k] as usize;
                let Some(value) = number(rrows[row_at * rw + b], entry[0]) else {
                    numeric = false;
                    break;
                };
                let same = groups.len() >= k && groups[groups.len() - k..] == entry[..k];
                if same {
                    let last = bounds.last_mut().unwrap_or_else(|| unreachable!());
                    last.0 = last.0.max(value);
                    last.1 = last.1.min(value);
                } else {
                    groups.extend_from_slice(&entry[..k]);
                    bounds.push((value, value));
                }
            }
            if numeric {
                let ng = bounds.len();
                let mut key = Vec::with_capacity(k);
                for id in lids {
                    let row = &lrows[id as usize * lw..id as usize * lw + lw];
                    key.clear();
                    key.push(row[0]);
                    key.extend(lk.iter().map(|c| row[*c]));
                    let at = gallop(&groups, k, k, 0, &key);
                    if at >= ng || groups[at * k..at * k + k] != *key {
                        continue;
                    }
                    let Some(value) = number(row[*a], row[0]) else { numeric = false; break };
                    let (max, min) = bounds[at];
                    let pass = match test {
                        Test::LessThan => value < max,
                        Test::LessThanOrEqual => value <= max,
                        Test::GreaterThan => value > min,
                        _ => value >= min,
                    };
                    if pass {
                        out.push(row[0]);
                        out.extend(cols.iter().map(|c| row[*c]));
                    }
                }
            }
            if !numeric {
                // Text or another kind in an order test: the pairs, tested one by one through the engine's comparison.
                out.clear();
                let li = index(lrows, lw, lk, None, &mut w.stats);
                let cmp = Cmp { test: *test, l: Opd::Col(1), r: Opd::Col(2), fast: None, memo: None };
                merge(&li, &ri, k, |key, x, y| {
                    let pair = [key[0], lrows[x * lw + a], rrows[y * rw + b]];
                    if live[key[0] as usize] && holds(&cmp, &pair, &mut ctx) {
                        out.push(key[0]);
                        out.extend(cols.iter().map(|c| lrows[x * lw + c]));
                    }
                });
            }
            if *dedup {
                sort_rows(&mut out, cols.len() + 1, true);
            }
        }
        Op::Ordered { l, r, lk, rk, lo, ro, out: cols } => {
            let (lw, rw) = (width_of(program, *l), width_of(program, *r));
            let (lrows, rrows) = (source(*l, results, rels), source(*r, results, rels));
            let lids: Vec<u32> = (0..lrows.len() / lw).filter(|at| live[lrows[at * lw] as usize]).map(|at| at as u32).collect();
            let rids: Vec<u32> = (0..rrows.len() / rw).filter(|at| live[rrows[at * rw] as usize]).map(|at| at as u32).collect();
            if lids.is_empty() || rids.is_empty() {
                return out;
            }
            let mut lkeys = lk.clone();
            lkeys.push(*lo);
            let mut rkeys = rk.clone();
            rkeys.push(*ro);
            let li = index(lrows, lw, &lkeys, Some(&lids), &mut w.stats);
            let ri = index(rrows, rw, &rkeys, Some(&rids), &mut w.stats);
            let k = lk.len() + 1;
            let iw = k + 2;
            let nr = ri.len() / iw;
            let mut next = 0usize;
            for entry in li.chunks_exact(iw) {
                let lkey = &entry[..k];
                let lord = num_of(entry[k]);
                while next < nr {
                    let rentry = &ri[next * iw..next * iw + iw];
                    let order = lkey.cmp(&rentry[..k]);
                    if order.is_gt() || (order.is_eq() && num_of(rentry[k]) <= lord) {
                        next += 1;
                    } else {
                        break;
                    }
                }
                if next >= nr {
                    break;
                }
                let rentry = &ri[next * iw..next * iw + iw];
                if rentry[..k] != *lkey {
                    continue;
                }
                let (a, b) = (entry[k + 1] as usize, rentry[k + 1] as usize);
                out.push(entry[0]);
                for (right, c) in cols {
                    out.push(if *right { rrows[b * rw + c] } else { lrows[a * lw + c] });
                }
                next += 1;
            }
        }
        Op::Closure { src, keys, from, to } => {
            let width = width_of(program, *src);
            out = closure_rows(source(*src, results, rels), width, keys, *from, *to, live, None);
        }
        Op::Axis { starts, edges, domain, dir, id, parent, child, domain_id } => {
            use ryo_values::domain::plan_parts::types::Direction;
            let up = matches!(dir, Direction::Parent | Direction::Ancestor | Direction::NearestAncestor);
            let (from, to) = if up { (*child, *parent) } else { (*parent, *child) };
            let (sw, ew, dw) = (width_of(program, *starts), width_of(program, *edges), width_of(program, *domain));
            let edge_rows = source(*edges, results, rels);
            let pairs_made = edge_pairs(&mut w.pair_cache, *edges, edge_rows, ew, from, to, live, &mut w.stats);
            let pairs: &[u64] = &pairs_made;
            let mut members = Vec::new();
            for row in source(*domain, results, rels).chunks_exact(dw) {
                if live[row[0] as usize] {
                    members.extend_from_slice(&[row[0], row[*domain_id]]);
                }
            }
            sort_rows(&mut members, 2, true);
            let np = pairs.len() / 3;
            let nm = members.len() / 2;
            let neighbours = |file: u64, node: u64| {
                let key = [file, node];
                let lo = gallop(&pairs, 3, 2, 0, &key);
                let mut hi = lo;
                while hi < np && pairs[hi * 3] == file && pairs[hi * 3 + 1] == node {
                    hi += 1;
                }
                (lo, hi)
            };
            let member = |file: u64, node: u64| {
                let key = [file, node];
                let at = gallop(&members, 2, 2, 0, &key);
                at < nm && members[at * 2] == file && members[at * 2 + 1] == node
            };
            let mut seen = rustc_hash::FxHashSet::default();
            let mut queue = std::collections::VecDeque::new();
            let start_rows = source(*starts, results, rels);
            let start_count = start_rows.len() / sw;
            if *dir == Direction::Ancestor && nm.saturating_mul(8) < start_count {
                // pairs are (file, child, parent) for an upward walk: reverse them to walk down from each member.
                let downs_made = edge_pairs(&mut w.pair_cache, *edges, edge_rows, ew, to, from, live, &mut w.stats);
                let downs: &[u64] = &downs_made;
                let nd = downs.len() / 3;
                let sids: Vec<u32> = (0..start_count).filter(|at| live[start_rows[at * sw] as usize]).map(|at| at as u32).collect();
                let sidx = index(start_rows, sw, &[*id], Some(&sids), &mut w.stats);
                let ns = sidx.len() / 3;
                for member_at in 0..nm {
                    let (file, entity) = (members[member_at * 2], members[member_at * 2 + 1]);
                    seen.clear();
                    queue.clear();
                    queue.push_back(entity);
                    while let Some(node) = queue.pop_front() {
                        let mut e = gallop(&downs, 3, 2, 0, &[file, node]);
                        while e < nd && downs[e * 3] == file && downs[e * 3 + 1] == node {
                            let below = downs[e * 3 + 2];
                            e += 1;
                            if !seen.insert(below) {
                                continue;
                            }
                            queue.push_back(below);
                            let mut s_at = gallop(&sidx, 3, 2, 0, &[file, below]);
                            while s_at < ns && sidx[s_at * 3] == file && sidx[s_at * 3 + 1] == below {
                                let row_at = sidx[s_at * 3 + 2] as usize;
                                out.extend_from_slice(&start_rows[row_at * sw..row_at * sw + sw]);
                                out.push(entity);
                                s_at += 1;
                            }
                        }
                    }
                }
                return out;
            }
            if *dir == Direction::Descendant && nm < start_count.saturating_mul(2) {
                let ups_made = edge_pairs(&mut w.pair_cache, *edges, edge_rows, ew, to, from, live, &mut w.stats);
                let ups: &[u64] = &ups_made;
                let nu = ups.len() / 3;
                let sids: Vec<u32> = (0..start_count).filter(|at| live[start_rows[at * sw] as usize]).map(|at| at as u32).collect();
                let sidx = index(start_rows, sw, &[*id], Some(&sids), &mut w.stats);
                let ns = sidx.len() / 3;
                for member_at in 0..nm {
                    let (file, entity) = (members[member_at * 2], members[member_at * 2 + 1]);
                    seen.clear();
                    queue.clear();
                    queue.push_back(entity);
                    while let Some(node) = queue.pop_front() {
                        let mut e = gallop(&ups, 3, 2, 0, &[file, node]);
                        while e < nu && ups[e * 3] == file && ups[e * 3 + 1] == node {
                            let above = ups[e * 3 + 2];
                            e += 1;
                            if !seen.insert(above) {
                                continue;
                            }
                            queue.push_back(above);
                            let mut s_at = gallop(&sidx, 3, 2, 0, &[file, above]);
                            while s_at < ns && sidx[s_at * 3] == file && sidx[s_at * 3 + 1] == above {
                                let row_at = sidx[s_at * 3 + 2] as usize;
                                out.extend_from_slice(&start_rows[row_at * sw..row_at * sw + sw]);
                                out.push(entity);
                                s_at += 1;
                            }
                        }
                    }
                }
                return out;
            }
            for row in source(*starts, results, rels).chunks_exact(sw) {
                let file = row[0];
                if !live[file as usize] {
                    continue;
                }
                let start = row[*id];
                let deep = matches!(dir, Direction::Ancestor | Direction::NearestAncestor | Direction::Descendant);
                seen.clear();
                queue.clear();
                queue.push_back(start);
                'walk: while let Some(node) = queue.pop_front() {
                    let (lo, hi) = neighbours(file, node);
                    for e in lo..hi {
                        let next = pairs[e * 3 + 2];
                        if !seen.insert(next) {
                            continue;
                        }
                        if deep {
                            queue.push_back(next);
                        }
                        if member(file, next) {
                            out.extend_from_slice(row);
                            out.push(next);
                            if *dir == Direction::NearestAncestor {
                                break 'walk;
                            }
                        }
                    }
                }
            }
        }
        Op::Bounds { src, col } => {
            let width = width_of(program, *src);
            for row in source(*src, results, rels).chunks_exact(width) {
                if !live[row[0] as usize] {
                    continue;
                }
                let cell = row[*col];
                if tag(cell) != TAG_SPAN {
                    continue;
                }
                let Some((start, end)) = span_of(cell) else { continue };
                if start > end {
                    continue;
                }
                out.extend_from_slice(row);
                out.push(num_cell(i64::from(start)));
                out.push(num_cell(i64::from(end)));
            }
        }
    }
    out
}

// ------------------------------------------------------------------------------------------------------------------------
// The end of the run.

fn json_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Evaluates every worker's last batch and writes the findings and the counters.
pub fn finish() {
    let Some(program) = program() else { return };
    let workers: Vec<Arc<Mutex<Worker>>> = WORKERS.lock().unwrap_or_else(|e| e.into_inner()).clone();
    std::thread::scope(|scope| {
        for worker in &workers {
            scope.spawn(move || {
                let mut guard = worker.lock().unwrap_or_else(|e| e.into_inner());
                evaluate(program, &mut guard);
            });
        }
    });
    let mut lines: Vec<(&str, Arc<str>, u32, u32)> = Vec::new();
    let mut stats = Stats::default();
    let mut texts = 0usize;
    for worker in &workers {
        let guard = worker.lock().unwrap_or_else(|e| e.into_inner());
        for (rule, path, start, end) in &guard.findings {
            lines.push((&program.rules[*rule as usize].id, Arc::clone(path), *start, *end));
        }
        let s = &guard.stats;
        for at in 0..NCLASS {
            stats.nanos[at] += s.nanos[at];
            stats.tuples[at] += s.tuples[at];
        }
        stats.index_nanos += s.index_nanos;
        stats.index_tuples += s.index_tuples;
        stats.sort_nanos += s.sort_nanos;
        stats.files += s.files;
        stats.batches += s.batches;
        stats.pruned_rule_files += s.pruned_rule_files;
        stats.run_rule_files += s.run_rule_files;
        stats.dyn_pruned += s.dyn_pruned;
        stats.peak_batch_cells = stats.peak_batch_cells.max(s.peak_batch_cells);
        if stats.node_nanos.len() < s.node_nanos.len() {
            stats.node_nanos.resize(s.node_nanos.len(), 0);
            stats.node_tuples.resize(s.node_tuples.len(), 0);
        }
        for (at, v) in s.node_nanos.iter().enumerate() {
            stats.node_nanos[at] += v;
            stats.node_tuples[at] += s.node_tuples[at];
        }
        texts += guard.strings.texts.len();
    }
    lines.sort();
    if let Some(path) = &switches().out {
        let mut text = String::new();
        for (rule, file, start, end) in &lines {
            text.push_str(&format!(
                "{{\"rule\":{},\"file\":{},\"start\":{start},\"end\":{end}}}\n",
                json_text(rule),
                json_text(file)
            ));
        }
        drop(std::fs::write(path, text));
    }
    if let Some(path) = &switches().stats {
        let mut text = String::from("{");
        text.push_str(&format!("\"batch\":{},\"files\":{},\"batches\":{},", switches().batch, stats.files, stats.batches));
        text.push_str(&format!("\"rules\":{},\"nodes\":{},\"relations\":{},", program.rules.len(), program.needed.len(), program.rels.len()));
        text.push_str(&format!("\"findings\":{},\"texts\":{texts},", lines.len()));
        text.push_str(&format!("\"index_s\":{:.4},\"index_tuples\":{},\"sort_s\":{:.4},", stats.index_nanos as f64 / 1e9, stats.index_tuples, stats.sort_nanos as f64 / 1e9));
        text.push_str(&format!("\"pruned_rule_files\":{},\"run_rule_files\":{},\"dyn_pruned\":{},\"peak_batch_cells\":{},", stats.pruned_rule_files, stats.run_rule_files, stats.dyn_pruned, stats.peak_batch_cells));
        text.push_str("\"classes\":{");
        for at in 0..NCLASS {
            let ns_per = if stats.tuples[at] > 0 { stats.nanos[at] as f64 / stats.tuples[at] as f64 } else { 0.0 };
            text.push_str(&format!(
                "{}\"{}\":{{\"s\":{:.4},\"tuples\":{},\"ns_per_tuple\":{:.1}}}",
                if at > 0 { "," } else { "" },
                CLASSES[at],
                stats.nanos[at] as f64 / 1e9,
                stats.tuples[at],
                ns_per
            ));
        }
        text.push_str("},\"top_nodes\":[");
        let mut order: Vec<usize> = (0..stats.node_nanos.len()).collect();
        order.sort_by_key(|at| std::cmp::Reverse(stats.node_nanos[*at]));
        for (i, at) in order.iter().take(60).enumerate() {
            let owners: Vec<&str> = program.owners[*at].iter().map(|r| &*program.rules[*r as usize].id).collect();
            let class = program.ops[*at].as_ref().map_or(0, Op::class);
            text.push_str(&format!(
                "{}{{\"node\":{at},\"op\":\"{}\",\"s\":{:.4},\"tuples\":{},\"owners\":{}}}",
                if i > 0 { "," } else { "" },
                CLASSES[class],
                stats.node_nanos[*at] as f64 / 1e9,
                stats.node_tuples[*at],
                if owners.len() > 6 { format!("\"{} rules\"", owners.len()) } else { format!("{:?}", owners) }
            ));
        }
        text.push_str("]}\n");
        drop(std::fs::write(path, text));
    }
}
