//! The generator: reads the merged logical program the front end produced and prints, for each rule whose plan it can express,
//! Rust source that runs the plan as nested loops specialised to it. Nothing is keyed on a rule, a rule's text or a relation's
//! name: every choice follows from the operation, its operands and the column kinds and keys the relations crate declares.

use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::{LogicalNode, PlanInput};
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::{
    Operand, PlanComparison, QueryLiteral, RepeatedRows, Side, StepScope, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;

type Res<T> = Result<T, String>;

/// A node that reads one relation's rows in place: which relation, each output column's column of the relation, and the
/// tests on the relation's row.
#[derive(Clone)]
struct View {
    rel: Relation,
    cols: Vec<usize>,
    tests: Vec<(Test, Op, Op)>,
}

/// An operand of a test over a relation row or a node row.
#[derive(Clone)]
enum Op {
    Col(usize),
    Num(i64),
    Text(String),
    Texts(Vec<String>),
}

struct Gen<'a, 'p> {
    nodes: &'a [LogicalNode<'p>],
    kinds: Vec<Option<Vec<CellKind>>>,
    lits: HashMap<String, usize>,
    rels: BTreeMap<String, (usize, Relation)>,
    bidx: HashMap<(usize, usize), usize>,
    mats: HashMap<usize, usize>,
    kidx: HashMap<(usize, Vec<usize>), usize>,
    done: BTreeSet<usize>,
    out: String,
    /// Each node's kinds as the engine's executable program lays them out, read from `RYO_CODEGEN_KINDS_IN`.
    known: HashMap<usize, Vec<CellKind>>,
    /// Nodes the generator cannot express, whose rows the engine's table plan answers (the bridge), with a rule reaching them.
    bridges: BTreeMap<usize, usize>,
    /// The operation that made each bridged node a bridge.
    bridge_why: BTreeMap<usize, String>,
    /// Regex literals, each compiled once per process.
    regexes: Vec<String>,
}

fn name(op: &Operation) -> &'static str {
    match op {
        Operation::Select { .. } => "Select",
        Operation::Project { .. } => "Project",
        Operation::Extend { .. } => "Extend",
        Operation::Join { .. } => "Join",
        Operation::Union { .. } => "Union",
        Operation::Intersection { .. } => "Intersection",
        Operation::Difference { .. } => "Difference",
        Operation::Antijoin { .. } => "Antijoin",
        Operation::GroupCount { .. } => "GroupCount",
        Operation::GroupDigest { .. } => "GroupDigest",
        Operation::Capture { .. } => "Capture",
        Operation::Rank { .. } => "Rank",
        Operation::Replace { .. } => "Replace",
        Operation::OrderedMatch { .. } => "OrderedMatch",
        Operation::Words { .. } => "Words",
        Operation::Axis { .. } => "Axis",
        Operation::Closure { .. } => "Closure",
        Operation::OrderedDfs { .. } => "OrderedDfs",
        Operation::Worklist { .. } => "Worklist",
        Operation::Cycles { .. } => "Cycles",
        Operation::Rows { .. } => "Rows",
        Operation::Hosted { .. } => "Hosted",
        Operation::SpanBetween { .. } => "SpanBetween",
        Operation::SourceMatches { .. } => "SourceMatches",
        Operation::RenderSyntax { .. } => "RenderSyntax",
        Operation::FitsRendered { .. } => "FitsRendered",
        Operation::SpanBounds { .. } => "SpanBounds",
    }
}

fn kind_code(kind: CellKind) -> u8 {
    match kind {
        CellKind::Node => 0,
        CellKind::Symbol => 1,
        CellKind::File => 2,
        CellKind::Span => 3,
        CellKind::Number => 4,
        CellKind::Flag => 5,
    }
}

/// The generated pack's source and a report of the rules it covers and what blocked the rest.
pub fn generate(program: &LogicalProgram<'_>) -> (String, String) {
    let nodes = program.nodes();
    let mut generator = Gen {
        nodes,
        kinds: vec![None; nodes.len()],
        lits: HashMap::new(),
        rels: BTreeMap::new(),
        bidx: HashMap::new(),
        mats: HashMap::new(),
        kidx: HashMap::new(),
        done: BTreeSet::new(),
        out: String::new(),
        known: read_known(),
        bridges: BTreeMap::new(),
        bridge_why: BTreeMap::new(),
        regexes: Vec::new(),
    };
    for at in 0..nodes.len() {
        generator.kinds[at] = generator.node_kinds(at).ok().or_else(|| generator.known.get(&at).cloned());
    }
    let bridging = !generator.known.is_empty();
    let mut rules = Vec::new();
    let mut report = String::new();
    let mut blocked: BTreeMap<String, usize> = BTreeMap::new();
    for (rule, emit) in program.emits() {
        let emit = emit.0 as usize;
        let before = generator.bridges.clone();
        let checked = generator.native_check(emit).and_then(|()| generator.check(emit, &mut BTreeSet::new(), bridging, rule as usize));
        match checked {
            Err(blocker) => {
                generator.bridges = before;
                *blocked.entry(blocker.clone()).or_default() += 1;
                let _ = writeln!(report, "BLOCKED\t{rule}\t{blocker}");
            }
            Ok(()) => {
                let saved = generator.out.len();
                match generator.push_fn(emit) {
                    Ok(()) => {
                        let ks = generator.kinds[emit].clone().unwrap_or_default();
                        let k = ks.len();
                        let codes: Vec<String> = ks.iter().map(|kind| kind_code(*kind).to_string()).collect();
                        let _ = writeln!(
                            generator.out,
                            "pub fn r{rule}(cx: &Cx<'_>, out: &mut Vec<u64>) {{\n    let mut set: FxHashSet<[u64; {k}]> = FxHashSet::default();\n    p{emit}(cx, &mut |r: &[u64; {k}]| {{ let _ = set.insert(*r); }});\n    for r in &set {{ out.extend_from_slice(r); }}\n}}"
                        );
                        rules.push(format!("    ({rule}, {k}, &[{}], r{rule} as RuleFn),", codes.join(", ")));
                        let mut reached = Vec::new();
                        generator.bridged_under(emit, &mut BTreeSet::new(), &mut reached);
                        let why: Vec<String> = reached.iter().map(|b| format!("{b}:{}", generator.bridge_why.get(b).cloned().unwrap_or_default())).collect();
                        let _ = writeln!(report, "COVERED\t{rule}\t{emit}\t{}", why.join(","));
                    }
                    Err(blocker) => {
                        generator.out.truncate(saved);
                        generator.bridges = before;
                        *blocked.entry(blocker.clone()).or_default() += 1;
                        let _ = writeln!(report, "BLOCKED\t{rule}\t{blocker}");
                    }
                }
            }
        }
    }
    for (node, rule) in &generator.bridges {
        let _ = writeln!(report, "BRIDGE\t{node}\t{rule}\t{}", generator.bridge_why.get(node).cloned().unwrap_or_default());
    }
    for (blocker, count) in &blocked {
        let _ = writeln!(report, "BLOCKER\t{count}\t{blocker}");
    }
    let mut source = String::new();
    let _ = writeln!(
        source,
        "// Generated by codegen::generator from the merged logical program (spike ryo-or7l). Do not edit.\nuse super::rt::{{Cx, ABSENT, key_of}};\nuse rustc_hash::FxHashSet;\nuse ryo_relations::domain::relations::types::Relation;\n\npub type RuleFn = fn(&Cx<'_>, &mut Vec<u64>);"
    );
    let bridged: Vec<usize> = generator.bridges.keys().copied().collect();
    let mut bridge_rows = Vec::new();
    for (slot, node) in bridged.iter().enumerate() {
        let mut seen = BTreeSet::new();
        generator.subtree_relations(*node, &mut seen);
        bridge_rows.push(format!("    ({node}, {}),", generator.bridges[node]));
        let _ = slot;
    }
    let _ = writeln!(source, "pub static BRIDGES: &[(u32, u32)] = &[\n{}\n];", bridge_rows.join("\n"));
    let mut rels: Vec<(usize, Relation)> = generator.rels.values().copied().collect();
    rels.sort_by_key(|(slot, _)| *slot);
    let names: Vec<String> = rels.iter().map(|(_, rel)| format!("Relation::{rel:?}")).collect();
    let _ = writeln!(source, "pub static RELS: &[Relation] = &[{}];", names.join(", "));
    let _ = writeln!(
        source,
        "pub const SIZES: [usize; 4] = [{}, {}, {}, {}];",
        generator.lits.len(),
        generator.bidx.len(),
        generator.mats.len(),
        generator.kidx.len()
    );
    let _ = writeln!(source, "static RE: [std::sync::OnceLock<ryo_matching::inbound::use_cases::matching::types::CompiledRegex>; {}] = [const {{ std::sync::OnceLock::new() }}; {}];", generator.regexes.len(), generator.regexes.len());
    let _ = writeln!(source, "pub static RULES: &[(u32, usize, &[u8], RuleFn)] = &[\n{}\n];", rules.join("\n"));
    source.push_str(&generator.out);
    (source, report)
}

impl Gen<'_, '_> {
    fn inputs(&self, at: usize) -> &[PlanInput] {
        self.nodes[at].inputs.0
    }

    fn input_node(&self, at: usize, i: usize) -> Res<usize> {
        match self.inputs(at).get(i) {
            Some(PlanInput::Node(node)) => Ok(node.0 as usize),
            Some(PlanInput::Relation(_)) => Err("relation operand of a binary node".into()),
            Some(PlanInput::Frontier(_)) => Err("input: frontier".into()),
            Some(PlanInput::Observation(_)) => Err("input: bound observation".into()),
            None => Err("input: missing".into()),
        }
    }

    fn input_kinds(&self, at: usize, i: usize) -> Res<Vec<CellKind>> {
        match self.inputs(at).get(i) {
            Some(PlanInput::Relation(rel)) => Ok(kinds(*rel).to_vec()),
            Some(PlanInput::Node(node)) => self.kinds[node.0 as usize].clone().ok_or_else(|| "input kinds".into()),
            Some(PlanInput::Frontier(_)) => Err("input: frontier".into()),
            Some(PlanInput::Observation(_)) => Err("input: bound observation".into()),
            None => Err("input: missing".into()),
        }
    }

    fn node_kinds(&self, at: usize) -> Res<Vec<CellKind>> {
        let node = &self.nodes[at];
        let input = |i| self.input_kinds(at, i);
        Ok(match node.operation {
            Operation::Select { kept, .. } => {
                let ks = input(0)?;
                if kept.is_empty() { ks } else { kept.iter().map(|c| ks[c.0 as usize]).collect() }
            }
            Operation::Project { columns, .. } => {
                let ks = input(0)?;
                columns.iter().map(|c| ks.get(c.0 as usize).copied().ok_or("column")).collect::<Result<_, _>>()?
            }
            Operation::Extend { values, .. } => {
                let mut ks = input(0)?;
                for value in values {
                    ks.push(match value {
                        Operand::Literal(QueryLiteral::Number(_)) => CellKind::Number,
                        Operand::Literal(QueryLiteral::Text(_)) => return Err("Extend: text (minted)".into()),
                        _ => return Err("Extend: parameter or function".into()),
                    });
                }
                ks
            }
            Operation::Join { output, view, .. } => {
                let (l, r) = (input(0)?, input(1)?);
                output
                    .iter()
                    .map(|c| {
                        let col = match c.side {
                            Side::Left => l.get(c.column.0 as usize),
                            Side::Right => {
                                let placed = view.get(c.column.0 as usize).map_or(c.column.0, |p| p.0);
                                r.get(placed as usize)
                            }
                        };
                        col.copied().ok_or_else(|| "join column".to_string())
                    })
                    .collect::<Res<_>>()?
            }
            Operation::Union { .. } | Operation::Intersection { .. } | Operation::Difference { .. } | Operation::Antijoin { .. } => input(0)?,
            Operation::SpanBounds { .. } => {
                let mut ks = input(0)?;
                ks.push(CellKind::Number);
                ks.push(CellKind::Number);
                ks
            }
            Operation::GroupCount { keys, .. } => {
                let ks = input(0)?;
                if !self.is_set(self.input_node(at, 0)?) {
                    return Err("GroupCount over rows that may repeat".into());
                }
                let mut out: Vec<CellKind> = keys.iter().map(|c| ks.get(c.0 as usize).copied().ok_or("column")).collect::<Result<_, _>>()?;
                out.push(CellKind::Number);
                out
            }
            other => return Err(name(other).into()),
        })
    }

    /// Whether every node the emit reads can be printed; the first blocker otherwise.
    fn check(&mut self, at: usize, seen: &mut BTreeSet<usize>, bridging: bool, rule: usize) -> Res<()> {
        if !seen.insert(at) || self.bridges.contains_key(&at) {
            return Ok(());
        }
        match self.check_native(at, seen, bridging, rule) {
            Ok(()) => Ok(()),
            Err(why) if bridging && self.nodes[at].scope == StepScope::PerFile && self.known.get(&at).is_some_and(|k| !k.is_empty()) && self.native_check(at).is_err() => {
                let _ = self.bridges.insert(at, rule);
                let _ = self.bridge_why.insert(at, self.native_check(at).err().unwrap_or(why));
                Ok(())
            }
            Err(why) => Err(why),
        }
    }

    /// Whether this node alone (not its inputs) can be printed.
    fn native_check(&self, at: usize) -> Res<()> {
        let node = &self.nodes[at];
        self.node_kinds(at)?;
        if self.kinds[at].as_ref().is_some_and(Vec::is_empty) {
            return Err("zero-column rows".into());
        }
        match node.operation {
            Operation::Select { comparisons, .. } => {
                let ks = self.input_kinds(at, 0)?;
                for comparison in comparisons {
                    self.comparison(comparison, &ks, node.bindings.0)?;
                }
            }
            Operation::Join { filter, view, reach, .. } => {
                if !filter.is_empty() || !view.is_empty() {
                    return Err("Join: filter or view".into());
                }
                if reach.is_some() {
                    return Err("Join: reach".into());
                }
            }
            _ => {}
        }
        for input in self.inputs(at) {
            match input {
                PlanInput::Frontier(_) => return Err("input: frontier".into()),
                PlanInput::Observation(_) => return Err("input: bound observation".into()),
                _ => {}
            }
        }
        Ok(())
    }

    fn bridged_under(&self, at: usize, seen: &mut BTreeSet<usize>, out: &mut Vec<usize>) {
        if !seen.insert(at) {
            return;
        }
        if self.bridges.contains_key(&at) {
            out.push(at);
            return;
        }
        for input in self.inputs(at) {
            if let PlanInput::Node(n) = input {
                self.bridged_under(n.0 as usize, seen, out);
            }
        }
    }

    fn subtree_relations(&mut self, at: usize, seen: &mut BTreeSet<usize>) {
        if !seen.insert(at) {
            return;
        }
        let inputs: Vec<PlanInput> = self.inputs(at).to_vec();
        for input in inputs {
            match input {
                PlanInput::Node(n) => self.subtree_relations(n.0 as usize, seen),
                PlanInput::Relation(rel) if crate::domain::tree_cells::service::native_form(rel).is_none() => {
                    let _ = self.rel(rel);
                }
                _ => {}
            }
        }
    }

    fn check_native(&mut self, at: usize, seen: &mut BTreeSet<usize>, bridging: bool, rule: usize) -> Res<()> {
        self.node_kinds(at)?;
        let node = &self.nodes[at];
        if node.scope != StepScope::PerFile {
            return Err("scope: after the files".into());
        }
        if self.kinds[at].is_none() {
            return Err(self.node_kinds(at).err().unwrap_or_else(|| "kinds".into()));
        }
        if self.kinds[at].as_ref().is_some_and(Vec::is_empty) {
            return Err("zero-column rows".into());
        }
        match node.operation {
            Operation::Select { comparisons, .. } => {
                let ks = self.input_kinds(at, 0)?;
                for comparison in comparisons {
                    self.comparison(comparison, &ks, node.bindings.0)?;
                }
            }
            Operation::Join { filter, view, reach, .. } => {
                if !filter.is_empty() || !view.is_empty() {
                    return Err("Join: filter or view".into());
                }
                if reach.is_some() {
                    return Err("Join: reach".into());
                }
            }
            Operation::Project { distinct: RepeatedRows::Kept, .. } => {}
            _ => {}
        }
        for input in self.inputs(at).to_vec() {
            match &input {
                PlanInput::Node(node) => self.check(node.0 as usize, seen, bridging, rule)?,
                PlanInput::Relation(_) => {}
                PlanInput::Frontier(_) => return Err("input: frontier".into()),
                PlanInput::Observation(_) => return Err("input: bound observation".into()),
            }
        }
        Ok(())
    }

    fn operand(&self, operand: &Operand, bindings: &[ParameterBinding]) -> Res<Op> {
        Ok(match operand {
            Operand::Column(c) => Op::Col(c.0 as usize),
            Operand::Literal(QueryLiteral::Number(n)) => Op::Num(n.0),
            Operand::Literal(QueryLiteral::Text(t)) => Op::Text(t.0.to_string()),
            Operand::Parameter(p) => match bindings.get(p.0 as usize) {
                Some(ParameterBinding::Text(t)) => Op::Text(t.0.to_string()),
                Some(ParameterBinding::Texts(ts)) => Op::Texts(ts.iter().map(|t| t.0.to_string()).collect()),
                _ => return Err("test: parameter operand".into()),
            },
            Operand::Function(_) => return Err("test: function operand".into()),
        })
    }

    fn comparison(&self, comparison: &PlanComparison, ks: &[CellKind], bindings: &[ParameterBinding]) -> Res<(Test, Op, Op)> {
        let (left, right) = (self.operand(&comparison.left, bindings)?, self.operand(&comparison.right, bindings)?);
        let kind = |op: &Op| match op {
            Op::Col(c) => ks.get(*c).copied(),
            Op::Num(_) => Some(CellKind::Number),
            Op::Text(_) | Op::Texts(_) => Some(CellKind::Symbol),
        };
        if matches!(left, Op::Texts(_)) || (matches!(right, Op::Texts(_)) && !matches!(comparison.test, Test::In | Test::NotIn)) {
            return Err("test: a set outside membership".into());
        }
        let (lk, rk) = (kind(&left).ok_or("test column")?, kind(&right).ok_or("test column")?);
        match comparison.test {
            Test::Equal | Test::NotEqual => {
                if lk != rk {
                    return Err(format!("test: {lk:?} against {rk:?}"));
                }
            }
            Test::LessThan | Test::LessThanOrEqual | Test::GreaterThan | Test::GreaterThanOrEqual => {
                if lk != CellKind::Number || rk != CellKind::Number {
                    return Err(format!("test: order on {lk:?}"));
                }
            }
            Test::StartsWith | Test::EndsWith | Test::Matches => {
                if lk != CellKind::Symbol || !matches!(right, Op::Text(_)) || !matches!(left, Op::Col(_)) {
                    return Err(format!("test: {:?} on a non-text", comparison.test));
                }
            }
            Test::In | Test::NotIn => {
                if lk != CellKind::Symbol || !matches!(right, Op::Texts(_)) {
                    return Err("test: membership outside text sets".into());
                }
            }
            other => return Err(format!("test: {other:?}")),
        }
        Ok((comparison.test, left, right))
    }

    fn lit(&mut self, text: &str) -> usize {
        let next = self.lits.len();
        *self.lits.entry(text.to_string()).or_insert(next)
    }

    fn rel(&mut self, rel: Relation) -> usize {
        let next = self.rels.len();
        self.rels.entry(format!("{rel:?}")).or_insert((next, rel)).0
    }

    /// The expression of an operand, `cell(c)` printing a column.
    fn op_expr(&mut self, op: &Op, cell: &dyn Fn(usize) -> String) -> String {
        match op {
            Op::Col(c) => cell(*c),
            Op::Num(n) => format!("{}u64", *n as u64),
            Op::Text(t) => {
                let k = self.lit(t);
                format!("cx.lit({k}, {t:?})")
            }
            Op::Texts(_) => "0".into(),
        }
    }

    fn test_expr(&mut self, (test, left, right): &(Test, Op, Op), cell: &dyn Fn(usize) -> String) -> String {
        let (l, r) = (self.op_expr(left, cell), self.op_expr(right, cell));
        match test {
            Test::Equal => format!("({l} == {r})"),
            Test::NotEqual => format!("({l} != {r})"),
            Test::LessThan => format!("(({l}) as i64) < (({r}) as i64)"),
            Test::LessThanOrEqual => format!("(({l}) as i64) <= (({r}) as i64)"),
            Test::GreaterThan => format!("(({l}) as i64) > (({r}) as i64)"),
            Test::GreaterThanOrEqual => format!("(({l}) as i64) >= (({r}) as i64)"),
            Test::StartsWith | Test::EndsWith => {
                let Op::Text(t) = right else { return "false".into() };
                let method = if *test == Test::StartsWith { "starts_with" } else { "ends_with" };
                format!("cx.text({l}).{method}({t:?})")
            }
            Test::Matches => {
                let Op::Text(t) = right else { return "false".into() };
                let k = self.regexes.len();
                self.regexes.push(t.clone());
                format!("super::rt::matches(&RE[{k}], {t:?}, cx.text({l}))")
            }
            Test::In | Test::NotIn => {
                let Op::Texts(ts) = right else { return "false".into() };
                let mut arms = Vec::new();
                for t in ts {
                    let k = self.lit(t);
                    arms.push(format!("x == cx.lit({k}, {t:?})"));
                }
                let any = if arms.is_empty() { "false".to_string() } else { arms.join(" || ") };
                let any = format!("{{ let x = {l}; {any} }}");
                if *test == Test::In { any } else { format!("!{any}") }
            }
            _ => "false".into(),
        }
    }

    /// The view of a node that reads one relation in place, when it is one.
    fn view(&self, at: usize) -> Option<View> {
        if self.bridges.contains_key(&at) {
            return None;
        }
        let node = &self.nodes[at];
        let base = |i: usize| -> Option<View> {
            match node.inputs.0.get(i)? {
                PlanInput::Relation(rel) => Some(View { rel: *rel, cols: (0..kinds(*rel).len()).collect(), tests: Vec::new() }),
                PlanInput::Node(n) => self.view(n.0 as usize),
                _ => None,
            }
        };
        match node.operation {
            Operation::Select { comparisons, kept, .. } => {
                let mut v = base(0)?;
                let ks = self.input_kinds(at, 0).ok()?;
                for comparison in comparisons {
                    let (test, l, r) = self.comparison(comparison, &ks, node.bindings.0).ok()?;
                    let map = |op: Op| match op {
                        Op::Col(c) => Op::Col(v.cols[c]),
                        other => other,
                    };
                    let mapped = (test, map(l), map(r));
                    v.tests.push(mapped);
                }
                if !kept.is_empty() {
                    v.cols = kept.iter().map(|c| v.cols[c.0 as usize]).collect();
                }
                Some(v)
            }
            Operation::Project { columns, .. } => {
                let mut v = base(0)?;
                v.cols = columns.iter().map(|c| v.cols.get(c.0 as usize).copied()).collect::<Option<_>>()?;
                Some(v)
            }
            _ => None,
        }
    }

    /// Whether the engine's rows of a node never repeat, so the count of a grouping over them is the count of distinct rows.
    fn is_set(&self, at: usize) -> bool {
        match self.nodes[at].operation {
            Operation::Project { distinct: RepeatedRows::Dropped, .. } | Operation::Union { .. } | Operation::GroupCount { .. } => true,
            Operation::Select { .. } | Operation::Extend { .. } | Operation::Antijoin { .. } | Operation::Difference { .. } | Operation::Intersection { .. } | Operation::SpanBounds { .. } => {
                match self.inputs(at).first() {
                    Some(PlanInput::Relation(_)) => true,
                    Some(PlanInput::Node(n)) => self.is_set(n.0 as usize),
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn arity(&self, at: usize) -> usize {
        self.kinds[at].as_ref().map_or(0, Vec::len)
    }

    /// Prints the loop over a view's rows, each row bound to `i`, around `body`; returns the code.
    fn view_loop(&mut self, v: &View, body: &str) -> String {
        let rel = self.rel(v.rel);
        let ks = kinds(v.rel);
        let bucket = v.tests.iter().position(|(test, l, r)| {
            *test == Test::Equal
                && matches!(l, Op::Col(c) if matches!(ks[*c], CellKind::Symbol | CellKind::Number))
                && matches!(r, Op::Text(_) | Op::Num(_))
        });
        let tests: Vec<(Test, Op, Op)> = v.tests.iter().enumerate().filter(|(k, _)| Some(*k) != bucket).map(|(_, t)| t.clone()).collect();
        let cell = move |c: usize| format!("cx.c({rel}, {c}, i)");
        let conds: Vec<String> = tests.iter().map(|t| self.test_expr(t, &cell)).collect();
        let cond = if conds.is_empty() { "true".to_string() } else { conds.join(" && ") };
        match bucket {
            Some(b) => {
                let (_, Op::Col(col), lit) = &v.tests[b] else { unreachable!() };
                let col = *col;
                let key = self.op_expr(lit, &cell);
                let next = self.bidx.len();
                let slot = *self.bidx.entry((rel, col)).or_insert(next);
                format!(
                    "if cx.lens[{rel}] != 0 {{ let key = {key}; if key != ABSENT {{ for &i in cx.bidx({slot}, {rel}, {col}, false).get(key) {{ if {cond} {{ cx.visits.set(cx.visits.get() + 1); {body} }} }} }} }}"
                )
            }
            None => format!("for i in 0..cx.lens[{rel}] as u32 {{ if {cond} {{ cx.visits.set(cx.visits.get() + 1); {body} }} }}"),
        }
    }

    /// Prints node `at`'s push function and those of the nodes it drives from.
    fn push_fn(&mut self, at: usize) -> Res<()> {
        if self.done.contains(&at) {
            return Ok(());
        }
        let k = self.arity(at);
        let body = if let Some(slot) = self.bridges.keys().position(|b| *b == at) {
            format!("let m = cx.bridged({slot}); for j in (0..m.len()).step_by({k}) {{ f(<&[u64; {k}]>::try_from(&m[j..j + {k}]).unwrap()); }}")
        } else if let Some(v) = self.view(at) {
            let rel = self.rel(v.rel);
            let cells: Vec<String> = v.cols.iter().map(|c| format!("cx.c({rel}, {c}, i)")).collect();
            self.view_loop(&v, &format!("f(&[{}]);", cells.join(", ")))
        } else {
            self.push_body(at)?
        };
        let _ = writeln!(self.out, "#[inline]\nfn p{at}<F: FnMut(&[u64; {k}])>(cx: &Cx<'_>, f: &mut F) {{\n    {body}\n}}");
        let _ = self.done.insert(at);
        Ok(())
    }

    fn drive(&mut self, at: usize, i: usize, row: &str, body: &str) -> Res<String> {
        let input = self.input_node(at, i)?;
        self.push_fn(input)?;
        let k = self.arity(input);
        Ok(format!("p{input}(cx, &mut |{row}: &[u64; {k}]| {{ {body} }});"))
    }

    /// The right side of a keyed match: code binding nothing, run per left row `l`, calling `hit` for each right row matching
    /// on every key, with `rc(c)` the right column `c` of that row. `empty` is code run before the drive, returning early when
    /// the right side holds no row.
    fn right_side(
        &mut self,
        at: usize,
        keys: &[(usize, usize)],
        hit: &dyn Fn(&dyn Fn(usize) -> String) -> String,
    ) -> Res<(String, String)> {
        let right = self.input_node(at, 1)?;
        let rk = self.kinds[right].clone().ok_or("kinds")?;
        if let Some(v) = self.view(right) {
            let rel = self.rel(v.rel);
            let ks = kinds(v.rel);
            let rc = |c: usize| format!("cx.c({rel}, {}, i)", v.cols[c]);
            let cell = move |c: usize| format!("cx.c({rel}, {c}, i)");
            let mut conds: Vec<String> = v.tests.clone().iter().map(|t| self.test_expr(t, &cell)).collect();
            let code = match keys.split_first() {
                Some((&(l0, r0), rest)) => {
                    for &(l, r) in rest {
                        conds.push(format!("({} == l[{l}])", rc(r)));
                    }
                    let cond = if conds.is_empty() { "true".into() } else { conds.join(" && ") };
                    let col = v.cols[r0];
                    let next = self.bidx.len();
                    let slot = *self.bidx.entry((rel, col)).or_insert(next);
                    let node = ks[col] == CellKind::Node;
                    format!("for &i in cx.bidx({slot}, {rel}, {col}, {node}).get(l[{l0}]) {{ if {cond} {{ {} }} }}", hit(&rc))
                }
                None => {
                    let cond = if conds.is_empty() { "true".into() } else { conds.join(" && ") };
                    format!("for i in 0..cx.lens[{rel}] as u32 {{ if {cond} {{ {} }} }}", hit(&rc))
                }
            };
            return Ok((format!("if cx.lens[{rel}] == 0 {{ EMPTY }}"), code));
        }
        self.push_fn(right)?;
        let kr = rk.len();
        let next = self.mats.len();
        let ms = *self.mats.entry(right).or_insert(next);
        let mat = format!("let m = cx.mat({ms}, |v| p{right}(cx, &mut |r: &[u64; {kr}]| v.extend_from_slice(r)));");
        let rc = |c: usize| format!("m[j + {c}]");
        let code = match keys.len() {
            0 => format!("for j in (0..m.len()).step_by({kr}) {{ {} }}", hit(&rc)),
            _ => {
                let rkeys: Vec<usize> = keys.iter().map(|(_, r)| *r).collect();
                let next = self.kidx.len();
                let ks = *self.kidx.entry((right, rkeys.clone())).or_insert(next);
                let node = keys.len() == 1 && rk[rkeys[0]] == CellKind::Node;
                let key = if keys.len() == 1 {
                    format!("l[{}]", keys[0].0)
                } else {
                    let ls: Vec<String> = keys.iter().map(|(l, _)| format!("l[{l}]")).collect();
                    format!("key_of(&[{}])", ls.join(", "))
                };
                let eq: Vec<String> = keys.iter().map(|(l, r)| format!("m[j + {r}] == l[{l}]")).collect();
                let rs: Vec<String> = rkeys.iter().map(ToString::to_string).collect();
                format!(
                    "for &ri in idx.get({key}) {{ let j = ri as usize * {kr}; if {} {{ {} }} }}",
                    eq.join(" && "),
                    hit(&rc)
                )
                .replace("idx.get", &format!("cx.kidx({ks}, m, {kr}, &[{}], {node}).get", rs.join(", ")))
            }
        };
        Ok((format!("{mat} if m.is_empty() {{ EMPTY }}"), code))
    }

    fn key_pairs(&self, at: usize, left_on: &[ryo_values::domain::plan_parts::types::ColumnIndex], right_on: &[ryo_values::domain::plan_parts::types::ColumnIndex]) -> Res<Vec<(usize, usize)>> {
        let (lk, rk) = (self.input_kinds(at, 0)?, self.input_kinds(at, 1)?);
        let mut pairs = Vec::new();
        for (l, r) in left_on.iter().zip(right_on) {
            let (l, r) = (l.0 as usize, r.0 as usize);
            // Every row of one file's facts names the same file: a key pair of two file columns always holds.
            if lk[l] == CellKind::File && rk[r] == CellKind::File {
                continue;
            }
            if lk[l] != rk[r] {
                return Err(format!("key: {:?} against {:?}", lk[l], rk[r]));
            }
            pairs.push((l, r));
        }
        Ok(pairs)
    }

    fn push_body(&mut self, at: usize) -> Res<String> {
        let node = &self.nodes[at];
        Ok(match node.operation {
            Operation::Select { comparisons, kept, .. } => {
                let ks = self.input_kinds(at, 0)?;
                let tests: Vec<_> = comparisons.iter().map(|c| self.comparison(c, &ks, node.bindings.0)).collect::<Res<_>>()?;
                let cell = |c: usize| format!("r[{c}]");
                let conds: Vec<String> = tests.iter().map(|t| self.test_expr(t, &cell)).collect();
                let cond = if conds.is_empty() { "true".into() } else { conds.join(" && ") };
                let emit = if kept.is_empty() {
                    "f(r);".to_string()
                } else {
                    let cs: Vec<String> = kept.iter().map(|c| format!("r[{}]", c.0)).collect();
                    format!("f(&[{}]);", cs.join(", "))
                };
                self.drive(at, 0, "r", &format!("if {cond} {{ {emit} }}"))?
            }
            Operation::Project { columns, .. } => {
                let cs: Vec<String> = columns.iter().map(|c| format!("r[{}]", c.0)).collect();
                self.drive(at, 0, "r", &format!("f(&[{}]);", cs.join(", ")))?
            }
            Operation::Extend { values, .. } => {
                let k = self.arity(self.input_node(at, 0)?);
                let mut cs: Vec<String> = (0..k).map(|c| format!("r[{c}]")).collect();
                for value in values {
                    cs.push(match value {
                        Operand::Literal(QueryLiteral::Number(n)) => format!("{}u64", n.0 as u64),
                        Operand::Literal(QueryLiteral::Text(_)) => return Err("Extend: text (minted)".into()),
                        _ => return Err("Extend: parameter or function".into()),
                    });
                }
                self.drive(at, 0, "r", &format!("f(&[{}]);", cs.join(", ")))?
            }
            Operation::Union { .. } => {
                let (a, b) = (self.input_node(at, 0)?, self.input_node(at, 1)?);
                self.push_fn(a)?;
                self.push_fn(b)?;
                format!("p{a}(cx, f); p{b}(cx, f);")
            }
            Operation::Join { left_on, right_on, output, .. } => {
                let keys = self.key_pairs(at, left_on, right_on)?;
                let out = output.clone();
                let hit = move |rc: &dyn Fn(usize) -> String| {
                    let cs: Vec<String> = out
                        .iter()
                        .map(|c| match c.side {
                            Side::Left => format!("l[{}]", c.column.0),
                            Side::Right => rc(c.column.0 as usize),
                        })
                        .collect();
                    format!("f(&[{}]);", cs.join(", "))
                };
                let (empty, code) = self.right_side(at, &keys, &hit)?;
                let drive = self.drive(at, 0, "l", &code)?;
                format!("{} {drive}", empty.replace("EMPTY", "return;"))
            }
            Operation::Antijoin { left_on, right_on, .. } => {
                let keys = self.key_pairs(at, left_on, right_on)?;
                self.absence(at, &keys, false)?
            }
            Operation::SpanBounds { span, .. } => {
                let k = self.arity(self.input_node(at, 0)?);
                let cs: Vec<String> = (0..k).map(|c| format!("r[{c}]")).collect();
                self.drive(at, 0, "r", &format!("let s = r[{}]; if super::rt::span_ok(s) {{ f(&[{}, s >> 32, s & 0xffff_ffff]); }}", span.0, cs.join(", ")))?
            }
            Operation::GroupCount { keys, distinct, .. } => {
                let input = self.input_node(at, 0)?;
                self.push_fn(input)?;
                let ki = self.arity(input);
                let kk = keys.len();
                let ks: Vec<String> = keys.iter().map(|c| format!("r[{}]", c.0)).collect();
                let add = match distinct {
                    Some(d) => format!("let _ = e.1.insert(r[{}]);", d.0),
                    None => "e.0 += 1;".to_string(),
                };
                let count = if distinct.is_some() { "d.len() as u64" } else { "*n" };
                let outs: Vec<String> = (0..kk).map(|c| format!("k[{c}]")).collect();
                format!(
                    "let mut seen: FxHashSet<[u64; {ki}]> = FxHashSet::default(); let mut groups: rustc_hash::FxHashMap<[u64; {kk}], (u64, FxHashSet<u64>)> = Default::default(); p{input}(cx, &mut |r: &[u64; {ki}]| {{ if seen.insert(*r) {{ let e = groups.entry([{}]).or_default(); {add} }} }}); for (k, (n, d)) in &groups {{ let _ = (n, d); f(&[{}, {count}]); }}",
                    ks.join(", "),
                    outs.join(", ")
                )
            }
            Operation::Difference { .. } | Operation::Intersection { .. } => {
                let k = self.arity(at);
                let keys: Vec<(usize, usize)> = (0..k).map(|c| (c, c)).collect();
                self.absence(at, &keys, matches!(node.operation, Operation::Intersection { .. }))?
            }
            other => return Err(name(other).into()),
        })
    }

    /// A left row kept when the right side holds a match on every key (`present`) or none.
    fn absence(&mut self, at: usize, keys: &[(usize, usize)], present: bool) -> Res<String> {
        let hit = |_: &dyn Fn(usize) -> String| "found = true; break;".to_string();
        let (empty, code) = self.right_side(at, keys, &hit)?;
        let keep = if present { "found" } else { "!found" };
        let drive = self.drive(at, 0, "l", &format!("let mut found = false; {code} if {keep} {{ f(l); }}"))?;
        let left = self.input_node(at, 0)?;
        let on_empty = if present {
            "return;".to_string()
        } else {
            format!("p{left}(cx, f); return;")
        };
        Ok(format!("{} {drive}", empty.replace("EMPTY", &on_empty)))
    }
}

fn read_known() -> HashMap<usize, Vec<CellKind>> {
    let mut known = HashMap::new();
    let Some(path) = std::env::var_os("RYO_CODEGEN_KINDS_IN") else { return known };
    let Ok(text) = std::fs::read_to_string(path) else { return known };
    for line in text.lines() {
        let mut fields = line.split('\t');
        let (Some(node), Some(codes)) = (fields.next(), fields.next()) else { continue };
        let Ok(node) = node.parse::<usize>() else { continue };
        let kinds = codes
            .split(',')
            .filter(|c| !c.is_empty())
            .filter_map(|c| match c {
                "0" => Some(CellKind::Node),
                "1" => Some(CellKind::Symbol),
                "2" => Some(CellKind::File),
                "3" => Some(CellKind::Span),
                "4" => Some(CellKind::Number),
                "5" => Some(CellKind::Flag),
                _ => None,
            })
            .collect();
        let _ = known.insert(node, kinds);
    }
    known
}

/// The kinds line of one node, as `read_known` reads it.
pub fn kinds_line(node: usize, kinds: &[CellKind]) -> String {
    let codes: Vec<String> = kinds.iter().map(|k| kind_code(*k).to_string()).collect();
    format!("{node}\t{}\n", codes.join(","))
}
