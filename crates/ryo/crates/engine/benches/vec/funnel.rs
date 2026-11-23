//! THROWAWAY (ryo-fmx3.1, step 2): each rule's tree-local prefix (`build/vec/scripts/funnel.py`) evaluated over its anchor
//! bucket three ways that must give identical survivors: a generic interpreter (a match on the op per candidate per step),
//! operations specialised at load (one closure per operation and column type, run over a selection vector), and bit-mask
//! knock-out (per node a few tests ANDed into a mask of the plans of its bucket).

use super::{Key, Vocab};
use ryo::languages::{ElementSets, NodeTable, Position, node_table};
use std::collections::HashMap;

pub(super) type KSet = [u64; 4];
pub(super) type FSet = [u64; 2];

fn kbit(set: &KSet, k: u16) -> bool {
    set[usize::from(k) / 64] >> (k % 64) & 1 == 1
}
fn fbit(set: &FSet, f: u16) -> bool {
    usize::from(f) < 128 && set[usize::from(f) / 64] >> (f % 64) & 1 == 1
}

#[derive(Clone, Debug)]
pub(super) struct Check {
    v: usize,
    kinds: Option<KSet>,
    notkinds: Option<KSet>,
    texts: Option<Vec<Vec<u8>>>,
    fields: Option<FSet>,
    re: Option<regex::bytes::Regex>,
}

impl Check {
    fn kinds_only(&self) -> bool {
        self.texts.is_none() && self.fields.is_none() && self.re.is_none() && (self.kinds.is_some() || self.notkinds.is_some())
    }
    /// The kinds this check lets through, when it tests kinds only.
    fn kind_set(&self) -> KSet {
        let mut s = self.kinds.unwrap_or([u64::MAX; 4]);
        if let Some(n) = self.notkinds {
            for i in 0..4 {
                s[i] &= !n[i];
            }
        }
        s
    }
}

#[derive(Clone, Debug)]
pub(super) enum Op {
    Check(Check),
    Up { from: usize, to: usize, leaf: bool },
    Down { from: usize, to: usize, leaf: bool },
    Not(Vec<Op>),
    /// Some child of `from`, held in `to`, matches the sub-plan (a subtree's own block: no backtracking across branches).
    Exists { from: usize, to: usize, plan: Vec<Op> },
    IsParent { child: usize, parent: usize },
    /// A test on the provider's facts, read from the file's fact columns.
    Fact(super::facts::FactOp),
    /// `to` = the nearest strict ancestor of `from` whose kind is in enclosing set `set` (a structure column).
    Enc { from: usize, to: usize, set: usize },
    /// Some strict ancestor of `from`, held in `to`, matches the block.
    AncX { from: usize, to: usize, plan: Vec<Op> },
}

/// One file's view: the table (its columns read in place), the source, and the structure columns when computed.
pub(super) struct Ctx<'a> {
    pub(super) table: &'a NodeTable,
    pub(super) src: &'a [u8],
    /// Per node, its child sets and parent kind in one cache line (the structure pass).
    pub(super) sets: &'a [ElementSets],
    pub(super) facts: &'a super::facts::Store,
    pub(super) enc: &'a [Vec<u32>],
    pub(super) matches: &'a [u128],
}

impl Ctx<'_> {
    #[inline]
    fn children(&self, n: u32) -> &[Position] {
        node_table::children_of(self.table, Position(n))
    }
    // the node table's own columns, read in place (no copy a file)
    #[inline(always)]
    fn kind(&self, n: u32) -> u16 {
        node_table::kind_of(self.table, Position(n)).0
    }
    #[inline(always)]
    fn field(&self, n: u32) -> u16 {
        node_table::field_of(self.table, Position(n)).0
    }
    #[inline(always)]
    fn parent(&self, n: u32) -> u32 {
        node_table::parent_of(self.table, Position(n)).0
    }
    #[inline(always)]
    fn text(&self, n: u32) -> &[u8] {
        let s = node_table::span_of(self.table, Position(n));
        &self.src[s.start.0 as usize..s.end.0 as usize]
    }
}

/// A check's tests of one node in one test: the kinds it lets through (its not-kinds taken out; every kind when it tests
/// none), its fields, texts and regex. One kernel a check, whatever its shape, in place of a closure a test.
#[derive(Clone)]
struct Own {
    kinds: KSet,
    fields: Option<FSet>,
    texts: Option<Vec<Vec<u8>>>,
    re: Option<regex::bytes::Regex>,
}

const ALL_KINDS: KSet = [u64::MAX; 4];

impl Own {
    fn of(c: &Check) -> Self {
        Own {
            kinds: if c.kinds.is_some() || c.notkinds.is_some() { c.kind_set() } else { ALL_KINDS },
            fields: c.fields,
            texts: c.texts.clone(),
            re: c.re.clone(),
        }
    }
    #[inline(always)]
    fn eval(&self, ctx: &Ctx<'_>, n: u32) -> bool {
        let i = n as usize;
        kbit(&self.kinds, ctx.kind(i as u32))
            && self.fields.as_ref().is_none_or(|s| fbit(s, ctx.field(i as u32)))
            && self.texts.as_ref().is_none_or(|t| {
                let x = ctx.text(n);
                t.iter().any(|y| y.as_slice() == x)
            })
            && self.re.as_ref().is_none_or(|re| re.is_match(ctx.text(n)))
    }
    /// Whether some child of `n` can pass: false when no child has a kind, or a field, this test lets through (the
    /// structure's child-kind and child-field sets), so its children need not be read.
    #[inline]
    fn some_child_can(&self, ctx: &Ctx<'_>, n: u32) -> bool {
        let i = n as usize;
        (self.kinds == ALL_KINDS || {
            let m = &ctx.sets[i].child_kinds;
            (m[0] & self.kinds[0]) | (m[1] & self.kinds[1]) | (m[2] & self.kinds[2]) | (m[3] & self.kinds[3]) != 0
        }) && self.fields.as_ref().is_none_or(|s| {
            let m = &ctx.sets[i].child_fields;
            (m[0] & s[0]) | (m[1] & s[1]) != 0
        })
    }
}

#[inline]
fn passes(ctx: &Ctx<'_>, c: &Check, n: u32) -> bool {
    let k = ctx.kind(n);
    if let Some(s) = &c.kinds {
        if !kbit(s, k) {
            return false;
        }
    }
    if let Some(s) = &c.notkinds {
        if kbit(s, k) {
            return false;
        }
    }
    if let Some(s) = &c.fields {
        if !fbit(s, ctx.field(n)) {
            return false;
        }
    }
    if let Some(t) = &c.texts {
        let text = ctx.text(n);
        if !t.iter().any(|x| x.as_slice() == text) {
            return false;
        }
    }
    if let Some(re) = &c.re {
        if !re.is_match(ctx.text(n)) {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------- loading
pub(super) struct Plan {
    pub(super) keys: Vec<Key>,
    pub(super) ops: Vec<Op>,
    pub(super) regs: usize,
    pub(super) stages: Vec<Stage>,
    /// All preceding stages read syntax alone.
    pub(super) before_facts: usize,
    pub(super) knock: Option<Vec<(Test, bool)>>,
    /// Required words: every clause has a word spelled in the file.
    pub(super) words: Vec<Vec<u16>>,
}

pub(super) struct FRule {
    pub(super) name: String,
    /// Its plans are of the node class: its rows are found from tree survivors (else the second half runs it wherever
    /// the skip kept it).
    pub(super) node: bool,
    pub(super) plans: Vec<Plan>,
    /// The plans of each disjunct with more than one: alternatives, each file entering at the one of them whose buckets
    /// hold the fewest of its nodes.
    pub(super) alts: Vec<Vec<usize>>,
    /// One CNF a branch of the rule: the rule can have a row only where some branch's CNF holds (empty list: unknown).
    pub(super) cnfs: Vec<Vec<Vec<u16>>>,
    /// The fact relations its plans test.
    pub(super) slots: Vec<usize>,
    /// The fact relations some test of its plans reads a value of (a condition or a second key): the others need only the
    /// yes/no question (cols prototype).
    pub(super) valued: Vec<usize>,
    /// The (slot, key column) pairs its plans enter a fact by (cols prototype).
    pub(super) keyed: Vec<(usize, usize)>,
}

// ---------------------------------------------------------------- the spec, typed (from the JSON lines or prepared in the process)
/// The bucket a plan is entered at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Bucket {
    Kinds,
    Fields,
    Words,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SpecCheck {
    pub(super) v: usize,
    pub(super) kinds: Option<Vec<String>>,
    pub(super) notkinds: Option<Vec<String>>,
    pub(super) texts: Option<Vec<Vec<u8>>>,
    pub(super) fields: Option<Vec<String>>,
    pub(super) re: Option<String>,
}

/// A fact test: the relation, its (key column, register) pairs and its (column, op, values) conditions.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SpecFact {
    pub(super) rel: String,
    pub(super) keys: Vec<(String, usize)>,
    pub(super) conds: Vec<(String, String, Vec<String>)>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum SpecOp {
    Check(SpecCheck),
    Up { from: usize, to: usize, leaf: bool },
    Down { from: usize, to: usize, leaf: bool },
    Not(Vec<SpecOp>),
    Exists { from: usize, to: usize, plan: Vec<SpecOp> },
    IsParent { child: usize, parent: usize },
    Fact(SpecFact),
    Enc { from: usize, to: usize, kinds: Vec<String> },
    AncX { from: usize, to: usize, plan: Vec<SpecOp> },
}

/// One plan: the disjunct of its rule it enters (plans of one disjunct are alternatives: each file enters at one of them),
/// its bucket and keys, its ops and registers, and its required-word CNF.
#[derive(Clone, Debug)]
pub(super) struct SpecPlan {
    pub(super) disjunct: usize,
    pub(super) bucket: Bucket,
    pub(super) keys: Vec<String>,
    pub(super) ops: Vec<SpecOp>,
    pub(super) regs: usize,
    pub(super) words: Vec<Vec<String>>,
}

impl SpecPlan {
    pub(super) fn same(&self, o: &SpecPlan) -> bool {
        self.bucket == o.bucket && self.keys == o.keys && self.ops == o.ops && self.words == o.words
    }
}

#[derive(Clone, Debug)]
pub(super) struct SpecRule {
    pub(super) name: String,
    /// Of the node class: its rows are found from tree survivors.
    pub(super) node: bool,
    pub(super) plans: Vec<SpecPlan>,
    pub(super) word_cnfs: Vec<Vec<Vec<String>>>,
}

fn strings(v: &serde_json::Value) -> Vec<String> {
    v.as_array().expect("names").iter().map(|n| n.as_str().expect("name").to_owned()).collect()
}

fn spec_ops(v: &serde_json::Value) -> Vec<SpecOp> {
    let u = |o: &serde_json::Value, k: &str| o[k].as_u64().expect("reg") as usize;
    v.as_array()
        .expect("ops")
        .iter()
        .map(|o| match o["op"].as_str().expect("op") {
            "fact" => SpecOp::Fact(SpecFact {
                rel: o["rel"].as_str().expect("rel").to_owned(),
                keys: o["keys"].as_array().expect("keys").iter().map(|k| (k[0].as_str().expect("key column").to_owned(), k[1].as_u64().expect("reg") as usize)).collect(),
                conds: o["conds"]
                    .as_array()
                    .expect("conds")
                    .iter()
                    .map(|c| (c[0].as_str().expect("column").to_owned(), c[1].as_str().expect("op").to_owned(), c[2].as_array().expect("vals").iter().map(|x| x.as_str().expect("val").to_owned()).collect()))
                    .collect(),
            }),
            "check" => SpecOp::Check(SpecCheck {
                v: u(o, "v"),
                kinds: o.get("kinds").map(strings),
                notkinds: o.get("notkinds").map(strings),
                texts: o.get("texts").map(|x| x.as_array().expect("texts").iter().map(|t| t.as_str().expect("text").as_bytes().to_vec()).collect()),
                fields: o.get("fields").map(strings),
                re: o.get("re").map(|x| x.as_str().expect("re").to_owned()),
            }),
            "up" => SpecOp::Up { from: u(o, "from"), to: u(o, "to"), leaf: o["leaf"].as_bool().unwrap_or(false) },
            "down" => SpecOp::Down { from: u(o, "from"), to: u(o, "to"), leaf: o["leaf"].as_bool().unwrap_or(false) },
            "not" => SpecOp::Not(spec_ops(&o["plan"])),
            "exists" => SpecOp::Exists { from: u(o, "from"), to: u(o, "to"), plan: spec_ops(&o["plan"]) },
            "isparent" => SpecOp::IsParent { child: u(o, "child"), parent: u(o, "parent") },
            "enc" => SpecOp::Enc { from: u(o, "from"), to: u(o, "to"), kinds: strings(&o["kinds"]) },
            "ancx" => SpecOp::AncX { from: u(o, "from"), to: u(o, "to"), plan: spec_ops(&o["plan"]) },
            other => panic!("op {other}"),
        })
        .collect()
}

fn cnf_strings(v: &serde_json::Value) -> Vec<Vec<String>> {
    v.as_array().map(|cl| cl.iter().map(strings).collect()).unwrap_or_default()
}

/// One line of a prepared plans file (`funnel4j.jsonl`) as the typed spec: each plan its own disjunct.
fn spec_of_line(r: &serde_json::Value) -> SpecRule {
    let plans = r["plans"]
        .as_array()
        .expect("plans")
        .iter()
        .enumerate()
        .map(|(i, p)| SpecPlan {
            disjunct: i,
            bucket: match p["bucket"].as_str() {
                Some("fields") => Bucket::Fields,
                Some("words") => Bucket::Words,
                _ => Bucket::Kinds,
            },
            keys: strings(&p["keys"]),
            ops: spec_ops(&p["ops"]),
            regs: p["regs"].as_u64().expect("regs") as usize,
            words: cnf_strings(&p["words"]),
        })
        .collect();
    SpecRule {
        name: r["rule"].as_str().expect("rule").to_owned(),
        node: r["class"].as_str() == Some("node"),
        plans,
        word_cnfs: r["word_cnfs"].as_array().map(|a| a.iter().map(cnf_strings).collect()).unwrap_or_default(),
    }
}

fn kset_of(names: &[String], vocab: &Vocab, missing: &mut Vec<String>) -> KSet {
    let mut s = [0u64; 4];
    for n in names {
        match vocab.kinds.get(n) {
            Some(c) => s[usize::from(*c) / 64] |= 1 << (c % 64),
            None => missing.push(format!("kind {n}")),
        }
    }
    s
}

fn fset_of(names: &[String], vocab: &Vocab, missing: &mut Vec<String>) -> FSet {
    let mut s = [0u64; 2];
    for n in names {
        match vocab.fields.get(n) {
            Some(c) if *c < 128 => s[usize::from(*c) / 64] |= 1 << (c % 64),
            _ => missing.push(format!("field {n}")),
        }
    }
    s
}

fn ops_of(v: &[SpecOp], vocab: &Vocab, missing: &mut Vec<String>, fc: &mut FactLoad<'_>) -> Vec<Op> {
    v.iter()
        .filter_map(|o| {
            Some(match o {
                SpecOp::Fact(f) => {
                    let op = fc.catalog.op_of(&f.rel, &f.keys, &f.conds, fc.interner)?;
                    if !fc.slots.contains(&op.slot) {
                        fc.slots.push(op.slot);
                    }
                    if !fc.keyed.contains(&(op.slot, op.keys[0].0)) {
                        fc.keyed.push((op.slot, op.keys[0].0));
                    }
                    if (!op.conds.is_empty() || op.keys.len() > 1) && !fc.valued.contains(&op.slot) {
                        fc.valued.push(op.slot);
                    }
                    Op::Fact(op)
                }
                SpecOp::Check(c) => Op::Check(Check {
                    v: c.v,
                    kinds: c.kinds.as_ref().map(|x| kset_of(x, vocab, missing)),
                    notkinds: c.notkinds.as_ref().map(|x| kset_of(x, vocab, missing)),
                    texts: c.texts.clone(),
                    fields: c.fields.as_ref().map(|x| fset_of(x, vocab, missing)),
                    re: c.re.as_ref().map(|x| regex::bytes::Regex::new(x).expect("regex")),
                }),
                SpecOp::Up { from, to, leaf } => Op::Up { from: *from, to: *to, leaf: *leaf },
                SpecOp::Down { from, to, leaf } => Op::Down { from: *from, to: *to, leaf: *leaf },
                SpecOp::Not(p) => Op::Not(ops_of(p, vocab, missing, fc)),
                SpecOp::Exists { from, to, plan } => Op::Exists { from: *from, to: *to, plan: ops_of(plan, vocab, missing, fc) },
                SpecOp::IsParent { child, parent } => Op::IsParent { child: *child, parent: *parent },
                SpecOp::Enc { from, to, kinds } => {
                    let set = kset_of(kinds, vocab, missing);
                    let at = fc.catalog.enc_sets.iter().position(|x| *x == set).unwrap_or_else(|| {
                        fc.catalog.enc_sets.push(set);
                        fc.catalog.enc_sets.len() - 1
                    });
                    Op::Enc { from: *from, to: *to, set: at }
                }
                SpecOp::AncX { from, to, plan } => Op::AncX { from: *from, to: *to, plan: ops_of(plan, vocab, missing, fc) },
            })
        })
        .collect()
}

/// The required words of the pack, numbered.
#[derive(Default)]
pub(super) struct Words {
    pub(super) ids: HashMap<Vec<u8>, u16>,
    pub(super) names: Vec<String>,
}

impl Words {
    fn id(&mut self, w: &str) -> u16 {
        if let Some(i) = self.ids.get(w.as_bytes()) {
            return *i;
        }
        let i = self.names.len() as u16;
        self.names.push(w.to_owned());
        self.ids.insert(w.as_bytes().to_vec(), i);
        i
    }
    fn cnf(&mut self, v: &[Vec<String>]) -> Vec<Vec<u16>> {
        v.iter().map(|c| c.iter().map(|w| self.id(w)).collect()).collect()
    }
}

#[inline]
pub(super) fn bit(set: &[u64], w: u16) -> bool {
    set.get(usize::from(w) / 64).is_some_and(|x| x >> (w % 64) & 1 == 1)
}

/// Every clause has a word present.
pub(super) fn cnf_holds(cnf: &[Vec<u16>], present: &[u64]) -> bool {
    cnf.iter().all(|cl| cl.iter().any(|w| bit(present, *w)))
}

/// What loading fact tests writes to: the catalog of fact relations, the run's interner, and the rule's slots.
pub(super) struct FactLoad<'l> {
    pub(super) catalog: &'l mut super::facts::Catalog,
    pub(super) interner: &'l mut super::facts::Interner,
    pub(super) slots: Vec<usize>,
    pub(super) valued: Vec<usize>,
    pub(super) keyed: Vec<(usize, usize)>,
}

/// The plans file prepared beforehand (the oracle path).
pub(super) fn load(path: &str, vocab: &Vocab, catalog: &mut super::facts::Catalog, interner: &mut super::facts::Interner) -> (Vec<FRule>, Words) {
    let specs: Vec<SpecRule> = std::fs::read_to_string(path).expect("funnel plans").lines().map(|l| spec_of_line(&serde_json::from_str(l).expect("json"))).collect();
    build(&specs, vocab, catalog, interner)
}

/// The harness's rules from the typed spec: each plan's keys coded and its stages specialised; the alternatives of each
/// disjunct grouped (a plan with no key in the vocabulary is left out of its disjunct; a disjunct with none left, or a plan
/// file's plan with no key, makes the rule run from the skip alone).
pub(super) fn build(specs: &[SpecRule], vocab: &Vocab, catalog: &mut super::facts::Catalog, interner: &mut super::facts::Interner) -> (Vec<FRule>, Words) {
    let mut fc = FactLoad { catalog, interner, slots: Vec::new(), valued: Vec::new(), keyed: Vec::new() };
    let mut missing = Vec::new();
    let mut rules = Vec::new();
    let mut words = Words::default();
    for r in specs {
        let mut plans = Vec::new();
        let mut disjuncts: Vec<(usize, Vec<usize>)> = Vec::new();
        let mut enterable = true;
        for p in &r.plans {
            let keys: Vec<Key> = p
                .keys
                .iter()
                .filter_map(|k| {
                    let code = match p.bucket {
                        Bucket::Words => Some(Key::Word(words.id(k))),
                        Bucket::Fields => vocab.fields.get(k).map(|c| Key::Field(*c)),
                        Bucket::Kinds => vocab.kinds.get(k).map(|c| Key::Kind(*c)),
                    };
                    if code.is_none() {
                        missing.push(format!("bucket {k}"));
                    }
                    code
                })
                .collect();
            let ops = ops_of(&p.ops, vocab, &mut missing, &mut fc);
            let (stages, before_facts) = compile(&ops);
            let knock = knock_tests(&ops);
            let cnf = words.cnf(&p.words);
            let at = disjuncts.iter().position(|(d, _)| *d == p.disjunct).unwrap_or_else(|| {
                disjuncts.push((p.disjunct, Vec::new()));
                disjuncts.len() - 1
            });
            // redo-honest: a plan with no bucket key is entered from no bucket, so it finds no survivor; a node-class rule with
            // such a plan would never be seeded and its findings were lost. Left out of its disjunct; a disjunct with no plan
            // left runs the rule from the skip alone.
            if keys.is_empty() {
                continue;
            }
            disjuncts[at].1.push(plans.len());
            plans.push(Plan { keys, ops, regs: p.regs, stages, before_facts, knock, words: cnf });
        }
        for (_, d) in &disjuncts {
            if d.is_empty() {
                enterable = false;
            }
        }
        let cnfs: Vec<Vec<Vec<u16>>> = r.word_cnfs.iter().map(|c| words.cnf(c)).collect();
        let slots = std::mem::take(&mut fc.slots);
        let valued = std::mem::take(&mut fc.valued);
        let keyed = std::mem::take(&mut fc.keyed);
        let alts = disjuncts.into_iter().map(|(_, d)| d).filter(|d| d.len() > 1).collect();
        rules.push(FRule { name: r.name.clone(), node: r.node && enterable, plans, alts, cnfs, slots, valued, keyed });
    }
    missing.sort();
    missing.dedup();
    if !missing.is_empty() {
        eprintln!("funnel: names not in the vocabulary: {missing:?}");
    }
    (rules, words)
}

// ---------------------------------------------------------------- 1. the generic interpreter
pub(super) fn exec(ctx: &Ctx<'_>, ops: &[Op], pc: usize, regs: &mut [u32], steps: &mut u64) -> bool {
    let Some(op) = ops.get(pc) else { return true };
    *steps += 1;
    match op {
        Op::Check(c) => passes(ctx, c, regs[c.v]) && exec(ctx, ops, pc + 1, regs, steps),
        Op::Up { from, to, .. } => {
            let n = regs[*from];
            let p = ctx.parent(n);
            if p == n {
                return false;
            }
            regs[*to] = p;
            exec(ctx, ops, pc + 1, regs, steps)
        }
        Op::Down { from, to, .. } => {
            for c in ctx.children(regs[*from]) {
                *steps += 1;
                regs[*to] = c.0;
                if exec(ctx, ops, pc + 1, regs, steps) {
                    return true;
                }
            }
            false
        }
        Op::Not(sub) => !exec(ctx, sub, 0, regs, steps) && exec(ctx, ops, pc + 1, regs, steps),
        Op::Exists { from, to, plan } => {
            let mut found = false;
            for c in ctx.children(regs[*from]) {
                *steps += 1;
                regs[*to] = c.0;
                if exec(ctx, plan, 0, regs, steps) {
                    found = true;
                    break;
                }
            }
            found && exec(ctx, ops, pc + 1, regs, steps)
        }
        Op::IsParent { child, parent } => {
            let c = regs[*child];
            ctx.parent(c) == regs[*parent] && ctx.parent(c) != c && exec(ctx, ops, pc + 1, regs, steps)
        }
        Op::Fact(f) => ctx.facts.holds(f, regs, ctx.matches) && exec(ctx, ops, pc + 1, regs, steps),
        Op::Enc { from, to, set } => {
            let e = ctx.enc[*set][regs[*from] as usize];
            e != u32::MAX && {
                regs[*to] = e;
                exec(ctx, ops, pc + 1, regs, steps)
            }
        }
        Op::AncX { from, to, plan } => {
            let mut n = regs[*from];
            let mut found = false;
            loop {
                let p = ctx.parent(n);
                if p == n {
                    break;
                }
                *steps += 1;
                regs[*to] = p;
                if exec(ctx, plan, 0, regs, steps) {
                    found = true;
                    break;
                }
                n = p;
            }
            found && exec(ctx, ops, pc + 1, regs, steps)
        }
    }
}

// ---------------------------------------------------------------- 2. specialised at load, selection vectors
/// Rows of registers, `r` a row.
#[derive(Default)]
pub(super) struct Rows {
    pub(super) r: usize,
    pub(super) d: Vec<u32>,
}

impl Rows {
    pub(super) fn len(&self) -> usize {
        self.d.len().checked_div(self.r).unwrap_or(0)
    }
    /// The rows of one candidate each, register 0 holding it.
    pub(super) fn enter(&mut self, r: usize, nodes: &[u32]) {
        self.r = r;
        self.d.clear();
        if r == 1 {
            self.d.extend_from_slice(nodes);
            return;
        }
        self.d.resize(nodes.len() * r, 0);
        for (j, n) in nodes.iter().enumerate() {
            self.d[j * r] = *n;
        }
    }
    #[inline]
    fn retain<F: Fn(&[u32]) -> bool>(&mut self, f: F) {
        let r = self.r;
        let mut w = 0;
        for i in 0..self.len() {
            if f(&self.d[i * r..i * r + r]) {
                if w != i {
                    self.d.copy_within(i * r..i * r + r, w * r);
                }
                w += 1;
            }
        }
        self.d.truncate(w * r);
    }
}

pub(super) type Stage = Box<dyn for<'c> Fn(&Ctx<'c>, &mut Rows, &mut u64)>;

fn filt<F>(f: F) -> Stage
where
    F: for<'c> Fn(&Ctx<'c>, &[u32]) -> bool + 'static,
{
    Box::new(move |ctx, rows, steps| {
        *steps += rows.len() as u64;
        rows.retain(|row| f(ctx, row));
    })
}

/// Some child of the row's `from` passes `head` (children read only when one can).
fn any_child(from: usize, head: Own) -> Stage {
    filt(move |ctx, row| {
        let n = row[from];
        head.some_child_can(ctx, n) && ctx.children(n).iter().any(|x| head.eval(ctx, x.0))
    })
}

/// A stage that moves each row from register `from` to the node `to_of` gives (`u32::MAX`: none, the row is dropped).
fn mover<F>(from: usize, to: usize, to_of: F) -> Stage
where
    F: for<'c> Fn(&Ctx<'c>, u32) -> u32 + 'static,
{
    Box::new(move |ctx, rows, steps| {
        *steps += rows.len() as u64;
        let r = rows.r;
        let mut w = 0;
        for j in 0..rows.len() {
            let t = to_of(ctx, rows.d[j * r + from]);
            if t != u32::MAX {
                if w != j {
                    rows.d.copy_within(j * r..j * r + r, w * r);
                }
                rows.d[w * r + to] = t;
                w += 1;
            }
        }
        rows.d.truncate(w * r);
    })
}

/// Compiles a plan into stages: one kernel a check; a leaf `up` + kind check as a parent-kind test; a leaf `down` + kind
/// check as a child-kind-bitset test; any other leaf `down` + check, and an `exists` of one check, as one scan of the
/// children, skipped when the child-kind and child-field sets rule it out; a `down` + check of the child as one expansion
/// to the children that pass.
fn compile(ops: &[Op]) -> (Vec<Stage>, usize) {
    let mut first_fact = None;
    let mut out: Vec<Stage> = Vec::new();
    let mut i = 0;
    while i < ops.len() {
        let next_check = |to: usize| match ops.get(i + 1) {
            Some(Op::Check(c)) if c.v == to => Some(c.clone()),
            _ => None,
        };
        if first_fact.is_none() {
            super::file_seeds::visits_facts(&ops[i], &mut |_| first_fact = Some(out.len()));
        }
        match &ops[i] {
            Op::Check(c) => {
                let (v, own) = (c.v, Own::of(c));
                out.push(filt(move |ctx, row| own.eval(ctx, row[v])));
            }
            Op::Up { from, to, leaf } => {
                let from = *from;
                match next_check(*to) {
                    Some(c) if *leaf && c.kinds_only() => {
                        let s = c.kind_set();
                        out.push(filt(move |ctx, row| {
                            let n = row[from] as usize;
                            !ctx.sets[n].root && kbit(&s, ctx.sets[n].parent_kind)
                        }));
                        i += 1;
                    }
                    _ if *leaf && next_check(*to).is_none() => out.push(filt(move |ctx, row| ctx.parent(row[from]) != row[from])),
                    _ => out.push(mover(from, *to, |ctx, n| {
                            let p = ctx.parent(n);
                            if p == n { u32::MAX } else { p }
                        })),
                }
            }
            Op::Down { from, to, leaf } => {
                let from = *from;
                match next_check(*to) {
                    Some(c) if *leaf && c.kinds_only() => {
                        let s = c.kind_set();
                        out.push(filt(move |ctx, row| {
                            let m = &ctx.sets[row[from] as usize].child_kinds;
                            (m[0] & s[0]) | (m[1] & s[1]) | (m[2] & s[2]) | (m[3] & s[3]) != 0
                        }));
                        i += 1;
                    }
                    Some(c) if *leaf => {
                        out.push(any_child(from, Own::of(&c)));
                        i += 1;
                    }
                    None if *leaf => out.push(filt(move |ctx, row| !ctx.children(row[from]).is_empty())),
                    Some(c) => {
                        out.push(expand(from, *to, Some(Own::of(&c))));
                        i += 1;
                    }
                    None => out.push(expand(from, *to, None)),
                }
            }
            Op::Not(_) => out.push(pred_stage(compile_pred(std::slice::from_ref(&ops[i])))),
            Op::Exists { from, to, plan } => {
                let (from, to) = (*from, *to);
                match plan.as_slice() {
                    [] => out.push(filt(move |ctx, row| !ctx.children(row[from]).is_empty())),
                    [Op::Check(c)] if c.v == to && c.kinds_only() => {
                        let s = c.kind_set();
                        out.push(filt(move |ctx, row| {
                            let m = &ctx.sets[row[from] as usize].child_kinds;
                            (m[0] & s[0]) | (m[1] & s[1]) | (m[2] & s[2]) | (m[3] & s[3]) != 0
                        }));
                    }
                    [Op::Check(c)] if c.v == to => out.push(any_child(from, Own::of(c))),
                    _ => out.push(pred_stage(compile_pred(std::slice::from_ref(&ops[i])))),
                }
            }
            Op::Fact(f) => {
                let f = f.clone();
                out.push(filt(move |ctx, row| ctx.facts.holds(&f, row, ctx.matches)));
            }
            Op::Enc { from, to, set } => {
                let set = *set;
                out.push(mover(*from, *to, move |ctx, n| ctx.enc[set][n as usize]));
            }
            Op::AncX { .. } => out.push(pred_stage(compile_pred(std::slice::from_ref(&ops[i])))),
            Op::IsParent { child, parent } => {
                let (c, p) = (*child, *parent);
                out.push(filt(move |ctx, row| {
                        let n = row[c] as usize;
                        ctx.parent(n as u32) == row[p] && ctx.parent(n as u32) as usize != n
                    }));
            }
        }
        i += 1;
    }
    let boundary = first_fact.unwrap_or(out.len());
    (out, boundary)
}

/// A load-time specialised predicate over registers: one closure per operation, chained in continuation style (a `down`
/// tries each child against the rest of the chain).
pub(super) type Pred = Box<dyn for<'c> Fn(&Ctx<'c>, &mut [u32], &mut u64) -> bool>;

fn check_pred(c: &Check, next: Pred) -> Pred {
    let (v, own) = (c.v, Own::of(c));
    Box::new(move |ctx, r, s| {
        *s += 1;
        own.eval(ctx, r[v]) && next(ctx, r, s)
    })
}

/// A `down` from `from` to `to` whose first test is `head` on the child: the children are read only when one can pass,
/// and the rest of the chain runs only on a child that passes.
fn down_pred(from: usize, to: usize, head: Own, next: Pred) -> Pred {
    Box::new(move |ctx, r, s| {
        *s += 1;
        let n = r[from];
        if !head.some_child_can(ctx, n) {
            return false;
        }
        for c in ctx.children(n) {
            *s += 1;
            if head.eval(ctx, c.0) {
                r[to] = c.0;
                if next(ctx, r, s) {
                    return true;
                }
            }
        }
        false
    })
}

pub(super) fn compile_pred(ops: &[Op]) -> Pred {
    let mut k: Pred = Box::new(|_, _, _| true);
    let mut i = ops.len();
    while i > 0 {
        i -= 1;
        // a leaf move followed by a kind-only check of the reached register fuses into a structure-column test
        let fused = |i: usize| -> Option<(usize, usize, bool, Check)> {
            let (from, to, up, leaf) = match ops.get(i)? {
                Op::Up { from, to, leaf } => (*from, *to, true, *leaf),
                Op::Down { from, to, leaf } => (*from, *to, false, *leaf),
                _ => return None,
            };
            match ops.get(i + 1)? {
                Op::Check(c) if leaf && c.v == to && c.kinds_only() => Some((from, to, up, c.clone())),
                _ => None,
            }
        };
        if i > 0 {
            if let Some((from, _, up, c)) = fused(i - 1) {
                if matches!(ops[i], Op::Check(_)) {
                    let set = c.kind_set();
                    let next = k;
                    k = if up {
                        Box::new(move |ctx, r, s| {
                            *s += 1;
                            let n = r[from] as usize;
                            !ctx.sets[n].root && kbit(&set, ctx.sets[n].parent_kind) && next(ctx, r, s)
                        })
                    } else {
                        Box::new(move |ctx, r, s| {
                            *s += 1;
                            let m = &ctx.sets[r[from] as usize].child_kinds;
                            ((m[0] & set[0]) | (m[1] & set[1]) | (m[2] & set[2]) | (m[3] & set[3]) != 0) && next(ctx, r, s)
                        })
                    };
                    i -= 1;
                    continue;
                }
            }
            // any other `down` followed by a check of the child: the check is the down's head
            if let (Op::Down { from, to, .. }, Op::Check(c)) = (&ops[i - 1], &ops[i]) {
                if c.v == *to {
                    k = down_pred(*from, *to, Own::of(c), k);
                    i -= 1;
                    continue;
                }
            }
        }
        let next = k;
        k = match &ops[i] {
            Op::Check(c) => check_pred(c, next),
            Op::Up { from, to, .. } => {
                let (from, to) = (*from, *to);
                Box::new(move |ctx, r, s| {
                    *s += 1;
                    let n = r[from];
                    let p = ctx.parent(n);
                    p != n && {
                        r[to] = p;
                        next(ctx, r, s)
                    }
                })
            }
            Op::Down { from, to, .. } => {
                let (from, to) = (*from, *to);
                Box::new(move |ctx, r, s| {
                    *s += 1;
                    for c in ctx.children(r[from]) {
                        r[to] = c.0;
                        if next(ctx, r, s) {
                            return true;
                        }
                    }
                    false
                })
            }
            Op::Exists { from, to, plan } => {
                let (from, to) = (*from, *to);
                match plan.as_slice() {
                    [Op::Check(c)] if c.v == to && c.kinds_only() => {
                        let set = c.kind_set();
                        Box::new(move |ctx, r, s| {
                            *s += 1;
                            let m = &ctx.sets[r[from] as usize].child_kinds;
                            ((m[0] & set[0]) | (m[1] & set[1]) | (m[2] & set[2]) | (m[3] & set[3]) != 0) && next(ctx, r, s)
                        })
                    }
                    // the block's first test is a check of the child: one kernel a child, the rest of the block only on
                    // the children that pass it, none read when the child sets rule them out
                    [Op::Check(c), rest @ ..] if c.v == to => {
                        let head = Own::of(c);
                        let inner = compile_pred(rest);
                        Box::new(move |ctx, r, s| {
                            *s += 1;
                            let n = r[from];
                            if !head.some_child_can(ctx, n) {
                                return false;
                            }
                            let mut found = false;
                            for c in ctx.children(n) {
                                *s += 1;
                                if head.eval(ctx, c.0) {
                                    r[to] = c.0;
                                    if inner(ctx, r, s) {
                                        found = true;
                                        break;
                                    }
                                }
                            }
                            found && next(ctx, r, s)
                        })
                    }
                    _ => {
                        let inner = compile_pred(plan);
                        Box::new(move |ctx, r, s| {
                            *s += 1;
                            let mut found = false;
                            for c in ctx.children(r[from]) {
                                r[to] = c.0;
                                if inner(ctx, r, s) {
                                    found = true;
                                    break;
                                }
                            }
                            found && next(ctx, r, s)
                        })
                    }
                }
            }
            Op::Not(sub) => {
                let inner = compile_pred(sub);
                Box::new(move |ctx, r, s| {
                    *s += 1;
                    !inner(ctx, r, s) && next(ctx, r, s)
                })
            }
            Op::Fact(f) => {
                let f = f.clone();
                Box::new(move |ctx, r, s| {
                    *s += 1;
                    ctx.facts.holds(&f, r, ctx.matches) && next(ctx, r, s)
                })
            }
            Op::Enc { from, to, set } => {
                let (from, to, set) = (*from, *to, *set);
                Box::new(move |ctx, r, s| {
                    *s += 1;
                    let e = ctx.enc[set][r[from] as usize];
                    e != u32::MAX && {
                        r[to] = e;
                        next(ctx, r, s)
                    }
                })
            }
            Op::AncX { from, to, plan } => {
                let (from, to) = (*from, *to);
                let inner = compile_pred(plan);
                Box::new(move |ctx, r, s| {
                    *s += 1;
                    let mut n = r[from];
                    let mut found = false;
                    loop {
                        let p = ctx.parent(n);
                        if p == n {
                            break;
                        }
                        r[to] = p;
                        if inner(ctx, r, s) {
                            found = true;
                            break;
                        }
                        n = p;
                    }
                    found && next(ctx, r, s)
                })
            }
            Op::IsParent { child, parent } => {
                let (c, p) = (*child, *parent);
                Box::new(move |ctx, r, s| {
                    *s += 1;
                    let n = r[c] as usize;
                    ctx.parent(n as u32) == r[p] && ctx.parent(n as u32) as usize != n && next(ctx, r, s)
                })
            }
        };
    }
    k
}

/// A filter stage over the selection vector that runs a specialised predicate on each row's registers.
fn pred_stage(pred: Pred) -> Stage {
    Box::new(move |ctx, rows, steps| {
        let r = rows.r;
        let mut regs = [0u32; 256];
        let mut w = 0;
        for j in 0..rows.len() {
            regs[..r].copy_from_slice(&rows.d[j * r..j * r + r]);
            if pred(ctx, &mut regs, steps) {
                if w != j {
                    rows.d.copy_within(j * r..j * r + r, w * r);
                }
                w += 1;
            }
        }
        rows.d.truncate(w * r);
    })
}

/// A row for each child of `from` (each child that passes `head`, when the next op checks the child).
fn expand(from: usize, to: usize, head: Option<Own>) -> Stage {
    Box::new(move |ctx, rows, steps| {
        *steps += rows.len() as u64;
        let r = rows.r;
        let mut d = Vec::with_capacity(rows.d.len());
        for j in 0..rows.len() {
            let row = &rows.d[j * r..j * r + r];
            let n = row[from];
            if head.as_ref().is_some_and(|h| !h.some_child_can(ctx, n)) {
                continue;
            }
            for c in ctx.children(n) {
                if head.as_ref().is_some_and(|h| !h.eval(ctx, c.0)) {
                    continue;
                }
                d.extend_from_slice(row);
                let at = d.len() - r + to;
                d[at] = c.0;
            }
        }
        rows.d = d;
    })
}

// ---------------------------------------------------------------- 3. knock-out masks
#[derive(Clone, Debug)]
pub(super) enum Test {
    Own(Check),
    ParentKind(KSet),
    ChildKind(KSet),
}

impl Test {
    fn key(&self) -> String {
        match self {
            Test::Own(c) => format!("own {:?} {:?} {:?} {:?} {:?}", c.kinds, c.notkinds, c.texts, c.fields, c.re.as_ref().map(regex::bytes::Regex::as_str)),
            Test::ParentKind(s) => format!("parent {s:?}"),
            Test::ChildKind(s) => format!("child {s:?}"),
        }
    }
    #[inline(always)]
    fn eval(&self, ctx: &Ctx<'_>, n: u32) -> bool {
        match self {
            Test::Own(c) => passes(ctx, c, n),
            Test::ParentKind(s) => !ctx.sets[n as usize].root && kbit(s, ctx.sets[n as usize].parent_kind),
            Test::ChildKind(s) => {
                let m = &ctx.sets[n as usize].child_kinds;
                (m[0] & s[0]) | (m[1] & s[1]) | (m[2] & s[2]) | (m[3] & s[3]) != 0
            }
        }
    }
}

/// The plan as a conjunction of (test on the anchor, polarity), when it is one.
fn knock_tests(ops: &[Op]) -> Option<Vec<(Test, bool)>> {
    let mut out = Vec::new();
    let mut i = 0;
    let check_after = |i: usize, to: usize| match ops.get(i + 1) {
        Some(Op::Check(c)) if c.v == to && c.kinds_only() => Some(c.kind_set()),
        _ => None,
    };
    while i < ops.len() {
        match &ops[i] {
            Op::Check(c) if c.v == 0 => {
                let mut c = c.clone();
                c.v = 0;
                out.push((Test::Own(c), true));
            }
            Op::Up { from: 0, to, leaf: true } => {
                let s = check_after(i, *to)?;
                out.push((Test::ParentKind(s), true));
                i += 1;
            }
            Op::Down { from: 0, to, leaf: true } => {
                let s = check_after(i, *to)?;
                out.push((Test::ChildKind(s), true));
                i += 1;
            }
            Op::Exists { from: 0, to, plan } => match plan.as_slice() {
                [Op::Check(c)] if c.v == *to && c.kinds_only() => out.push((Test::ChildKind(c.kind_set()), true)),
                _ => return None,
            },
            Op::Not(sub) => match sub.as_slice() {
                [Op::Exists { from: 0, to, plan }] => match plan.as_slice() {
                    [Op::Check(c)] if c.v == *to && c.kinds_only() => out.push((Test::ChildKind(c.kind_set()), false)),
                    _ => return None,
                },
                [Op::Down { from: 0, to, leaf: true }, Op::Check(c)] if c.v == *to && c.kinds_only() => out.push((Test::ChildKind(c.kind_set()), false)),
                [Op::Check(c)] if c.v == 0 => out.push((Test::Own(c.clone()), false)),
                [Op::Up { from: 0, to, leaf: true }, Op::Check(c)] if c.v == *to && c.kinds_only() => out.push((Test::ParentKind(c.kind_set()), false)),
                _ => return None,
            },
            _ => return None,
        }
        i += 1;
    }
    Some(out)
}

/// One rule entered at one bucket key: the rule's plans that start there (a candidate survives when any of them matches).
pub(super) struct Unit {
    pub(super) rule: usize,
    pub(super) key: Key,
    pub(super) plans: Vec<usize>,
    pub(super) knock: bool,
}

pub(super) fn units(rules: &[FRule]) -> Vec<Unit> {
    let mut out: Vec<Unit> = Vec::new();
    for (r, rule) in rules.iter().enumerate() {
        let mut by_key: Vec<(Key, Vec<usize>)> = Vec::new();
        for (p, plan) in rule.plans.iter().enumerate() {
            for k in &plan.keys {
                match by_key.iter_mut().find(|(x, _)| x == k) {
                    Some((_, v)) => v.push(p),
                    None => by_key.push((*k, vec![p])),
                }
            }
        }
        for (key, plans) in by_key {
            let knock = plans.iter().all(|p| rule.plans[*p].knock.is_some());
            out.push(Unit { rule: r, key, plans, knock });
        }
    }
    out
}

/// The knock-out units of one bucket key: their plans as slots, the distinct tests and pass masks.
pub(super) struct Group {
    pub(super) key: Key,
    /// The unit of each slot.
    pub(super) slot_unit: Vec<usize>,
    pub(super) units: Vec<usize>,
    words: usize,
    all: Vec<u64>,
    tests: Vec<(Test, Vec<u64>, Vec<u64>)>,
}

pub(super) fn groups(rules: &[FRule], units: &[Unit]) -> Vec<Group> {
    let mut keys: Vec<Key> = units.iter().filter(|u| u.knock).map(|u| u.key).collect();
    keys.sort();
    keys.dedup();
    let mut out = Vec::new();
    for key in keys {
        let mut slots: Vec<(usize, usize)> = Vec::new();
        let mut members = Vec::new();
        for (ui, u) in units.iter().enumerate() {
            if u.knock && u.key == key {
                members.push(ui);
                for p in &u.plans {
                    slots.push((ui, *p));
                }
            }
        }
        let words = slots.len().div_ceil(64);
        let mut all = vec![0u64; words];
        for i in 0..slots.len() {
            all[i / 64] |= 1 << (i % 64);
        }
        let mut tests: Vec<(Test, Vec<u64>, Vec<u64>)> = Vec::new();
        let mut index: HashMap<String, usize> = HashMap::new();
        for (i, (ui, p)) in slots.iter().enumerate() {
            for (t, polarity) in rules[units[*ui].rule].plans[*p].knock.as_ref().expect("knock") {
                let at = *index.entry(t.key()).or_insert_with(|| {
                    tests.push((t.clone(), all.clone(), all.clone()));
                    tests.len() - 1
                });
                // the test true kills the slots that require it false, and false the slots that require it true
                let mask = if *polarity { &mut tests[at].2 } else { &mut tests[at].1 };
                mask[i / 64] &= !(1u64 << (i % 64));
            }
        }
        out.push(Group { key, slot_unit: slots.iter().map(|(u, _)| *u).collect(), units: members, words, all, tests });
    }
    out
}

impl Group {
    pub(super) fn words(&self) -> usize {
        self.words
    }
    pub(super) fn tests(&self) -> usize {
        self.tests.len()
    }
}

/// Runs one group over its bucket with the slots the file keeps; counts each surviving (unit, node) once. Returns steps.
pub(super) fn knock(ctx: &Ctx<'_>, group: &Group, nodes: &[u32], file: &[u64], survivors: &mut [u64], last: &mut [u32]) -> u64 {
    let mut steps = 0u64;
    let mut alive = vec![0u64; group.words];
    for &n in nodes {
        for w in 0..group.words {
            alive[w] = group.all[w] & file[w];
        }
        for (t, if_true, if_false) in &group.tests {
            steps += 1;
            let m = if t.eval(ctx, n) { if_true } else { if_false };
            let mut any = 0;
            for w in 0..group.words {
                alive[w] &= m[w];
                any |= alive[w];
            }
            if any == 0 {
                break;
            }
        }
        for w in 0..group.words {
            let mut bits = alive[w];
            while bits != 0 {
                let b = bits.trailing_zeros() as usize;
                let u = group.slot_unit[w * 64 + b];
                if last[u] != n + 1 {
                    last[u] = n + 1;
                    survivors[u] += 1;
                }
                bits &= bits - 1;
            }
        }
    }
    steps
}

// ---------------------------------------------------------------- the file's required words
/// A memo entry not looked up yet.
const UNSEEN: u32 = u32::MAX;
/// A memo entry whose text is no required word.
const NO_WORD: u32 = u32::MAX - 1;

/// The high bit of a name code: a fixed text of the language, the same in every file of the run.
pub(super) const FIXED_TEXT: u32 = 1 << 31;

/// The file's required words from its name tokens, in id order: the present bits and the (word, token id) pairs. A token the
/// walk gave a code (`codes`: the file's own tree, in id order) is looked up once a code through `memo` (a fixed text's,
/// once a run through `fixed`); every other (an embedded segment's, or all of a file whose bytes were no text) by its bytes.
#[expect(clippy::too_many_arguments, reason = "throwaway prototype")]
pub(super) fn file_words(
    table: &NodeTable,
    src: &[u8],
    name_token: u16,
    codes: &[u32],
    words: &Words,
    memo: &mut Vec<u32>,
    fixed: &mut Vec<u32>,
    present: &mut [u64],
    pairs: &mut Vec<(u16, u32)>,
) {
    present.iter_mut().for_each(|w| *w = 0);
    pairs.clear();
    memo.clear();
    let (starts, ends) = (node_table::starts(table), node_table::ends(table));
    let text = |i: usize| &src[starts[i] as usize..ends[i] as usize];
    for (j, p) in node_table::ids_of_kind(table, ryo::languages::KindBits(name_token)).iter().enumerate() {
        let i = p.0 as usize;
        let w = match codes.get(j) {
            Some(&c) => {
                let m = if c & FIXED_TEXT == 0 { &mut *memo } else { &mut *fixed };
                let c = (c & !FIXED_TEXT) as usize;
                if c >= m.len() {
                    m.resize(c + 1, UNSEEN);
                }
                if m[c] == UNSEEN {
                    m[c] = words.ids.get(text(i)).map_or(NO_WORD, |w| u32::from(*w));
                }
                m[c]
            }
            None => words.ids.get(text(i)).map_or(NO_WORD, |w| u32::from(*w)),
        };
        if w != NO_WORD {
            let w = w as u16;
            present[usize::from(w) / 64] |= 1 << (w % 64);
            pairs.push((w, p.0));
        }
    }
}
