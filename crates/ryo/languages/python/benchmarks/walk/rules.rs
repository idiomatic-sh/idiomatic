//! Rules on top of the walk: each rule's whole lowered plan (see `lower.py`; suppression is the pack's own fragment, lowered like
//! every other step) interpreted lazily per file over the view,
//! relation leaves read in place through access paths (kind bucket, id probe, children of a parent, parent of a child), no fact
//! rows. A rule whose anchors (kind literals) are all absent from the file is not started (kinds-present skip). Suppression is
//! never host code: steps are memoised per file by the merged program's node id, and a join whose left side is empty never
//! evaluates its right side, so the shared fragment runs once per file and only when some rule has candidates.

use crate::Totals;
use crate::sema::{FxMap, Sema};
use crate::symbols::{NULL, Symbols};
use crate::table::Table;
use crate::view::{View, node_kind_name};
use regex_automata::meta::Regex;
use ruff_python_ast::AnyNodeRef;
use ruff_python_ast::token::TokenKind;
use serde_json::Value;
use std::sync::atomic::Ordering;
use std::time::Instant;

const K_NODE: u8 = 0;
const K_SYM: u8 = 1;
const K_FILE: u8 = 2;
const K_SPAN: u8 = 3;
const K_NUM: u8 = 4;
const K_FLAG: u8 = 5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RelId {
    Ss,
    Sc,
    Sl,
    Id,
    Ee,
    St,
    Cm,
    Ir,
}

#[derive(Clone, Copy, Debug)]
enum Input {
    Step(usize),
    Rel(RelId),
}

#[derive(Clone, Debug)]
enum Operand {
    Column(usize),
    Number(i64),
    Text(u32),
    Texts(Vec<u32>),
    Numbers(Vec<i64>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Test {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    NotIn,
    Glob,
    NotGlob,
    Matches,
    Starts,
    Ends,
}

struct Comparison {
    test: Test,
    left: Operand,
    right: Operand,
    regex: Vec<Regex>,
}

enum Op {
    Select { from: Input, comps: Vec<Comparison>, kept: Vec<usize> },
    Project { from: Input, columns: Vec<usize>, distinct: bool },
    Union { left: Input, right: Input },
    Intersection { left: Input, right: Input },
    Difference { left: Input, right: Input },
    Join { left: Input, right: Input, left_on: Vec<usize>, right_on: Vec<usize>, output: Vec<(bool, usize)>, filter: Vec<Comparison> },
    Antijoin { left: Input, right: Input, left_on: Vec<usize>, right_on: Vec<usize> },
    Extend { from: Input, values: Vec<Operand> },
    Axis { from: Input, domain: Input, direction: Dir, id: usize, domain_id: usize, filter: Vec<Comparison>, output: Vec<(bool, usize)> },
    SourceMatches { from: Input, span: usize, base_line: Option<usize>, regex: Regex, capture: Option<String>, capture_text: bool, capture_line: bool },
    SpanBounds { from: Input, span: usize },
    SpanBetween { from: Input, start: usize, end: usize },
    GroupCount { from: Input, keys: Vec<usize>, distinct: Option<usize> },
    Closure { edges: Input, keys: Vec<usize>, source: usize, destination: usize },
    Capture { from: Input, path: usize, pattern: String },
    Replace { from: Input, text: usize, regex: Regex, replacement: String },
    OrderedMatch { left: Input, right: Input, left_on: Vec<usize>, right_on: Vec<usize>, left_order: usize, right_order: usize, output: Vec<(bool, usize)> },
    /// The rule's core, answered by its navigation programs (nav mode): its rows are put in the memo before the tail runs.
    Injected { kinds: Vec<u8> },
}

#[derive(Clone, Copy, PartialEq)]
enum Dir {
    Parent,
    Child,
    Ancestor,
    Descendant,
    Nearest,
}

struct Rule {
    id: String,
    steps: Vec<Op>,
    /// per step: kind literal anchors when it is a semantic-syntax select on kind
    anchors: Vec<Option<Vec<u32>>>,
    /// the merged program's node id of each step: equal ids are one evaluation per file, across rules
    nodes: Vec<u32>,
    root: usize,
    emit_key: Vec<usize>,
}

pub(crate) struct Program {
    rules: Vec<Rule>,
    symbols: Symbols,
    census: Vec<Census>,
    pub(crate) nav: Option<crate::nav::NavProgram>,
}

/// Only what the kinds-present skip needs, for every rule of the pack (covered or not).
struct Census {
    steps: Vec<(u8, Vec<usize>, Option<Vec<u32>>)>, // (class, inputs by position, anchors); class 0 any-input 1 all-inputs 2 left-only 3 empty 4 maybe
    root: usize,
}

fn rel_of(name: &str) -> Option<RelId> {
    Some(match name {
        "semantic-syntax" => RelId::Ss,
        "semantic-child" => RelId::Sc,
        "source-location" => RelId::Sl,
        "identifier" => RelId::Id,
        "embedded-expression" => RelId::Ee,
        "syntax-tree" => RelId::St,
        "comment" => RelId::Cm,
        "indent-region" => RelId::Ir,
        _ => return None,
    })
}

fn rel_kinds(rel: RelId) -> Vec<u8> {
    match rel {
        RelId::Ss | RelId::St => vec![K_NODE, K_SYM, K_SYM, K_FILE, K_SPAN],
        RelId::Cm => vec![K_NODE, K_SYM, K_FILE, K_SPAN, K_FLAG],
        RelId::Ir => vec![K_NODE, K_NODE, K_SYM, K_NUM, K_NUM, K_FILE, K_SPAN],
        RelId::Sc => vec![K_NODE, K_NODE, K_NUM, K_SYM, K_FILE],
        RelId::Sl => vec![K_NODE, K_FILE, K_SPAN, K_NUM, K_NUM, K_NUM, K_NUM, K_NUM, K_NUM],
        RelId::Id => vec![K_NODE, K_SYM, K_FILE, K_SPAN],
        RelId::Ee => vec![K_NODE, K_NODE, K_SYM, K_SYM, K_FILE, K_SPAN],
    }
}

thread_local! {
    static RX: std::cell::RefCell<FxMap<String, Regex>> = std::cell::RefCell::new(FxMap::default());
}

/// One compiled regex per pattern for the whole load (a rule pack repeats the suppression fragment's patterns in every rule).
fn rx(pattern: &str) -> Result<Regex, regex_automata::meta::BuildError> {
    if let Some(r) = RX.with(|m| m.borrow().get(pattern).cloned()) {
        return Ok(r);
    }
    let r = Regex::new(pattern)?;
    RX.with(|m| m.borrow_mut().insert(pattern.to_owned(), r.clone()));
    Ok(r)
}

fn glob_to_regex(glob: &str) -> String {
    let mut out = String::from("^");
    let chars: Vec<char> = glob.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' if chars.get(i + 1) == Some(&'*') => {
                out.push_str(".*");
                i += 1;
                if chars.get(i + 1) == Some(&'/') {
                    out.push_str("/?");
                    i += 1;
                }
            }
            '*' => out.push_str("[^/]*"),
            '?' => out.push_str("[^/]"),
            c => out.push_str(&regex_syntax_escape(c)),
        }
        i += 1;
    }
    out.push('$');
    out
}

fn regex_syntax_escape(c: char) -> String {
    if "\\.+*?()|[]{}^$#&-~".contains(c) { format!("\\{c}") } else { c.to_string() }
}

struct Loader<'a> {
    sym: &'a mut Symbols,
    pos: FxMap<u64, usize>,
}

impl Loader<'_> {
    fn input(&self, v: &Value) -> Result<Input, String> {
        if let Some(s) = v.get("step") {
            return self.pos.get(&s.as_u64().unwrap()).map(|&p| Input::Step(p)).ok_or("unknown step".into());
        }
        if let Some(r) = v.get("relation") {
            return rel_of(r.as_str().unwrap()).map(Input::Rel).ok_or(format!("relation {r}"));
        }
        Err(format!("input {v}"))
    }

    fn operand(&mut self, v: &Value) -> Result<Operand, String> {
        if let Some(c) = v.get("column") {
            return Ok(Operand::Column(c.as_u64().unwrap() as usize));
        }
        if let Some(n) = v.get("number") {
            return Ok(Operand::Number(n.as_i64().unwrap()));
        }
        if let Some(f) = v.get("flag") {
            return Ok(Operand::Number(i64::from(f.as_bool().unwrap())));
        }
        if let Some(t) = v.get("text") {
            return Ok(Operand::Text(self.sym.base(t.as_str().unwrap())));
        }
        if let Some(t) = v.get("texts") {
            return Ok(Operand::Texts(t.as_array().unwrap().iter().map(|x| self.sym.base(x.as_str().unwrap())).collect()));
        }
        if let Some(t) = v.get("numbers") {
            return Ok(Operand::Numbers(t.as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect()));
        }
        Err(format!("operand {v}"))
    }

    fn comps(&mut self, v: &Value) -> Result<Vec<Comparison>, String> {
        let mut out = Vec::new();
        for c in v.as_array().unwrap() {
            let test = match c["test"].as_str().unwrap() {
                "=" => Test::Eq,
                "!=" => Test::Ne,
                "<" => Test::Lt,
                "<=" => Test::Le,
                ">" => Test::Gt,
                ">=" => Test::Ge,
                "in" => Test::In,
                "not-in" => Test::NotIn,
                "glob-any" => Test::Glob,
                "not-glob-any" => Test::NotGlob,
                "matches" => Test::Matches,
                "starts-with" => Test::Starts,
                "ends-with" => Test::Ends,
                t => return Err(format!("test {t}")),
            };
            let left = self.operand(&c["left"])?;
            let right = self.operand(&c["right"])?;
            let mut regex = Vec::new();
            let texts = |sym: &Symbols, o: &Operand| -> Vec<String> {
                match o {
                    Operand::Text(t) => vec![sym.str(*t).unwrap_or("").to_owned()],
                    Operand::Texts(ts) => ts.iter().map(|t| sym.str(*t).unwrap_or("").to_owned()).collect(),
                    _ => vec![],
                }
            };
            if matches!(test, Test::Glob | Test::NotGlob) {
                for g in texts(self.sym, &right) {
                    regex.push(rx(&glob_to_regex(&g)).map_err(|e| e.to_string())?);
                }
            }
            if test == Test::Matches {
                for g in texts(self.sym, &right) {
                    regex.push(rx(&g).map_err(|e| e.to_string())?);
                }
            }
            out.push(Comparison { test, left, right, regex });
        }
        Ok(out)
    }

    fn cols(v: &Value) -> Vec<usize> {
        v.as_array().map(|a| a.iter().map(|x| x.as_u64().unwrap() as usize).collect()).unwrap_or_default()
    }

    fn output(v: &Value) -> Vec<(bool, usize)> {
        v.as_array()
            .map(|a| a.iter().map(|o| (o["side"].as_str() == Some("right"), o["column"].as_u64().unwrap() as usize)).collect())
            .unwrap_or_default()
    }

    fn step(&mut self, s: &Value) -> Result<(Op, Option<Vec<u32>>), String> {
        let op = s["op"].as_str().unwrap();
        let mut anchors = None;
        let built = match op {
            "select" => {
                let from = self.input(&s["from"])?;
                let comps = self.comps(&s["comparisons"])?;
                if let Input::Rel(RelId::Ss) = from {
                    let lits: Vec<u32> = comps
                        .iter()
                        .filter(|c| c.test == Test::Eq && matches!(c.left, Operand::Column(1)))
                        .filter_map(|c| if let Operand::Text(t) = c.right { Some(t) } else { None })
                        .collect();
                    if !lits.is_empty() {
                        anchors = Some(lits);
                    }
                }
                Op::Select { from, comps, kept: Self::cols(&s["kept"]) }
            }
            "project" => Op::Project { from: self.input(&s["from"])?, columns: Self::cols(&s["columns"]), distinct: s["distinct"].as_bool().unwrap_or(false) },
            "union" => Op::Union { left: self.input(&s["left"])?, right: self.input(&s["right"])? },
            "intersection" => Op::Intersection { left: self.input(&s["left"])?, right: self.input(&s["right"])? },
            "difference" => Op::Difference { left: self.input(&s["left"])?, right: self.input(&s["right"])? },
            "join" => {
                let right = self.input(&s["right"])?;
                let view = Self::cols(&s["view"]);
                let mut right_on = Self::cols(&s["right_on"]);
                let mut output = Self::output(&s["output"]);
                // a view over the tree relation: right column k in right_on/output is column view[k]
                if !view.is_empty() && matches!(right, Input::Rel(RelId::St | RelId::Ss)) {
                    right_on = right_on.iter().map(|&k| view[k]).collect();
                    output = output.iter().map(|&(side, c)| (side, if side { view[c] } else { c })).collect();
                }
                Op::Join {
                    left: self.input(&s["left"])?,
                    right,
                    left_on: Self::cols(&s["left_on"]),
                    right_on,
                    output,
                    filter: self.comps(&s["filter"])?,
                }
            }
            "antijoin" => Op::Antijoin {
                left: self.input(&s["left"])?,
                right: self.input(&s["right"])?,
                left_on: Self::cols(&s["left_on"]),
                right_on: Self::cols(&s["right_on"]),
            },
            "extend" => {
                let mut values = Vec::new();
                for v in s["values"].as_array().unwrap() {
                    values.push(self.operand(v)?);
                }
                Op::Extend { from: self.input(&s["from"])?, values }
            }
            "axis" => {
                if s["parent"].as_u64() != Some(0) || s["child"].as_u64() != Some(1) {
                    return Err("axis edge columns".into());
                }
                Op::Axis {
                    from: self.input(&s["from"])?,
                    domain: self.input(&s["domain"])?,
                    direction: match s["direction"].as_str().unwrap() {
                        "parent" => Dir::Parent,
                        "child" => Dir::Child,
                        "ancestor" => Dir::Ancestor,
                        "descendant" => Dir::Descendant,
                        "nearest-ancestor" => Dir::Nearest,
                        d => return Err(format!("axis {d}")),
                    },
                    id: s["id"].as_u64().unwrap() as usize,
                    domain_id: s["domain_id"].as_u64().unwrap() as usize,
                    filter: self.comps(&s["domain_filter"])?,
                    output: Self::output(&s["output"]),
                }
            }
            "source_matches" => Op::SourceMatches {
                from: self.input(&s["from"])?,
                span: s["span"].as_u64().unwrap() as usize,
                base_line: s["base_line"].as_u64().map(|x| x as usize),
                regex: rx(s["pattern"].as_str().unwrap()).map_err(|e| e.to_string())?,
                capture: s["capture"].as_str().map(str::to_owned),
                capture_text: s["capture_text"].as_bool().unwrap_or(false),
                capture_line: s["capture_line"].as_bool().unwrap_or(false),
            },
            "span_bounds" => Op::SpanBounds { from: self.input(&s["from"])?, span: s["span"].as_u64().unwrap() as usize },
            "span_between" => Op::SpanBetween {
                from: self.input(&s["from"])?,
                start: s["start"].as_u64().unwrap() as usize,
                end: s["end"].as_u64().unwrap() as usize,
            },
            "group_count" => Op::GroupCount {
                from: self.input(&s["from"])?,
                keys: Self::cols(&s["keys"]),
                distinct: s["distinct"].as_u64().map(|x| x as usize),
            },
            "closure" => Op::Closure {
                edges: self.input(&s["edges"])?,
                keys: Self::cols(&s["keys"]),
                source: s["source"].as_u64().unwrap() as usize,
                destination: s["destination"].as_u64().unwrap() as usize,
            },
            "capture" => Op::Capture {
                from: self.input(&s["from"])?,
                path: s["path"].as_u64().unwrap() as usize,
                pattern: s["pattern"].as_str().unwrap().to_owned(),
            },
            "replace" => Op::Replace {
                from: self.input(&s["from"])?,
                text: s["text"].as_u64().unwrap() as usize,
                regex: rx(s["pattern"].as_str().unwrap()).map_err(|e| e.to_string())?,
                replacement: s["replacement"].as_str().unwrap().to_owned(),
            },
            "ordered_match" => Op::OrderedMatch {
                left: self.input(&s["left"])?,
                right: self.input(&s["right"])?,
                left_on: Self::cols(&s["left_on"]),
                right_on: Self::cols(&s["right_on"]),
                left_order: s["left_order"].as_u64().unwrap() as usize,
                right_order: s["right_order"].as_u64().unwrap() as usize,
                output: Self::output(&s["output"]),
            },
            "injected" => Op::Injected {
                kinds: s["kinds"].as_array().unwrap().iter().map(|k| crate::nav::cell_kind(k.as_str().unwrap())).collect(),
            },
            o => return Err(format!("op {o}")),
        };
        Ok((built, anchors))
    }
}

impl Program {
    pub(crate) fn load(path: &str) -> Self {
        // bind: the program file is read as a stream, one rule's JSON at a time (each dropped once it is loaded); it was
        // parsed whole into one tree, which was most of a run's peak memory
        use serde::de::{DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
        struct State {
            symbols: Symbols,
            rules: Vec<Rule>,
            nav_builder: crate::nav::NavBuilder,
            nav_count: usize,
            census: Option<Value>,
        }
        fn one_rule(r: &Value, st: &mut State) {
            let State { symbols, rules, nav_builder, nav_count, .. } = st;
            let mut loader = Loader { sym: &mut *symbols, pos: FxMap::default() };
            let steps_v = r["steps"].as_array().unwrap();
            for (i, s) in steps_v.iter().enumerate() {
                loader.pos.insert(s["step"].as_u64().unwrap(), i);
            }
            let mut steps = Vec::new();
            let mut anchors = Vec::new();
            let mut failed = None;
            for s in steps_v {
                match loader.step(s) {
                    Ok((op, a)) => {
                        steps.push(op);
                        anchors.push(a);
                    }
                    Err(e) => {
                        failed = Some(e);
                        break;
                    }
                }
            }
            let id = r["id"].as_str().unwrap().to_owned();
            if let Some(e) = failed {
                eprintln!("skip {id}: {e}");
                return;
            }
            let root = loader.pos[&r["root"].as_u64().unwrap()];
            let nodes = steps_v.iter().map(|s| s["merged"].as_u64().unwrap() as u32).collect();
            rules.push(Rule { id, steps, anchors, nodes, root, emit_key: Loader::cols(&r["emit_key"]) });
            if r.get("nav").is_some() {
                nav_builder.rule(r, symbols);
                *nav_count += 1;
            }
        }
        /// share: the elements of one of the program's lists, each loaded and dropped: the rules, the pack's made tables,
        /// the pack's shared entries
        #[derive(Clone, Copy)]
        enum Piece {
            Rule,
            Made,
            Shared,
        }
        struct Rules<'a>(&'a mut State, Piece);
        impl<'de> DeserializeSeed<'de> for Rules<'_> {
            type Value = ();
            fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
                d.deserialize_seq(self)
            }
        }
        impl<'de> Visitor<'de> for Rules<'_> {
            type Value = ();
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("the rules")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
                while let Some(r) = seq.next_element::<Value>()? {
                    match self.1 {
                        Piece::Rule => one_rule(&r, self.0),
                        Piece::Made => self.0.nav_builder.made(&r, &mut self.0.symbols),
                        Piece::Shared => self.0.nav_builder.shared(&r, &mut self.0.symbols),
                    }
                }
                Ok(())
            }
        }
        struct Top<'a>(&'a mut State);
        impl<'de> Visitor<'de> for Top<'_> {
            type Value = ();
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a program")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
                while let Some(k) = map.next_key::<String>()? {
                    match k.as_str() {
                        "rules" => map.next_value_seed(Rules(&mut *self.0, Piece::Rule))?,
                        "made" => map.next_value_seed(Rules(&mut *self.0, Piece::Made))?,
                        "shared" => map.next_value_seed(Rules(&mut *self.0, Piece::Shared))?,
                        "kinds" => self.0.nav_builder.kinds(map.next_value::<serde_json::Map<String, Value>>()?),
                        "census" => self.0.census = Some(map.next_value::<Value>()?),
                        _ => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }
                Ok(())
            }
        }
        let mut st = State { symbols: Symbols::default(), rules: Vec::new(), nav_builder: Default::default(), nav_count: 0, census: None };
        let file = std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(path).expect("program"));
        let mut de = serde_json::Deserializer::from_reader(file);
        de.deserialize_map(Top(&mut st)).expect("json");
        let State { mut symbols, rules, nav_builder, nav_count, census: census_v } = st;
        let v = serde_json::json!({ "census": census_v.unwrap_or(Value::Null) });
        let mut census = Vec::new();
        if let Some(all) = v["census"].as_array() {
            for r in all {
                let steps_v = r["steps"].as_array().unwrap();
                let mut pos = FxMap::default();
                for (i, s) in steps_v.iter().enumerate() {
                    pos.insert(s["step"].as_u64().unwrap(), i);
                }
                let steps = steps_v
                    .iter()
                    .map(|s| {
                        let class = s["class"].as_u64().unwrap() as u8;
                        let inputs = s["inputs"].as_array().unwrap().iter().map(|x| pos[&x.as_u64().unwrap()]).collect();
                        let anchors = s["kinds"].as_array().map(|k| k.iter().map(|x| symbols.base(x.as_str().unwrap())).collect());
                        (class, inputs, anchors)
                    })
                    .collect();
                census.push(Census { steps, root: pos[&r["root"].as_u64().unwrap()] });
            }
        }
        // node kind names are base symbols before any literal (the view interns them as base symbols per file otherwise)
        let nav = if nav_count == 0 {
            None
        } else {
            assert_eq!(nav_count, rules.len(), "every nav rule's tail loads");
            Some(nav_builder.finish())
        };
        Self { rules, symbols, census, nav }
    }
}

#[derive(Clone, Default)]
struct Rel {
    kinds: Vec<u8>,
    data: Vec<u64>,
}

impl Rel {
    fn new(kinds: Vec<u8>) -> Self {
        Self { kinds, data: Vec::new() }
    }
    fn arity(&self) -> usize {
        self.kinds.len()
    }
    fn len(&self) -> usize {
        if self.kinds.is_empty() { 0 } else { self.data.len() / self.kinds.len() }
    }
    fn row(&self, i: usize) -> &[u64] {
        let a = self.arity();
        &self.data[i * a..(i + 1) * a]
    }
    fn rows(&self) -> impl Iterator<Item = &[u64]> {
        let a = self.arity().max(1);
        self.data.chunks(a)
    }
}

fn span(s: u32, e: u32) -> u64 {
    (u64::from(s) << 32) | u64::from(e)
}

struct Ctx<'a> {
    view: &'a View,
    table: &'a Table<'a>,
    source: &'a str,
    file: u64,
    path: &'a str,
}

pub(crate) struct State<'p> {
    program: &'p Program,
    sym: Symbols,
    view: View,
    memo: FxMap<u32, std::rc::Rc<Rel>>,
    scans: FxMap<u8, std::rc::Rc<Rel>>,
    cur: usize,
    pub evaluated: u64,
    census_on: bool,
    pub probed: u64,
    node_ns: FxMap<u32, (u64, u64)>,
    inner_ns: u64,
    pub op_ns: [u64; 32],
    pub op_rows: [u64; 32],
    present: Vec<bool>,
    may: Vec<bool>,
    pub started: u64,
    pub census_started: u64,
    pub census_rules: u64,
    pub anchor_rows: u64,
    pub view_ns: u64,
    pub eval_ns: u64,
    pub views: u64,
    pub files: u64,
    provider: Option<crate::nav::Provider>,
    cache: crate::nav::FileCache,
    pub nc: crate::nav::Counters,
    table_k: u64,
    pub nav_ns: u64,
    pub tail_ns: u64,
    pub tail_files: u64,
    regs: Vec<u64>,
    outs: Vec<Vec<u64>>,
    model: crate::model::Model,
}

impl<'p> State<'p> {
    pub(crate) fn new(program: &'p Program) -> Self {
        Self {
            program,
            sym: program.symbols.clone(),
            view: View::default(),
            memo: FxMap::default(),
            scans: FxMap::default(),
            cur: 0,
            evaluated: 0,
            census_on: std::env::var("WALK_CENSUS").is_ok(),
            probed: 0,
            node_ns: FxMap::default(),
            inner_ns: 0,
            op_ns: [0; 32],
            op_rows: [0; 32],
            present: Vec::new(),
            may: Vec::new(),
            started: 0,
            census_started: 0,
            census_rules: 0,
            anchor_rows: 0,
            view_ns: 0,
            eval_ns: 0,
            views: 0,
            files: 0,
            provider: None,
            cache: crate::nav::FileCache::default(),
            nc: crate::nav::Counters {
                prof: std::env::var_os("NAV_PROF").is_some(),
                file_log: std::env::var("NAV_FILE_LOG").ok().and_then(|v| v.parse().ok()).unwrap_or(0),
                ..Default::default()
            },
            table_k: std::env::var("NAV_TABLE_K").ok().and_then(|v| v.parse().ok()).unwrap_or(64),
            nav_ns: 0,
            tail_ns: 0,
            tail_files: 0,
            regs: Vec::new(),
            outs: Vec::new(),
            model: crate::model::Model::default(),
        }
    }

    /// Nav mode: the cores run as navigation programs (nodes outer, rules inner); a rule with core rows in the file runs its
    /// tail (the pack's suppression fragment and the emit) on the walk interpreter with the core injected.
    #[allow(clippy::too_many_arguments)]
    fn file_nav(
        &mut self,
        index: usize,
        path: &str,
        source: &str,
        parsed: &ruff_python_parser::Parsed<ruff_python_ast::Mod>,
        table: &Table<'_>,
        out: &mut Vec<String>,
    ) {
        let program = self.program;
        let nav = program.nav.as_ref().unwrap();
        self.files += 1;
        self.sym.reset_file();
        let t0 = Instant::now();
        let mut view = std::mem::take(&mut self.view);
        view.build(table, parsed, source, &mut self.sym);
        self.views += 1;
        let t1 = Instant::now();
        self.view_ns += (t1 - t0).as_nanos() as u64;
        if self.provider.is_none() && nav.needs_provider {
            self.provider = Some(crate::nav::Provider::new());
        }
        self.cache.facts = None;
        self.cache.model_lex = false;
        self.cache.model_flow = false;
        self.cache.ident = None;
        self.cache.tree_idx.clear();
        self.cache.sub_memo.clear();
        let nrules = nav.rules.len();
        let mut outs = std::mem::take(&mut self.outs);
        outs.resize_with(nrules + 1, Vec::new);
        for o in &mut outs {
            o.clear();
        }
        let mut regs = std::mem::take(&mut self.regs);
        regs.resize(nav.max_regs.max(1), 0);
        let fact_before = self.nc.fact_ns;
        let mut aux_rows: Vec<(usize, Vec<(u32, Vec<u8>, Vec<u64>)>)> = Vec::new();
        {
            let f = crate::nav::File { view: &view, table, source, path, file: index, incomplete: !parsed.errors().is_empty(), parsed };
            let mut ex = crate::nav::Exec {
                prog: nav,
                f: &f,
                sym: &mut self.sym,
                cache: &mut self.cache,
                provider: self.provider.as_mut(),
                model: &mut self.model,
                scratch: String::new(),
                c: &mut self.nc,
                regs,
                outs,
                cur_rule: 0,
                collect: Vec::new(),
                union_memo: FxMap::default(),
                sub_memo: FxMap::default(),
                tables: FxMap::default(),
                budget_end: u64::MAX,
                aborted: false,
                check_tables: std::env::var_os("NAV_TABLE_CHECK").is_some(),
                check_seen: Default::default(),
                check_done: Default::default(),
                check_depth: 0,
                no_budget: std::env::var_os("NAV_TABLE_NO_BUDGET").is_some(),
                table_k: self.table_k,
                trace: std::env::var_os("NAV_TRACE_KEYED").is_some(),
                def_rows: FxMap::default(),
                probe_idx: FxMap::default(),
                kbuf: Vec::new(),
                need_state: vec![0; nav.need_slots],
            };
            ex.build_kinds();
            let steps_file = ex.c.steps;
            if ex.c.rule_steps.len() != nrules + 1 {
                ex.c.rule_steps = vec![0; nrules + 1];
                ex.c.rule_visits = vec![0; nrules + 1];
                ex.c.rule_starts = vec![0; nrules + 1];
            }
            // per-file anchor choice: for each branch, the alternative with the least expected work given the real bucket sizes
            let mut dispatch = std::mem::take(&mut ex.cache.dispatch);
            let mut used = std::mem::take(&mut ex.cache.dispatch_used);
            let mut other = std::mem::take(&mut ex.cache.other);
            for &k in &used {
                dispatch[k as usize].clear();
            }
            used.clear();
            other.clear();
            for (ri, rule) in nav.rules.iter().enumerate() {
                for (bi, alts) in rule.programs.iter().enumerate() {
                    // every atom of a branch must hold, so one alternative anchored on an empty bucket proves the branch empty in this
                    // file; otherwise the planner's cheapest alternative runs (its static cost model ranks plans better than bucket
                    // sizes alone, measured: per-file choice by size times static work cost 2x the steps)
                    if alts.iter().any(|a| a.anchor.as_ref().is_some_and(|k| k.iter().all(|&k| ex.bucket(k).is_empty()))) {
                        continue;
                    }
                    let ai = 0usize;
                    let size = alts[0].anchor.as_ref().map_or(1, |k| k.iter().map(|&k| ex.bucket(k).len()).sum::<usize>());
                    let a = &alts[ai];
                    match &a.anchor {
                        Some(kinds) => {
                            if size == 0 {
                                continue;
                            }
                            for &k in kinds.iter() {
                                let n = ex.bucket(k).len();
                                if n == 0 {
                                    continue;
                                }
                                if dispatch.len() <= k as usize {
                                    dispatch.resize_with(k as usize + 1, Vec::new);
                                }
                                if dispatch[k as usize].is_empty() {
                                    used.push(k);
                                }
                                dispatch[k as usize].push((ri as u32, bi as u32, ai as u32));
                                ex.c.candidates += n as u64;
                            }
                            ex.c.starts += 1;
                            ex.c.rule_starts[ri] += 1;
                        }
                        None => {
                            other.push((ri as u32, bi as u32, ai as u32));
                            ex.c.starts += 1;
                            ex.c.rule_starts[ri] += 1;
                        }
                    }
                }
            }
            // nodes outer, rules inner
            for &k in &used {
                let list = &dispatch[k as usize];
                let n = ex.bucket(k).len();
                for i in 0..n {
                    let id = ex.bucket(k)[i];
                    for &(ri, bi, ai) in list {
                        let p: &[crate::nav::Ins] = &nav.rules[ri as usize].programs[bi as usize][ai as usize].prog;
                        if let crate::nav::Ins::IterKind { out, .. } = &p[0] {
                            ex.regs[*out as usize] = u64::from(id);
                        }
                        ex.c.visits += 1;
                        ex.cur_rule = ri as usize;
                        ex.run(p, 1);
                    }
                }
            }
            for &(ri, bi, ai) in &other {
                let p: &[crate::nav::Ins] = &nav.rules[ri as usize].programs[bi as usize][ai as usize].prog;
                ex.cur_rule = ri as usize;
                ex.run(p, 0);
            }
            // navwide: auxiliary relations of the rules whose core has rows
            for (ri, rule) in nav.rules.iter().enumerate() {
                if !rule.aux.is_empty() && !ex.outs[ri].is_empty() {
                    let rows = ex.run_aux(ri);
                    aux_rows.push((ri, rows));
                }
            }
            ex.cache.dispatch = dispatch;
            ex.cache.dispatch_used = used;
            ex.cache.other = other;
            if ex.c.file_log > 0 && ex.c.steps - steps_file >= ex.c.file_log {
                // bind (diagnostic, NAV_FILE_LOG): the file's steps and what its memos hold
                eprintln!(
                    "F\t{}\t{}\tnodes={}\tmemo_entries={}\tmemo_cells={}\tdef_rows_cells={}\tprobe_indexes={}\ttable_cells={}",
                    path,
                    ex.c.steps - steps_file,
                    view.len(),
                    ex.union_memo.len(),
                    ex.union_memo.values().map(|r| r.len()).sum::<usize>(),
                    ex.def_rows.values().map(|r| r.len()).sum::<usize>(),
                    ex.probe_idx.len(),
                    ex.tables.values().map(|t| t.rows.as_ref().map_or(0, |r| r.len())).sum::<usize>()
                );
            }
            regs = ex.regs;
            outs = ex.outs;
        }
        let t2 = Instant::now();
        self.nav_ns += (t2 - t1).as_nanos() as u64 - (self.nc.fact_ns - fact_before);
        let ctx = Ctx { view: &view, table, source, file: 0, path };
        self.memo.clear();
        self.scans.clear();
        let mut any = false;
        for (r, rows) in outs.iter().enumerate().take(nrules) {
            if rows.is_empty() {
                continue;
            }
            any = true;
            let nr = &nav.rules[r];
            let rel = Rel { kinds: nr.kinds.clone(), data: rows.clone() };
            self.memo.insert(nr.xmerged, std::rc::Rc::new(rel));
            let mine: Vec<u32> = aux_rows.iter().filter(|(ri, _)| *ri == r).flat_map(|(_, a)| a.iter()).map(|(m, kinds, data)| {
                self.memo.insert(*m, std::rc::Rc::new(Rel { kinds: kinds.clone(), data: data.clone() }));
                *m
            }).collect();
            let rule = &program.rules[r];
            self.cur = r;
            let emit = self.demand(rule.root, &ctx);
            self.memo.remove(&nr.xmerged);
            for m in mine {
                self.memo.remove(&m);
            }
            if emit.arity() == 0 {
                continue;
            }
            let span_col = (0..emit.arity()).find(|&c| emit.kinds[c] == K_SPAN);
            let mut seen: FxMap<K, ()> = FxMap::default();
            for row in emit.rows() {
                let key: K = rule.emit_key.iter().map(|&k| row[k]).collect();
                if seen.insert(key, ()).is_some() {
                    continue;
                }
                if let Some(sc) = span_col {
                    let s = row[sc];
                    out.push(format!("{}\t{}\t{}\t{}", rule.id, path, s >> 32, s & 0xffff_ffff));
                }
            }
        }
        if any {
            self.tail_files += 1;
        }
        self.memo.clear();
        self.scans.clear();
        self.tail_ns += t2.elapsed().as_nanos() as u64;
        self.view = view;
        self.regs = regs;
        self.outs = outs;
    }

    /// Provider kind names present in the file, from the table's kinds-present bitset (no view needed). Token kinds are taken
    /// as present (not anchored); async variants share their node kind.
    fn kinds_present(&mut self, table: &Table<'_>) {
        self.present.clear();
        self.present.resize(1 << 16, false);
        let mut bits = table.present;
        while bits != 0 {
            let k = bits.trailing_zeros() as u8;
            bits &= bits - 1;
            if let Some(&first) = table.of_kind(k).first() {
                let node = table.node[first as usize];
                let name = node_kind_name(node);
                let mut mark = |s: &str, sym: &mut Symbols, present: &mut Vec<bool>| {
                    if let Some(id) = sym.get_base(s) {
                        if (id as usize) < present.len() {
                            present[id as usize] = true;
                        }
                    }
                };
                mark(name, &mut self.sym, &mut self.present);
                match node {
                    AnyNodeRef::StmtFunctionDef(_) => {
                        mark("function-def", &mut self.sym, &mut self.present);
                        mark("async-function-def", &mut self.sym, &mut self.present);
                    }
                    AnyNodeRef::StmtFor(_) => {
                        mark("for", &mut self.sym, &mut self.present);
                        mark("async-for", &mut self.sym, &mut self.present);
                    }
                    AnyNodeRef::StmtWith(_) => {
                        mark("with", &mut self.sym, &mut self.present);
                        mark("async-with", &mut self.sym, &mut self.present);
                    }
                    _ => {}
                }
            }
        }
    }

    fn anchor_present(&self, lits: &[u32]) -> bool {
        lits.iter().any(|&l| {
            let name = self.sym.str(l).unwrap_or("");
            // a literal that names no node kind is a token kind (or unknown): taken as present
            !is_node_kind_name(name) || self.present.get(l as usize).copied().unwrap_or(false)
        })
    }

    fn rule_may_run(&mut self, rule: &Rule) -> bool {
        self.may.clear();
        for (i, op) in rule.steps.iter().enumerate() {
            let get = |x: &Input, may: &Vec<bool>| match x {
                Input::Step(p) => may[*p],
                Input::Rel(RelId::Ee) => false,
                Input::Rel(_) => true,
            };
            let m = match op {
                Op::Select { from, .. } => {
                    get(from, &self.may) && rule.anchors[i].as_ref().is_none_or(|a| self.anchor_present(a))
                }
                Op::Project { from, .. }
                | Op::Extend { from, .. }
                | Op::SourceMatches { from, .. }
                | Op::SpanBounds { from, .. }
                | Op::SpanBetween { from, .. }
                | Op::GroupCount { from, .. }
                | Op::Capture { from, .. }
                | Op::Replace { from, .. } => get(from, &self.may),
                Op::Injected { .. } => true,
                Op::Closure { edges, .. } => get(edges, &self.may),
                Op::OrderedMatch { left, right, .. } => get(left, &self.may) && get(right, &self.may),
                Op::Axis { from, domain, .. } => get(from, &self.may) && get(domain, &self.may),
                Op::Union { left, right } => get(left, &self.may) || get(right, &self.may),
                Op::Intersection { left, right } | Op::Join { left, right, .. } => get(left, &self.may) && get(right, &self.may),
                Op::Difference { left, .. } | Op::Antijoin { left, .. } => get(left, &self.may),
            };
            self.may.push(m);
        }
        self.may[rule.root]
    }

    fn census(&mut self) {
        let program = self.program;
        for c in &program.census {
            let mut may: Vec<bool> = Vec::with_capacity(c.steps.len());
            for (class, inputs, anchors) in &c.steps {
                let ins = inputs.iter().map(|&p| may[p]);
                let mut m = match class {
                    0 => ins.clone().any(|x| x) || inputs.is_empty(),
                    1 => ins.clone().all(|x| x),
                    2 => inputs.first().is_none_or(|&p| may[p]),
                    3 => false,
                    _ => true,
                };
                if let Some(a) = anchors {
                    m = m && self.anchor_present(a);
                }
                may.push(m);
            }
            self.census_rules += 1;
            if may[c.root] {
                self.census_started += 1;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn file(
        &mut self,
        index: usize,
        path: &str,
        source: &str,
        parsed: &ruff_python_parser::Parsed<ruff_python_ast::Mod>,
        table: &Table<'_>,
        _sema: &Sema,
        totals: &Totals,
        out: &mut Vec<String>,
    ) {
        if self.program.nav.is_some() {
            return self.file_nav(index, path, source, parsed, table, out);
        }
        self.files += 1;
        self.sym.reset_file();
        self.kinds_present(table);
        if self.census_on {
            self.census();
        }
        let program = self.program;
        let runnable: Vec<usize> = (0..program.rules.len()).filter(|&r| self.rule_may_run(&program.rules[r])).collect();
        if runnable.is_empty() {
            return;
        }
        let t0 = Instant::now();
        let mut view = std::mem::take(&mut self.view);
        view.build(table, parsed, source, &mut self.sym);
        self.views += 1;
        let t1 = Instant::now();
        self.view_ns += (t1 - t0).as_nanos() as u64;
        let ctx = Ctx { view: &view, table, source, file: 0, path };
        self.memo.clear();
        self.scans.clear();
        for r in runnable {
            let rule = &program.rules[r];
            self.started += 1;
            self.cur = r;
            let emit = self.demand(rule.root, &ctx);
            if emit.arity() == 0 {
                continue;
            }
            let span_col = (0..emit.arity()).find(|&c| emit.kinds[c] == K_SPAN);
            let mut seen: FxMap<K, ()> = FxMap::default();
            for row in emit.rows() {
                let key: K = rule.emit_key.iter().map(|&k| row[k]).collect();
                if seen.insert(key, ()).is_some() {
                    continue;
                }
                if let Some(sc) = span_col {
                    let s = row[sc];
                    out.push(format!("{}\t{}\t{}\t{}", rule.id, path, s >> 32, s & 0xffff_ffff));
                }
            }
        }
        self.memo.clear();
        self.scans.clear();
        self.eval_ns += t1.elapsed().as_nanos() as u64;
        self.view = view;
        let _ = totals;
    }

    pub(crate) fn finish(&mut self, totals: &Totals) {
        totals.extra[0].fetch_add(self.started, Ordering::Relaxed);
        totals.extra[1].fetch_add(self.census_started, Ordering::Relaxed);
        totals.extra[2].fetch_add(self.census_rules, Ordering::Relaxed);
        totals.extra[3].fetch_add(self.anchor_rows, Ordering::Relaxed);
        totals.extra[4].fetch_add(self.view_ns, Ordering::Relaxed);
        totals.extra[5].fetch_add(self.eval_ns, Ordering::Relaxed);
        totals.extra[6].fetch_add(self.views, Ordering::Relaxed);
        totals.extra[7].fetch_add(self.files * self.program.rules.len() as u64, Ordering::Relaxed);
        if self.program.nav.is_some() {
            let c = &self.nc;
            eprintln!(
                "{{\"nav\":{{\"files\":{},\"view_ns\":{},\"kind_index_ns\":{},\"nav_ns\":{},\"fact_ns\":{},\"provider_build_ns\":{},\"tail_ns\":{},\"tail_files\":{},\"steps\":{},\"candidates\":{},\"starts\":{},\"emitted\":{},\"subs\":{},\"made_tables\":{},\"made_rows\":{},\"fact_files\":{},\"fact_rows\":{},\"model_lex_ns\":{},\"model_flow_ns\":{},\"model_rows_ns\":{},\"model_files\":{},\"flow_files\":{},\"provider_files\":{},\"derived_ns\":{},\"derived_files\":{},\"visits\":{},\"made_group_ns\":{},\"index_ns\":{},\"index_builds\":{},\"keyed_calls\":{},\"keyed_misses\":{},\"keyed_hits\":{},\"table_builds\":{},\"table_fails\":{},\"need_asked\":{},\"need_failed\":{},\"per_rule\":{{{}}}}}}}",
                self.files, self.view_ns, c.kind_index_ns, self.nav_ns, c.fact_ns, c.provider_build_ns, self.tail_ns, self.tail_files, c.steps,
                c.candidates, c.starts, c.emitted, c.subs, c.made_tables, c.made_rows, c.fact_files, c.fact_rows, c.model_lex_ns, c.model_flow_ns, c.model_rows_ns, c.model_files, c.flow_files, c.provider_files, c.derived_ns, c.derived_files, c.visits, c.made_group_ns, c.index_ns, c.index_builds, c.keyed_calls, c.keyed_misses, c.keyed_hits, c.table_builds, c.table_fails, c.need_asked, c.need_failed,
                self.program.nav.as_ref().unwrap().rules.iter().enumerate().filter(|(i, _)| *i < c.rule_steps.len())
                    .map(|(i, r)| format!("\"{}\":[{},{},{}]", r.id, c.rule_starts[i], c.rule_visits[i], c.rule_steps[i]))
                    .collect::<Vec<_>>().join(",")
            );
            if c.prof {
                // bind (diagnostic): keyed calls per shared entry, and made-table builds
                eprintln!(
                    "{{\"navkprof\":[{}],\"navmade\":[{}]}}",
                    c.kprof.iter().enumerate().filter(|(_, v)| v.iter().any(|x| *x > 0))
                        .map(|(i, v)| format!("[{},{}]", i, v.iter().map(u64::to_string).collect::<Vec<_>>().join(",")))
                        .collect::<Vec<_>>().join(","),
                    c.made_prof.iter().enumerate().filter(|(_, v)| v[0] > 0)
                        .map(|(i, v)| format!("[\"{}\",{},{},{}]", self.program.nav.as_ref().unwrap().facts[i].name, v[0], v[1], v[2]))
                        .collect::<Vec<_>>().join(",")
                );
                // cut (diagnostic): shared entries that ran, as [index, calls, inclusive steps, own steps]
                eprintln!(
                    "{{\"navprof\":[{}]}}",
                    (0..c.shared_calls.len()).filter(|&i| c.shared_calls[i] > 0)
                        .map(|i| format!("[{},{},{},{}]", i, c.shared_calls[i], c.shared_incl[i], c.shared_self[i]))
                        .collect::<Vec<_>>().join(",")
                );
            }
        }
        let ops: Vec<String> = OP_NAMES
            .iter()
            .enumerate()
            .map(|(i, n)| format!("\"{n}\":[{},{}]", self.op_ns[i], self.op_rows[i]))
            .collect();
        let mut top: Vec<(u32, (u64, u64))> = self.node_ns.iter().map(|(&k, &v)| (k, v)).collect();
        top.sort_by(|a, b| b.1.0.cmp(&a.1.0));
        let top: Vec<String> = top.iter().take(40).map(|(k, (ns, rows))| format!("[{k},{ns},{rows}]")).collect();
        eprintln!(
            "{{\"worker_ops\":{{{}}},\"evaluated\":{},\"probed\":{},\"top_nodes\":[{}]}}",
            ops.join(","),
            self.evaluated,
            self.probed,
            top.join(",")
        );
    }

    fn input(&mut self, x: &Input, ctx: &Ctx<'_>, comps: &[Comparison]) -> std::rc::Rc<Rel> {
        match x {
            Input::Step(p) => self.demand(*p, ctx),
            Input::Rel(r) => {
                if comps.is_empty() {
                    if let Some(hit) = self.scans.get(&(*r as u8)) {
                        return hit.clone();
                    }
                    let rel = std::rc::Rc::new(self.scan(*r, ctx, comps));
                    self.scans.insert(*r as u8, rel.clone());
                    return rel;
                }
                std::rc::Rc::new(self.scan(*r, ctx, comps))
            }
        }
    }

    /// A step's rows, evaluated on first demand and memoised per file by the merged program's node id (shared across rules).
    fn demand(&mut self, pos: usize, ctx: &Ctx<'_>) -> std::rc::Rc<Rel> {
        let program = self.program;
        let rule = &program.rules[self.cur];
        let node = rule.nodes[pos];
        if let Some(hit) = self.memo.get(&node) {
            return hit.clone();
        }
        self.evaluated += 1;
        let outer = std::mem::take(&mut self.inner_ns);
        let t = Instant::now();
        let rel = self.eval(&rule.steps[pos], ctx);
        let op = op_index(&rule.steps[pos]);
        // exclusive time: the steps demanded inside this one are charged to themselves
        let spent = t.elapsed().as_nanos() as u64;
        let own = spent.saturating_sub(self.inner_ns);
        self.op_ns[op] += own;
        let e = self.node_ns.entry(node).or_insert((0, 0));
        e.0 += own;
        e.1 += rel.len() as u64;
        self.inner_ns = outer + spent;
        self.op_rows[op] += rel.len() as u64;
        let rel = std::rc::Rc::new(rel);
        self.memo.insert(node, rel.clone());
        rel
    }

    fn rel_row(&mut self, rel: RelId, id: u32, ctx: &Ctx<'_>, buf: &mut Vec<u64>) -> bool {
        let v = ctx.view;
        let i = id as usize;
        buf.clear();
        match rel {
            RelId::Ss | RelId::St => {
                if i >= v.len() {
                    return false;
                }
                buf.extend([id as u64, v.kind[i] as u64, v.text[i] as u64, ctx.file, span(v.start[i], v.stop[i])]);
            }
            RelId::Sc => {
                if i >= v.len() || v.parent[i] == u32::MAX {
                    return false;
                }
                buf.extend([v.parent[i] as u64, id as u64, v.ordinal[i] as u64, v.field[i] as u64, ctx.file]);
            }
            RelId::Sl => {
                if i >= v.len() {
                    return false;
                }
                let (s, e) = (v.start[i], v.stop[i]);
                buf.extend([
                    id as u64,
                    ctx.file,
                    span(s, e),
                    v.line_of(s) as u64,
                    v.line_of(e) as u64,
                    s as u64,
                    e as u64,
                    v.column_of(s) as u64,
                    v.column_of(e) as u64,
                ]);
            }
            RelId::Id => {
                if i >= v.n_nodes {
                    return false;
                }
                let text = match ctx.table.node[i] {
                    AnyNodeRef::Identifier(x) => x.as_str(),
                    AnyNodeRef::ExprName(x) => x.id.as_str(),
                    _ => return false,
                };
                let t = self.sym.text(text);
                buf.extend([id as u64, t as u64, ctx.file, span(v.start[i], v.stop[i])]);
            }
            RelId::Cm => {
                if i >= v.len() || self.sym.str(v.kind[i]) != Some("comment") {
                    return false;
                }
                let full = &ctx.source[v.start[i] as usize..v.stop[i] as usize];
                let t = self.sym.text(full.strip_prefix('#').unwrap_or(full));
                buf.extend([id as u64, t as u64, ctx.file, span(v.start[i], v.stop[i]), 0]);
            }
            RelId::Ir => {
                let Some(&r) = v.region_of.get(&id) else { return false };
                let [rid, parent, ind, s, e] = v.regions[r as usize];
                buf.extend([rid as u64, parent as u64, ind as u64, s as u64, e as u64, ctx.file, span(s, e)]);
            }
            RelId::Ee => return false,
        }
        true
    }

    fn scan<C: std::borrow::Borrow<Comparison>>(&mut self, rel: RelId, ctx: &Ctx<'_>, comps: &[C]) -> Rel {
        let mut out = Rel::new(rel_kinds(rel));
        let v = ctx.view;
        let mut buf = Vec::new();
        // access path: a semantic-syntax kind literal reads the kind's ids only
        let mut ids: Option<Vec<u32>> = None;
        if rel == RelId::Ir {
            ids = Some(v.regions.iter().map(|r| r[0]).collect());
        }
        if rel == RelId::Cm {
            let comment = self.sym.get_base("comment").unwrap_or(NULL);
            ids = Some((0..v.len() as u32).filter(|&i| v.kind[i as usize] == comment).collect());
        }
        if rel == RelId::Ss || rel == RelId::St {
            for c in comps.iter().map(std::borrow::Borrow::borrow) {
                if c.test == Test::Eq {
                    if let (Operand::Column(1), Operand::Text(t)) = (&c.left, &c.right) {
                        ids = Some((0..v.len() as u32).filter(|&i| v.kind[i as usize] == *t).collect());
                    }
                }
            }
        }
        if rel == RelId::Sc {
            for c in comps.iter().map(std::borrow::Borrow::borrow) {
                if c.test == Test::Eq {
                    if let (Operand::Column(3), Operand::Text(t)) = (&c.left, &c.right) {
                        ids = Some((0..v.len() as u32).filter(|&i| v.field[i as usize] == *t).collect());
                    }
                }
            }
        }
        let all: Box<dyn Iterator<Item = u32>> = match ids {
            Some(list) => Box::new(list.into_iter()),
            None => Box::new(0..v.len() as u32),
        };
        for id in all {
            if self.rel_row(rel, id, ctx, &mut buf) && self.holds(comps, &buf, &out.kinds, ctx) {
                out.data.extend_from_slice(&buf);
            }
        }
        self.anchor_rows += out.len() as u64;
        out
    }

    fn value_text<'a>(&'a self, v: u64, kind: u8, ctx: &'a Ctx<'_>) -> Option<&'a str> {
        match kind {
            K_SYM => self.sym.str(v as u32),
            K_FILE => Some(ctx.path),
            _ => None,
        }
    }

    fn holds<C: std::borrow::Borrow<Comparison>>(&self, comps: &[C], row: &[u64], kinds: &[u8], ctx: &Ctx<'_>) -> bool {
        for c in comps.iter().map(std::borrow::Borrow::borrow) {
            let (lv, lk) = match &c.left {
                Operand::Column(i) => (row[*i], kinds[*i]),
                Operand::Number(n) => (*n as u64, K_NUM),
                Operand::Text(t) => (*t as u64, K_SYM),
                _ => return false,
            };
            let ok = match c.test {
                Test::Eq | Test::Ne | Test::Lt | Test::Le | Test::Gt | Test::Ge => {
                    let (rv, rk) = match &c.right {
                        Operand::Column(i) => (row[*i], kinds[*i]),
                        Operand::Number(n) => (*n as u64, K_NUM),
                        Operand::Text(t) => (*t as u64, K_SYM),
                        _ => return false,
                    };
                    let ord = if lk == K_SYM && rk == K_SYM {
                        if lv == rv {
                            std::cmp::Ordering::Equal
                        } else {
                            match (self.sym.str(lv as u32), self.sym.str(rv as u32)) {
                                (Some(a), Some(b)) => a.cmp(b),
                                _ => lv.cmp(&rv),
                            }
                        }
                    } else if lk == K_NUM || rk == K_NUM {
                        (lv as i64).cmp(&(rv as i64))
                    } else {
                        lv.cmp(&rv)
                    };
                    match c.test {
                        Test::Eq => ord.is_eq(),
                        Test::Ne => !ord.is_eq(),
                        Test::Lt => ord.is_lt(),
                        Test::Le => ord.is_le(),
                        Test::Gt => ord.is_gt(),
                        _ => ord.is_ge(),
                    }
                }
                Test::In | Test::NotIn => {
                    let inside = match &c.right {
                        Operand::Texts(ts) => ts.contains(&(lv as u32)),
                        Operand::Numbers(ns) => ns.contains(&(lv as i64)),
                        _ => false,
                    };
                    inside == (c.test == Test::In)
                }
                Test::Glob | Test::NotGlob | Test::Matches => {
                    let hit = self.value_text(lv, lk, ctx).is_some_and(|t| c.regex.iter().any(|r| r.is_match(t)));
                    if c.test == Test::NotGlob { !hit } else { hit }
                }
                Test::Starts | Test::Ends => {
                    let rt = match &c.right {
                        Operand::Text(t) => self.sym.str(*t),
                        Operand::Column(i) => self.value_text(row[*i], kinds[*i], ctx),
                        _ => None,
                    };
                    match (self.value_text(lv, lk, ctx), rt) {
                        (Some(a), Some(b)) => {
                            if c.test == Test::Starts { a.starts_with(b) } else { a.ends_with(b) }
                        }
                        _ => false,
                    }
                }
            };
            if !ok {
                return false;
            }
        }
        true
    }

    fn eval(&mut self, op: &Op, ctx: &Ctx<'_>) -> Rel {
        match op {
            Op::Injected { kinds } => Rel::new(kinds.clone()),
            Op::Select { from, comps, kept } => {
                if let Input::Rel(r) = from {
                    let all = self.scan(*r, ctx, comps);
                    return keep(&all, kept);
                }
                let input = self.input(from, ctx, &[]);
                let mut out = Rel::new(input.kinds.clone());
                for row in input.rows() {
                    if input.arity() > 0 && self.holds(comps, row, &input.kinds, ctx) {
                        out.data.extend_from_slice(row);
                    }
                }
                keep(&out, kept)
            }
            Op::Project { from, columns, distinct } => {
                let input = self.input(from, ctx, &[]);
                let mut out = Rel::new(columns.iter().map(|&c| input.kinds[c]).collect());
                let mut seen: FxMap<K, ()> = FxMap::default();
                for row in input.rows() {
                    if input.arity() == 0 {
                        break;
                    }
                    if *distinct && seen.insert(columns.iter().map(|&c| row[c]).collect::<K>(), ()).is_some() {
                        continue;
                    }
                    out.data.extend(columns.iter().map(|&c| row[c]));
                }
                out
            }
            Op::Union { left, right } => {
                let l = self.input(left, ctx, &[]);
                let r = self.input(right, ctx, &[]);
                let kinds = if l.kinds.is_empty() { r.kinds.clone() } else { l.kinds.clone() };
                let mut out = Rel::new(kinds);
                let mut seen: FxMap<K, ()> = FxMap::default();
                for rel in [&l, &r] {
                    for row in rel.rows() {
                        if rel.arity() > 0 && seen.insert(row.iter().copied().collect::<K>(), ()).is_none() {
                            out.data.extend_from_slice(row);
                        }
                    }
                }
                out
            }
            Op::Intersection { left, right } | Op::Difference { left, right } => {
                let want = matches!(op, Op::Intersection { .. });
                let l = self.input(left, ctx, &[]);
                if l.len() == 0 {
                    return Rel::new(l.kinds.clone());
                }
                let r = self.input(right, ctx, &[]);
                let rs: FxMap<K, ()> = r.rows().filter(|_| r.arity() > 0).map(|x| (x.iter().copied().collect::<K>(), ())).collect();
                let mut out = Rel::new(l.kinds.clone());
                let mut seen: FxMap<K, ()> = FxMap::default();
                for row in l.rows() {
                    let k: K = row.iter().copied().collect();
                    if l.arity() > 0 && rs.contains_key(&k) == want && seen.insert(k, ()).is_none() {
                        out.data.extend_from_slice(row);
                    }
                }
                out
            }
            Op::Antijoin { left, right, left_on, right_on } => {
                let l = self.input(left, ctx, &[]);
                if l.len() == 0 {
                    return Rel::new(l.kinds.clone());
                }
                if !left_on.is_empty() {
                    if let Some((rel, comps, map)) = self.probe_through(right, right_on) {
                        // keep the left rows the probe finds nothing for
                        let ron: Vec<usize> = right_on.iter().map(|&c| map[c]).collect();
                        let marks: Vec<(bool, usize)> = (0..l.arity()).map(|c| (false, c)).collect();
                        let lmark = std::rc::Rc::new(self.mark_rows(&l));
                        let mut on = left_on.clone();
                        on.push(l.arity());
                        let _ = on;
                        let hit = self.join_rel(&lmark, rel, left_on, &ron, &[(false, l.arity())], &comps, ctx);
                        let found: FxMap<u64, ()> = hit.data.iter().map(|&i| (i, ())).collect();
                        let mut out = Rel::new(l.kinds.clone());
                        for (i, row) in l.rows().enumerate() {
                            if !found.contains_key(&(i as u64)) {
                                out.data.extend_from_slice(row);
                            }
                        }
                        let _ = marks;
                        self.probed += 1;
                        return out;
                    }
                }
                let r = self.input(right, ctx, &[]);
                let mut out = Rel::new(l.kinds.clone());
                if left_on.is_empty() {
                    if r.len() == 0 {
                        out.data = l.data.clone();
                    }
                    return out;
                }
                let keys: FxMap<K, ()> =
                    r.rows().filter(|_| r.arity() > 0).map(|x| (right_on.iter().map(|&c| x[c]).collect(), ())).collect();
                for row in l.rows() {
                    if l.arity() == 0 {
                        break;
                    }
                    let k: K = left_on.iter().map(|&c| row[c]).collect();
                    if !keys.contains_key(&k) {
                        out.data.extend_from_slice(row);
                    }
                }
                out
            }
            Op::Join { left, right, left_on, right_on, output, filter } => {
                // the left side is a relation (or a select/project chain over one) keyed on its id and the right side is a
                // step: evaluate the step, drive the loop from its rows and probe the relation (never materialise it)
                if filter.is_empty() && matches!(right, Input::Step(_)) && !left_on.is_empty() {
                    if let Some((rel, comps, map)) = self.probe_through(left, left_on) {
                        let r = self.input(right, ctx, &[]);
                        if r.len() == 0 {
                            return Rel::new(vec![K_NODE; output.len()]);
                        }
                        let lon: Vec<usize> = left_on.iter().map(|&c| map[c]).collect();
                        let swapped: Vec<(bool, usize)> =
                            output.iter().map(|&(side, c)| if side { (false, c) } else { (true, map[c]) }).collect();
                        self.probed += 1;
                        return self.join_rel(&r, rel, right_on, &lon, &swapped, &comps, ctx);
                    }
                }
                let l = self.input(left, ctx, &[]);
                if l.len() == 0 {
                    return Rel::new(vec![K_NODE; output.len()]);
                }
                if let Input::Rel(rel) = right {
                    return self.join_rel(&l, *rel, left_on, right_on, output, filter, ctx);
                }
                // a not-yet-evaluated select over a relation, joined on that relation's id: probe it, never materialise it
                if filter.is_empty() {
                    if let Some((rel, comps, map)) = self.probe_through(right, right_on) {
                        let ron: Vec<usize> = right_on.iter().map(|&c| map[c]).collect();
                        let out_m: Vec<(bool, usize)> = output.iter().map(|&(side, c)| (side, if side { map[c] } else { c })).collect();
                        self.probed += 1;
                        return self.join_rel(&l, rel, left_on, &ron, &out_m, &comps, ctx);
                    }
                }
                let r = self.input(right, ctx, &[]);
                let mut kinds = Vec::new();
                for &(side, c) in output {
                    kinds.push(if side { r.kinds[c] } else { l.kinds[c] });
                }
                let mut out = Rel::new(kinds);
                let mut index: FxMap<K, Vec<usize>> = FxMap::default();
                for i in 0..r.len() {
                    let row = r.row(i);
                    if self.holds(filter, row, &r.kinds, ctx) {
                        index.entry(right_on.iter().map(|&c| row[c]).collect()).or_default().push(i);
                    }
                }
                for lrow in l.rows() {
                    if l.arity() == 0 {
                        break;
                    }
                    let k: K = left_on.iter().map(|&c| lrow[c]).collect();
                    if let Some(hits) = index.get(&k) {
                        for &i in hits {
                            let rrow = r.row(i);
                            for &(side, c) in output {
                                out.data.push(if side { rrow[c] } else { lrow[c] });
                            }
                        }
                    }
                }
                out
            }
            Op::Extend { from, values } => {
                let input = self.input(from, ctx, &[]);
                let mut kinds = input.kinds.clone();
                for v in values {
                    kinds.push(match v {
                        Operand::Text(_) => K_SYM,
                        _ => K_NUM,
                    });
                }
                let mut out = Rel::new(kinds);
                for row in input.rows() {
                    if input.arity() == 0 {
                        break;
                    }
                    out.data.extend_from_slice(row);
                    for v in values {
                        out.data.push(match v {
                            Operand::Number(n) => *n as u64,
                            Operand::Text(t) => *t as u64,
                            _ => 0,
                        });
                    }
                }
                out
            }
            Op::Axis { from, domain, direction, id, domain_id, filter, output } => {
                let input = self.input(from, ctx, &[]);
                let (dom, dom_is_rel) = match domain {
                    Input::Rel(r) => (std::rc::Rc::new(Rel::new(rel_kinds(*r))), Some(*r)),
                    Input::Step(_) => (self.input(domain, ctx, &[]), None),
                };
                let mut index: FxMap<u64, Vec<usize>> = FxMap::default();
                if dom_is_rel.is_none() {
                    for i in 0..dom.len() {
                        let row = dom.row(i);
                        if self.holds(filter, row, &dom.kinds, ctx) {
                            index.entry(row[*domain_id]).or_default().push(i);
                        }
                    }
                }
                let mut kinds = Vec::new();
                if output.is_empty() {
                    kinds.extend(input.kinds.iter().copied());
                    kinds.push(K_NODE);
                } else {
                    for &(side, c) in output {
                        kinds.push(if side { dom.kinds[c] } else { input.kinds[c] });
                    }
                }
                let mut out = Rel::new(kinds);
                let v = ctx.view;
                let mut reached: Vec<u32> = Vec::new();
                let mut stack: Vec<u32> = Vec::new();
                let mut buf = Vec::new();
                for row in input.rows() {
                    if input.arity() == 0 {
                        break;
                    }
                    let e = row[*id] as u32;
                    reached.clear();
                    match direction {
                        Dir::Parent => {
                            if (e as usize) < v.len() && v.parent[e as usize] != u32::MAX {
                                reached.push(v.parent[e as usize]);
                            }
                        }
                        Dir::Child => {
                            if (e as usize) < v.len() {
                                reached.extend_from_slice(v.children_of(e));
                            }
                        }
                        Dir::Ancestor | Dir::Nearest => {
                            let mut p = if (e as usize) < v.len() { v.parent[e as usize] } else { u32::MAX };
                            while p != u32::MAX {
                                reached.push(p);
                                p = v.parent[p as usize];
                            }
                        }
                        Dir::Descendant => {
                            stack.clear();
                            if (e as usize) < v.len() {
                                stack.extend(v.children_of(e).iter().rev());
                            }
                            while let Some(x) = stack.pop() {
                                reached.push(x);
                                stack.extend(v.children_of(x).iter().rev());
                            }
                        }
                    }
                    for &x in &reached {
                        let mut admitted: Vec<Vec<u64>> = Vec::new();
                        if let Some(r) = dom_is_rel {
                            if self.rel_row(r, x, ctx, &mut buf) && buf[*domain_id] == x as u64 && self.holds(filter, &buf, &dom.kinds, ctx) {
                                admitted.push(buf.clone());
                            }
                        } else if let Some(hits) = index.get(&(x as u64)) {
                            for &i in hits {
                                admitted.push(dom.row(i).to_vec());
                            }
                        }
                        if admitted.is_empty() {
                            continue;
                        }
                        if output.is_empty() {
                            out.data.extend_from_slice(row);
                            out.data.push(x as u64);
                        } else {
                            for drow in &admitted {
                                for &(side, c) in output {
                                    out.data.push(if side { drow[c] } else { row[c] });
                                }
                            }
                        }
                        if *direction == Dir::Nearest {
                            break;
                        }
                    }
                }
                out
            }
            Op::SourceMatches { from, span: sc, base_line, regex, capture, capture_text, capture_line } => {
                let input = self.input(from, ctx, &[]);
                let mut kinds = input.kinds.clone();
                kinds.push(K_SPAN);
                if base_line.is_some() {
                    kinds.push(K_NUM);
                }
                if capture.is_some() {
                    kinds.push(K_SPAN);
                    if *capture_text {
                        kinds.push(K_SYM);
                    }
                    if *capture_line {
                        kinds.push(K_NUM);
                    }
                }
                let mut out = Rel::new(kinds);
                let bytes = ctx.source.as_bytes();
                let group = capture.as_ref().and_then(|name| regex.group_info().to_index(regex_automata::PatternID::ZERO, name));
                let rows: Vec<Vec<u64>> = input.rows().filter(|_| input.arity() > 0).map(<[u64]>::to_vec).collect();
                for row in rows {
                    let s = (row[*sc] >> 32) as usize;
                    let e = (row[*sc] & 0xffff_ffff) as usize;
                    if e > bytes.len() || s > e {
                        continue;
                    }
                    let hay = &bytes[s..e];
                    let line_at = |off: usize| -> u64 {
                        let base = base_line.map_or(0, |b| row[b]);
                        base + bytes[s..off].iter().filter(|&&b| b == b'\n').count() as u64
                    };
                    let mut caps = regex.create_captures();
                    let mut at = 0usize;
                    while at <= hay.len() {
                        let input_r = regex_automata::Input::new(hay).range(at..);
                        regex.search_captures(&input_r, &mut caps);
                        let Some(m) = caps.get_match() else { break };
                        let (ms, me) = (s + m.start(), s + m.end());
                        let mut r = row.clone();
                        r.push(span(ms as u32, me as u32));
                        if base_line.is_some() {
                            r.push(line_at(ms));
                        }
                        let mut keep_row = true;
                        if let Some(g) = group {
                            match caps.get_group(g) {
                                Some(cg) => {
                                    let (cs, ce) = (s + cg.start, s + cg.end);
                                    r.push(span(cs as u32, ce as u32));
                                    if *capture_text {
                                        let t = String::from_utf8_lossy(&bytes[cs..ce]).to_string();
                                        r.push(self.sym.minted(t) as u64);
                                    }
                                    if *capture_line {
                                        r.push(line_at(cs));
                                    }
                                }
                                None => keep_row = false,
                            }
                        }
                        if keep_row {
                            out.data.extend_from_slice(&r);
                        }
                        at = if m.end() > m.start() { m.end() } else { m.end() + 1 };
                    }
                }
                out
            }
            Op::SpanBounds { from, span: sc } => {
                let input = self.input(from, ctx, &[]);
                let mut kinds = input.kinds.clone();
                kinds.extend([K_NUM, K_NUM]);
                let mut out = Rel::new(kinds);
                for row in input.rows() {
                    if input.arity() == 0 {
                        break;
                    }
                    out.data.extend_from_slice(row);
                    out.data.push(row[*sc] >> 32);
                    out.data.push(row[*sc] & 0xffff_ffff);
                }
                out
            }
            Op::SpanBetween { from, start, end } => {
                let input = self.input(from, ctx, &[]);
                let mut kinds = input.kinds.clone();
                kinds.push(K_SPAN);
                let mut out = Rel::new(kinds);
                for row in input.rows() {
                    if input.arity() == 0 {
                        break;
                    }
                    out.data.extend_from_slice(row);
                    out.data.push((row[*start] & !0xffff_ffff) | (row[*end] & 0xffff_ffff));
                }
                out
            }
            Op::Closure { edges, keys, source, destination } => {
                let input = self.input(edges, ctx, &[]);
                let mut kinds: Vec<u8> = keys.iter().map(|&k| input.kinds[k]).collect();
                if input.arity() == 0 {
                    return Rel::new(vec![K_NODE; keys.len() + 2]);
                }
                kinds.push(input.kinds[*source]);
                kinds.push(input.kinds[*destination]);
                let mut adj: FxMap<(Vec<u64>, u64), Vec<u64>> = FxMap::default();
                let mut starts: Vec<(Vec<u64>, u64)> = Vec::new();
                for row in input.rows() {
                    let k: Vec<u64> = keys.iter().map(|&c| row[c]).collect();
                    let e = adj.entry((k.clone(), row[*source])).or_default();
                    if e.is_empty() {
                        starts.push((k, row[*source]));
                    }
                    e.push(row[*destination]);
                }
                let mut out = Rel::new(kinds);
                for (k, a) in starts {
                    let mut seen: FxMap<u64, ()> = FxMap::default();
                    let mut stack: Vec<u64> = adj.get(&(k.clone(), a)).cloned().unwrap_or_default();
                    while let Some(b) = stack.pop() {
                        if seen.insert(b, ()).is_some() {
                            continue;
                        }
                        out.data.extend_from_slice(&k);
                        out.data.push(a);
                        out.data.push(b);
                        if let Some(next) = adj.get(&(k.clone(), b)) {
                            stack.extend(next.iter().copied());
                        }
                    }
                }
                out
            }
            Op::Capture { from, path, pattern } => {
                let input = self.input(from, ctx, &[]);
                let slots = pattern.bytes().filter(|&b| b == b'{').count();
                let mut kinds = input.kinds.clone();
                kinds.extend(std::iter::repeat_n(K_SYM, slots));
                let mut out = Rel::new(kinds);
                for row in input.rows() {
                    if input.arity() == 0 {
                        break;
                    }
                    let Some(text) = self.value_text(row[*path], input.kinds[*path], ctx).map(str::to_owned) else { continue };
                    let mut found: Vec<Vec<std::ops::Range<usize>>> = Vec::new();
                    capture_matches(pattern, &text, &text, &mut Vec::new(), &mut found);
                    for spans in found {
                        out.data.extend_from_slice(row);
                        for r in spans {
                            let id = self.sym.minted(text[r].to_owned());
                            out.data.push(id as u64);
                        }
                    }
                }
                out
            }
            Op::Replace { from, text, regex, replacement } => {
                let input = self.input(from, ctx, &[]);
                let mut kinds = input.kinds.clone();
                kinds.push(K_SYM);
                let mut out = Rel::new(kinds);
                for row in input.rows() {
                    if input.arity() == 0 {
                        break;
                    }
                    let Some(t) = self.value_text(row[*text], input.kinds[*text], ctx).map(str::to_owned) else {
                        out.data.extend_from_slice(row);
                        out.data.push(NULL as u64);
                        continue;
                    };
                    let mut result = String::new();
                    let mut last = 0;
                    for m in regex.find_iter(t.as_bytes()) {
                        result.push_str(&t[last..m.start()]);
                        result.push_str(replacement);
                        last = m.end();
                    }
                    result.push_str(&t[last..]);
                    let id = self.sym.minted(result);
                    out.data.extend_from_slice(row);
                    out.data.push(id as u64);
                }
                out
            }
            Op::OrderedMatch { left, right, left_on, right_on, left_order, right_order, output } => {
                let l = self.input(left, ctx, &[]);
                if l.len() == 0 {
                    return Rel::new(vec![K_NODE; output.len()]);
                }
                let r = self.input(right, ctx, &[]);
                let mut kinds = Vec::new();
                for &(side, c) in output {
                    kinds.push(if side { r.kinds.get(c).copied().unwrap_or(K_NODE) } else { l.kinds[c] });
                }
                let mut out = Rel::new(kinds);
                if r.len() == 0 {
                    return out;
                }
                let mut groups: FxMap<Vec<u64>, Vec<usize>> = FxMap::default();
                for i in 0..r.len() {
                    groups.entry(right_on.iter().map(|&c| r.row(i)[c]).collect()).or_default().push(i);
                }
                for g in groups.values_mut() {
                    g.sort_by_key(|&i| r.row(i)[*right_order] as i64);
                }
                let mut lefts: Vec<usize> = (0..l.len()).collect();
                lefts.sort_by_key(|&i| l.row(i)[*left_order] as i64);
                let mut used: FxMap<usize, ()> = FxMap::default();
                for i in lefts {
                    let lrow = l.row(i);
                    let k: Vec<u64> = left_on.iter().map(|&c| lrow[c]).collect();
                    let Some(g) = groups.get(&k) else { continue };
                    let lo = lrow[*left_order] as i64;
                    if let Some(&j) = g.iter().find(|&&j| (r.row(j)[*right_order] as i64) > lo && !used.contains_key(&j)) {
                        used.insert(j, ());
                        let rrow = r.row(j);
                        for &(side, c) in output {
                            out.data.push(if side { rrow[c] } else { lrow[c] });
                        }
                    }
                }
                out
            }
            Op::GroupCount { from, keys, distinct } => {
                let input = self.input(from, ctx, &[]);
                let mut kinds: Vec<u8> = keys.iter().map(|&k| input.kinds[k]).collect();
                kinds.push(K_NUM);
                let mut groups: Vec<(Vec<u64>, Vec<u64>)> = Vec::new();
                let mut at: FxMap<Vec<u64>, usize> = FxMap::default();
                for row in input.rows() {
                    if input.arity() == 0 {
                        break;
                    }
                    let k: Vec<u64> = keys.iter().map(|&c| row[c]).collect();
                    let slot = *at.entry(k.clone()).or_insert_with(|| {
                        groups.push((k, Vec::new()));
                        groups.len() - 1
                    });
                    groups[slot].1.push(distinct.map_or(0, |d| row[d]));
                }
                let mut out = Rel::new(kinds);
                for (k, mut vals) in groups {
                    let n = if distinct.is_some() {
                        vals.sort_unstable();
                        vals.dedup();
                        vals.len()
                    } else {
                        vals.len()
                    };
                    out.data.extend_from_slice(&k);
                    out.data.push(n as u64);
                }
                out
            }
        }
    }

    /// When `x` is a select over a node relation that has not been evaluated in this file, and `right_on` (through the select's
    /// kept columns) lands on that relation's id (or semantic-child's parent or child): the relation, the select's comparisons and
    /// the kept-column map, so the caller probes instead of materialising the select.
    fn probe_through(&self, x: &Input, right_on: &[usize]) -> Option<(RelId, Vec<&'p Comparison>, Vec<usize>)> {
        if let Input::Rel(rel) = x {
            if matches!(rel, RelId::Ir | RelId::Cm | RelId::Ee) {
                return None;
            }
            let lands = right_on.iter().any(|&c| match rel {
                RelId::Sc => c == 0 || c == 1,
                _ => c == 0,
            });
            return lands.then(|| (*rel, Vec::new(), (0..rel_kinds(*rel).len()).collect()));
        }
        let Input::Step(p) = x else { return None };
        let program: &'p Program = self.program;
        let rule = &program.rules[self.cur];
        if self.memo.contains_key(&rule.nodes[*p]) {
            return None;
        }
        let (rel, comps, map) = self.chain(*p, 0)?;
        if matches!(rel, RelId::Ir | RelId::Cm | RelId::Ee) {
            return None;
        }
        let lands = right_on.iter().any(|&c| match rel {
            RelId::Sc => map.get(c).is_some_and(|&m| m == 0 || m == 1),
            _ => map.get(c) == Some(&0),
        });
        lands.then_some((rel, comps, map))
    }

    /// A chain of `select` / `project` steps over one relation, seen as (relation, every comparison on its raw columns,
    /// output column -> raw column). A distinct project is kept only when it keeps the id (rows stay unique per id).
    fn chain(&self, p: usize, depth: usize) -> Option<(RelId, Vec<&'p Comparison>, Vec<usize>)> {
        if depth > 8 {
            return None;
        }
        let program: &'p Program = self.program;
        let rule = &program.rules[self.cur];
        let base = |x: &Input| -> Option<(RelId, Vec<&'p Comparison>, Vec<usize>)> {
            match x {
                Input::Rel(r) => Some((*r, Vec::new(), (0..rel_kinds(*r).len()).collect())),
                Input::Step(q) => {
                    if self.memo.contains_key(&rule.nodes[*q]) {
                        None
                    } else {
                        self.chain(*q, depth + 1)
                    }
                }
            }
        };
        match &rule.steps[p] {
            Op::Select { from, comps, kept } => {
                let (r, mut all, map) = base(from)?;
                // comparisons of this select are on its input's columns: only a direct read of the relation keeps them raw
                if !matches!(from, Input::Rel(_)) && !comps.is_empty() {
                    let identity = map.iter().enumerate().all(|(i, &m)| i == m);
                    if !identity {
                        return None;
                    }
                }
                all.extend(comps.iter());
                let map = if kept.is_empty() { map } else { kept.iter().map(|&k| map[k]).collect() };
                Some((r, all, map))
            }
            Op::Project { from, columns, distinct } => {
                let (r, all, map) = base(from)?;
                let map: Vec<usize> = columns.iter().map(|&c| map[c]).collect();
                let keeps_id = match r {
                    RelId::Sc => map.contains(&0) && map.contains(&1),
                    _ => map.contains(&0),
                };
                if *distinct && !keeps_id {
                    return None;
                }
                Some((r, all, map))
            }
            _ => None,
        }
    }

    /// The left rows with their row number appended (to find which rows a probe matched).
    fn mark_rows(&self, l: &Rel) -> Rel {
        let mut kinds = l.kinds.clone();
        kinds.push(K_NUM);
        let mut out = Rel::new(kinds);
        for (i, row) in l.rows().enumerate() {
            out.data.extend_from_slice(row);
            out.data.push(i as u64);
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn join_rel<C: std::borrow::Borrow<Comparison>>(&mut self, l: &Rel, rel: RelId, left_on: &[usize], right_on: &[usize], output: &[(bool, usize)], filter: &[C], ctx: &Ctx<'_>) -> Rel {
        let rk = rel_kinds(rel);
        let mut kinds = Vec::new();
        for &(side, c) in output {
            kinds.push(if side { rk[c] } else { l.kinds[c] });
        }
        let mut out = Rel::new(kinds);
        // probe paths: the id column of a node relation, the parent or the child of semantic-child
        let id_key = match rel {
            RelId::Sc => right_on.iter().position(|&c| c == 0 || c == 1),
            _ => right_on.iter().position(|&c| c == 0),
        };
        let mut buf = Vec::new();
        if let Some(kpos) = id_key {
            let col = right_on[kpos];
            for lrow in l.rows() {
                if l.arity() == 0 {
                    break;
                }
                let key = lrow[left_on[kpos]] as u32;
                let mut probe = |this: &mut Self, id: u32, out: &mut Rel, buf: &mut Vec<u64>| {
                    if this.rel_row(rel, id, ctx, buf)
                        && left_on.iter().zip(right_on).all(|(&a, &b)| lrow[a] == buf[b])
                        && this.holds(filter, buf, &rk, ctx)
                    {
                        for &(side, c) in output {
                            out.data.push(if side { buf[c] } else { lrow[c] });
                        }
                    }
                };
                if rel == RelId::Sc && col == 0 {
                    if (key as usize) < ctx.view.len() {
                        let kids: Vec<u32> = ctx.view.children_of(key).to_vec();
                        for c in kids {
                            probe(self, c, &mut out, &mut buf);
                        }
                    }
                } else {
                    probe(self, key, &mut out, &mut buf);
                }
            }
            return out;
        }
        let r = self.scan(rel, ctx, filter);
        let mut index: FxMap<K, Vec<usize>> = FxMap::default();
        for i in 0..r.len() {
            index.entry(right_on.iter().map(|&c| r.row(i)[c]).collect()).or_default().push(i);
        }
        for lrow in l.rows() {
            if l.arity() == 0 {
                break;
            }
            let k: K = left_on.iter().map(|&c| lrow[c]).collect();
            if let Some(hits) = index.get(&k) {
                for &i in hits {
                    let rrow = r.row(i);
                    for &(side, c) in output {
                        out.data.push(if side { rrow[c] } else { lrow[c] });
                    }
                }
            }
        }
        out
    }
}

fn keep(rel: &Rel, kept: &[usize]) -> Rel {
    if kept.is_empty() {
        return rel.clone();
    }
    let mut out = Rel::new(kept.iter().map(|&c| rel.kinds[c]).collect());
    for row in rel.rows() {
        if rel.arity() == 0 {
            break;
        }
        out.data.extend(kept.iter().map(|&c| row[c]));
    }
    out
}

fn is_node_kind_name(name: &str) -> bool {
    NODE_KIND_NAMES.contains(&name)
}

const NODE_KIND_NAMES: &[&str] = &[
    "module", "expression", "function-def", "async-function-def", "class-def", "return", "delete", "type-alias", "assign",
    "aug-assign", "ann-assign", "for", "async-for", "while", "if", "with", "async-with", "match", "raise", "try", "assert", "import",
    "import-from", "global", "nonlocal", "expr", "pass", "break", "continue", "bool-op", "named-expr", "bin-op", "unary-op", "lambda",
    "if-exp", "dict", "set", "list-comp", "set-comp", "dict-comp", "generator-exp", "await", "yield", "yield-from", "compare", "call",
    "joined-str", "template-str", "constant", "attribute", "subscript", "starred", "name", "list", "tuple", "slice", "except-handler",
    "comprehension", "call-arguments", "arguments", "arg", "parameter-with-default", "keyword", "alias", "withitem", "match-case",
    "decorator", "elif-else-clause", "type-params", "f-string-part", "t-string-part", "string-part", "bytes-part", "identifier",
    "formatted-value",
];


/// RyoQL `capture`: every way the text matches the slot pattern (`{name}` one segment, no `/`; `{*name}` any text), each slot
/// non-empty; one result per match, the slots' byte ranges in order.
fn capture_matches(pattern: &str, whole: &str, tail: &str, slots: &mut Vec<std::ops::Range<usize>>, out: &mut Vec<Vec<std::ops::Range<usize>>>) {
    let Some(open) = pattern.find('{') else {
        if pattern == tail {
            out.push(slots.clone());
        }
        return;
    };
    let (literal, after_open) = pattern.split_at(open);
    let crosses = after_open.starts_with("{*");
    let Some(from) = tail.strip_prefix(literal) else { return };
    let rest = after_open.split_once('}').map(|(_, a)| a).unwrap_or_default();
    let start = whole.len() - from.len();
    if !rest.contains('{') {
        let Some(end) = from.len().checked_sub(rest.len()) else { return };
        if end >= 1 {
            if let (Some(slot), Some(after)) = (from.get(..end), from.get(end..)) {
                if after == rest && (crosses || !slot.contains('/')) {
                    slots.push(start..start + end);
                    out.push(slots.clone());
                    slots.pop();
                }
            }
        }
        return;
    }
    for end in 1..=from.len() {
        let (Some(slot), Some(after)) = (from.get(..end), from.get(end..)) else { continue };
        if !crosses && slot.contains('/') {
            continue;
        }
        slots.push(start..start + end);
        capture_matches(rest, whole, after, slots, out);
        slots.pop();
    }
}

/// A row or key of up to 10 cells without a heap allocation (longer ones spill to a Vec).
#[derive(Clone, PartialEq, Eq, Hash)]
enum K {
    S(u8, [u64; 10]),
    B(Vec<u64>),
}

impl FromIterator<u64> for K {
    fn from_iter<I: IntoIterator<Item = u64>>(iter: I) -> Self {
        let mut v = [0u64; 10];
        let mut n = 0usize;
        let mut spill: Option<Vec<u64>> = None;
        for x in iter {
            if let Some(s) = &mut spill {
                s.push(x);
            } else if n < 10 {
                v[n] = x;
                n += 1;
            } else {
                let mut s = v.to_vec();
                s.push(x);
                spill = Some(s);
            }
        }
        match spill {
            Some(s) => K::B(s),
            None => K::S(n as u8, v),
        }
    }
}

fn op_index(op: &Op) -> usize {
    match op {
        Op::Select { from: Input::Rel(_), .. } => 0,
        Op::Select { .. } => 1,
        Op::Project { .. } => 2,
        Op::Union { .. } => 3,
        Op::Intersection { .. } => 4,
        Op::Difference { .. } => 5,
        Op::Join { right: Input::Rel(_), .. } => 6,
        Op::Join { .. } => 7,
        Op::Antijoin { .. } => 8,
        Op::Extend { .. } => 9,
        Op::Axis { .. } => 10,
        Op::SourceMatches { .. } => 11,
        Op::SpanBounds { .. } => 12,
        Op::SpanBetween { .. } => 13,
        Op::GroupCount { .. } => 14,
        Op::Closure { .. } => 15,
        Op::Capture { .. } => 16,
        Op::Replace { .. } => 17,
        Op::OrderedMatch { .. } => 18,
        Op::Injected { .. } => 19,
    }
}

pub(crate) const OP_NAMES: [&str; 20] = [
    "select(relation)", "select(step)", "project", "union", "intersection", "difference", "join(relation)", "join(step)",
    "antijoin", "extend", "axis", "source_matches", "span_bounds", "span_between", "group_count", "closure", "capture",
    "replace", "ordered_match", "injected",
];
