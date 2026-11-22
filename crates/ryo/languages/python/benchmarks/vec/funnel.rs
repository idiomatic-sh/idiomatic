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

/// One file's view: the table, the source and the columns, with the structure columns when computed.
pub(super) struct Ctx<'a> {
    pub(super) table: &'a NodeTable,
    pub(super) src: &'a [u8],
    pub(super) kind: &'a [u16],
    pub(super) field: &'a [u16],
    pub(super) start: &'a [u32],
    pub(super) end: &'a [u32],
    pub(super) parent: &'a [u32],
    /// Per element, its child sets, parent kind and root flag, as the node table wrote them.
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
    #[inline]
    fn text(&self, n: u32) -> &[u8] {
        &self.src[self.start[n as usize] as usize..self.end[n as usize] as usize]
    }
}

#[inline]
fn passes(ctx: &Ctx<'_>, c: &Check, n: u32) -> bool {
    let k = ctx.kind[n as usize];
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
        if !fbit(s, ctx.field[n as usize]) {
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
    pub(super) knock: Option<Vec<(Test, bool)>>,
    /// Required words: every clause has a word spelled in the file.
    pub(super) words: Vec<Vec<u16>>,
}

pub(super) struct FRule {
    pub(super) name: String,
    pub(super) plans: Vec<Plan>,
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

impl FRule {
    pub(super) fn words_alive(&self, present: &[u64]) -> bool {
        self.cnfs.is_empty() || self.cnfs.iter().any(|c| cnf_holds(c, present))
    }
}

fn kset_of(v: &serde_json::Value, vocab: &Vocab, missing: &mut Vec<String>) -> KSet {
    let mut s = [0u64; 4];
    for n in v.as_array().expect("kinds") {
        let n = n.as_str().expect("kind");
        match vocab.kinds.get(n) {
            Some(c) => s[usize::from(*c) / 64] |= 1 << (c % 64),
            None => missing.push(format!("kind {n}")),
        }
    }
    s
}

fn fset_of(v: &serde_json::Value, vocab: &Vocab, missing: &mut Vec<String>) -> FSet {
    let mut s = [0u64; 2];
    for n in v.as_array().expect("fields") {
        let n = n.as_str().expect("field");
        match vocab.fields.get(n) {
            Some(c) if *c < 128 => s[usize::from(*c) / 64] |= 1 << (c % 64),
            _ => missing.push(format!("field {n}")),
        }
    }
    s
}

fn ops_of(v: &serde_json::Value, vocab: &Vocab, missing: &mut Vec<String>, fc: &mut FactLoad<'_>) -> Vec<Op> {
    let u = |o: &serde_json::Value, k: &str| o[k].as_u64().expect("reg") as usize;
    v.as_array()
        .expect("ops")
        .iter()
        .filter_map(|o| {
            if o["op"].as_str() == Some("fact") {
                let op = fc.catalog.op(o, fc.interner);
                if let Some(f) = &op {
                    if !fc.slots.contains(&f.slot) {
                        fc.slots.push(f.slot);
                    }
                    if !fc.keyed.contains(&(f.slot, f.keys[0].0)) {
                        fc.keyed.push((f.slot, f.keys[0].0));
                    }
                    if (!f.conds.is_empty() || f.keys.len() > 1) && !fc.valued.contains(&f.slot) {
                        fc.valued.push(f.slot);
                    }
                }
                return op.map(Op::Fact);
            }
            Some(o)
                .map(|o| match o["op"].as_str().expect("op") {
            "check" => Op::Check(Check {
                v: u(o, "v"),
                kinds: o.get("kinds").map(|x| kset_of(x, vocab, missing)),
                notkinds: o.get("notkinds").map(|x| kset_of(x, vocab, missing)),
                texts: o.get("texts").map(|x| x.as_array().expect("texts").iter().map(|t| t.as_str().expect("text").as_bytes().to_vec()).collect()),
                fields: o.get("fields").map(|x| fset_of(x, vocab, missing)),
                re: o.get("re").map(|x| regex::bytes::Regex::new(x.as_str().expect("re")).expect("regex")),
            }),
            "up" => Op::Up { from: u(o, "from"), to: u(o, "to"), leaf: o["leaf"].as_bool().unwrap_or(false) },
            "down" => Op::Down { from: u(o, "from"), to: u(o, "to"), leaf: o["leaf"].as_bool().unwrap_or(false) },
            "not" => Op::Not(ops_of(&o["plan"], vocab, missing, fc)),
            "exists" => Op::Exists { from: u(o, "from"), to: u(o, "to"), plan: ops_of(&o["plan"], vocab, missing, fc) },
            "isparent" => Op::IsParent { child: u(o, "child"), parent: u(o, "parent") },
            "enc" => {
                let set = kset_of(&o["kinds"], vocab, missing);
                let at = fc.catalog.enc_sets.iter().position(|x| *x == set).unwrap_or_else(|| {
                    fc.catalog.enc_sets.push(set);
                    fc.catalog.enc_sets.len() - 1
                });
                Op::Enc { from: u(o, "from"), to: u(o, "to"), set: at }
            }
            "ancx" => Op::AncX { from: u(o, "from"), to: u(o, "to"), plan: ops_of(&o["plan"], vocab, missing, fc) },
            other => panic!("op {other}"),
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
    fn cnf(&mut self, v: &serde_json::Value) -> Vec<Vec<u16>> {
        v.as_array()
            .map(|cl| cl.iter().map(|c| c.as_array().expect("clause").iter().map(|w| self.id(w.as_str().expect("word"))).collect()).collect())
            .unwrap_or_default()
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

pub(super) fn load(path: &str, vocab: &Vocab, catalog: &mut super::facts::Catalog, interner: &mut super::facts::Interner) -> (Vec<FRule>, Words) {
    let mut fc = FactLoad { catalog, interner, slots: Vec::new(), valued: Vec::new(), keyed: Vec::new() };
    let mut missing = Vec::new();
    let mut rules = Vec::new();
    let mut words = Words::default();
    for line in std::fs::read_to_string(path).expect("funnel plans").lines() {
        let r: serde_json::Value = serde_json::from_str(line).expect("json");
        let mut plans = Vec::new();
        for p in r["plans"].as_array().expect("plans") {
            let fields = p["bucket"].as_str() == Some("fields");
            let by_word = p["bucket"].as_str() == Some("words");
            let keys: Vec<Key> = p["keys"]
                .as_array()
                .expect("keys")
                .iter()
                .filter_map(|k| {
                    let k = k.as_str().expect("key");
                    let code = if by_word {
                        Some(Key::Word(words.id(k)))
                    } else if fields {
                        vocab.fields.get(k).map(|c| Key::Field(*c))
                    } else {
                        vocab.kinds.get(k).map(|c| Key::Kind(*c))
                    };
                    if code.is_none() {
                        missing.push(format!("bucket {k}"));
                    }
                    code
                })
                .collect();
            let ops = ops_of(&p["ops"], vocab, &mut missing, &mut fc);
            let stages = compile(&ops);
            let knock = knock_tests(&ops);
            let cnf = words.cnf(&p["words"]);
            plans.push(Plan { keys, ops, regs: p["regs"].as_u64().expect("regs") as usize, stages, knock, words: cnf });
        }
        let cnfs: Vec<Vec<Vec<u16>>> = r["word_cnfs"].as_array().map(|a| a.iter().map(|c| words.cnf(c)).collect()).unwrap_or_default();
        let slots = std::mem::take(&mut fc.slots);
        let valued = std::mem::take(&mut fc.valued);
        let keyed = std::mem::take(&mut fc.keyed);
        rules.push(FRule { name: r["rule"].as_str().expect("rule").to_owned(), plans, cnfs, slots, valued, keyed });
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
            let p = ctx.parent[n as usize];
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
            ctx.parent[c as usize] == regs[*parent] && ctx.parent[c as usize] != c && exec(ctx, ops, pc + 1, regs, steps)
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
                let p = ctx.parent[n as usize];
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
/// Rows of registers (stride `r`) with the index of the row they came from (for a negation's sub-run).
#[derive(Default)]
pub(super) struct Rows {
    pub(super) r: usize,
    pub(super) d: Vec<u32>,
    pub(super) tag: Vec<u32>,
}

impl Rows {
    pub(super) fn len(&self) -> usize {
        self.tag.len()
    }
    #[inline]
    fn retain<F: Fn(&[u32]) -> bool>(&mut self, f: F) {
        let r = self.r;
        let mut w = 0;
        for i in 0..self.tag.len() {
            if f(&self.d[i * r..i * r + r]) {
                if w != i {
                    self.d.copy_within(i * r..i * r + r, w * r);
                    self.tag[w] = self.tag[i];
                }
                w += 1;
            }
        }
        self.d.truncate(w * r);
        self.tag.truncate(w);
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

fn check_stages(c: &Check, out: &mut Vec<Stage>) {
    let v = c.v;
    if let Some(s) = c.kinds {
        let ones: Vec<u16> = (0..256u16).filter(|k| kbit(&s, *k)).collect();
        if ones.len() == 1 {
            let k = ones[0];
            out.push(filt(move |ctx, row| ctx.kind[row[v] as usize] == k));
        } else {
            out.push(filt(move |ctx, row| kbit(&s, ctx.kind[row[v] as usize])));
        }
    }
    if let Some(s) = c.notkinds {
        out.push(filt(move |ctx, row| !kbit(&s, ctx.kind[row[v] as usize])));
    }
    if let Some(s) = c.fields {
        let ones: Vec<u16> = (0..128u16).filter(|f| fbit(&s, *f)).collect();
        if ones.len() == 1 {
            let f = ones[0];
            out.push(filt(move |ctx, row| ctx.field[row[v] as usize] == f));
        } else {
            out.push(filt(move |ctx, row| fbit(&s, ctx.field[row[v] as usize])));
        }
    }
    if let Some(t) = &c.texts {
        if t.len() == 1 {
            let t = t[0].clone();
            out.push(filt(move |ctx, row| ctx.text(row[v]) == t.as_slice()));
        } else {
            let t = t.clone();
            out.push(filt(move |ctx, row| {
                let x = ctx.text(row[v]);
                t.iter().any(|y| y.as_slice() == x)
            }));
        }
    }
    if let Some(re) = &c.re {
        let re = re.clone();
        out.push(filt(move |ctx, row| re.is_match(ctx.text(row[v]))));
    }
}

/// Compiles a plan into stages, fusing a leaf `up` + kind check into a parent-kind test, a leaf `down` + kind check into a
/// child-kind-bitset test, and any other leaf `down` + check into one scan of the children.
pub(super) fn compile(ops: &[Op]) -> Vec<Stage> {
    let mut out: Vec<Stage> = Vec::new();
    let mut i = 0;
    while i < ops.len() {
        let next_check = |to: usize| match ops.get(i + 1) {
            Some(Op::Check(c)) if c.v == to => Some(c.clone()),
            _ => None,
        };
        match &ops[i] {
            Op::Check(c) => check_stages(c, &mut out),
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
                    _ if *leaf && next_check(*to).is_none() => out.push(filt(move |ctx, row| ctx.parent[row[from] as usize] != row[from])),
                    _ => {
                        let to = *to;
                        out.push(Box::new(move |ctx, rows, steps| {
                            *steps += rows.len() as u64;
                            let r = rows.r;
                            let mut w = 0;
                            for j in 0..rows.tag.len() {
                                let n = rows.d[j * r + from];
                                let p = ctx.parent[n as usize];
                                if p != n {
                                    if w != j {
                                        rows.d.copy_within(j * r..j * r + r, w * r);
                                        rows.tag[w] = rows.tag[j];
                                    }
                                    rows.d[w * r + to] = p;
                                    w += 1;
                                }
                            }
                            rows.d.truncate(w * r);
                            rows.tag.truncate(w);
                        }));
                    }
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
                        out.push(filt(move |ctx, row| ctx.children(row[from]).iter().any(|x| passes(ctx, &c, x.0))));
                        i += 1;
                    }
                    None if *leaf => out.push(filt(move |ctx, row| !ctx.children(row[from]).is_empty())),
                    _ => out.push(expand(from, *to)),
                }
            }
            Op::Not(sub) => out.push(pred_stage(compile_pred(&[Op::Not(sub.clone())]))),
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
                    [Op::Check(c)] if c.v == to => {
                        let c = c.clone();
                        out.push(filt(move |ctx, row| ctx.children(row[from]).iter().any(|x| passes(ctx, &c, x.0))));
                    }
                    _ => out.push(pred_stage(compile_pred(&[Op::Exists { from, to, plan: plan.clone() }]))),
                }
            }
            Op::Fact(f) => {
                let f = f.clone();
                out.push(filt(move |ctx, row| ctx.facts.holds(&f, row, ctx.matches)));
            }
            Op::Enc { from, to, set } => {
                let (from, to, set) = (*from, *to, *set);
                out.push(Box::new(move |ctx, rows, steps| {
                    *steps += rows.len() as u64;
                    let r = rows.r;
                    let mut w = 0;
                    for j in 0..rows.tag.len() {
                        let e = ctx.enc[set][rows.d[j * r + from] as usize];
                        if e != u32::MAX {
                            if w != j {
                                rows.d.copy_within(j * r..j * r + r, w * r);
                                rows.tag[w] = rows.tag[j];
                            }
                            rows.d[w * r + to] = e;
                            w += 1;
                        }
                    }
                    rows.d.truncate(w * r);
                    rows.tag.truncate(w);
                }));
            }
            Op::AncX { .. } => out.push(pred_stage(compile_pred(std::slice::from_ref(&ops[i])))),
            Op::IsParent { child, parent } => {
                let (c, p) = (*child, *parent);
                out.push(filt(move |ctx, row| {
                    let n = row[c] as usize;
                    ctx.parent[n] == row[p] && ctx.parent[n] as usize != n
                }));
            }
        }
        i += 1;
    }
    out
}

/// A load-time specialised predicate over registers: one closure per operation and column type, chained in continuation
/// style (a `down` tries each child against the rest of the chain).
pub(super) type Pred = Box<dyn for<'c> Fn(&Ctx<'c>, &mut [u32], &mut u64) -> bool>;

fn check_pred(c: &Check, next: Pred) -> Pred {
    let v = c.v;
    let mut k = next;
    if let Some(re) = &c.re {
        let re = re.clone();
        k = Box::new(move |ctx, r, s| re.is_match(ctx.text(r[v])) && k(ctx, r, s));
    }
    if let Some(t) = &c.texts {
        if t.len() == 1 {
            let t = t[0].clone();
            k = Box::new(move |ctx, r, s| ctx.text(r[v]) == t.as_slice() && k(ctx, r, s));
        } else {
            let t = t.clone();
            k = Box::new(move |ctx, r, s| {
                let x = ctx.text(r[v]);
                t.iter().any(|y| y.as_slice() == x) && k(ctx, r, s)
            });
        }
    }
    if let Some(f) = c.fields {
        let ones: Vec<u16> = (0..128u16).filter(|x| fbit(&f, *x)).collect();
        if ones.len() == 1 {
            let one = ones[0];
            k = Box::new(move |ctx, r, s| ctx.field[r[v] as usize] == one && k(ctx, r, s));
        } else {
            k = Box::new(move |ctx, r, s| fbit(&f, ctx.field[r[v] as usize]) && k(ctx, r, s));
        }
    }
    if c.kinds.is_some() || c.notkinds.is_some() {
        let set = c.kind_set();
        let ones: Vec<u16> = (0..256u16).filter(|x| kbit(&set, *x)).collect();
        if ones.len() == 1 {
            let one = ones[0];
            k = Box::new(move |ctx, r, s| ctx.kind[r[v] as usize] == one && k(ctx, r, s));
        } else {
            k = Box::new(move |ctx, r, s| kbit(&set, ctx.kind[r[v] as usize]) && k(ctx, r, s));
        }
    }
    Box::new(move |ctx, r, s| {
        *s += 1;
        k(ctx, r, s)
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
        }
        let next = k;
        k = match &ops[i] {
            Op::Check(c) => check_pred(c, next),
            Op::Up { from, to, .. } => {
                let (from, to) = (*from, *to);
                Box::new(move |ctx, r, s| {
                    *s += 1;
                    let n = r[from];
                    let p = ctx.parent[n as usize];
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
                        let p = ctx.parent[n as usize];
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
                    ctx.parent[n] == r[p] && ctx.parent[n] as usize != n && next(ctx, r, s)
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
        for j in 0..rows.tag.len() {
            regs[..r].copy_from_slice(&rows.d[j * r..j * r + r]);
            if pred(ctx, &mut regs, steps) {
                if w != j {
                    rows.d.copy_within(j * r..j * r + r, w * r);
                    rows.tag[w] = rows.tag[j];
                }
                w += 1;
            }
        }
        rows.d.truncate(w * r);
        rows.tag.truncate(w);
    })
}

fn expand(from: usize, to: usize) -> Stage {
    Box::new(move |ctx, rows, steps| {
        *steps += rows.len() as u64;
        let r = rows.r;
        let mut d = Vec::with_capacity(rows.d.len());
        let mut tag = Vec::with_capacity(rows.tag.len());
        for j in 0..rows.tag.len() {
            let row = &rows.d[j * r..j * r + r];
            for c in ctx.children(row[from]) {
                d.extend_from_slice(row);
                let at = d.len() - r + to;
                d[at] = c.0;
                tag.push(rows.tag[j]);
            }
        }
        rows.d = d;
        rows.tag = tag;
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
    #[inline]
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

/// The file's required words from its name tokens, in id order: the present bits and the (word, token id) pairs. A token the
/// walk gave a code (`codes`: the file's own tree, in id order) is looked up once a code through `memo`; every other (an
/// embedded segment's, or all of a file whose bytes were no text) by its bytes.
#[expect(clippy::too_many_arguments, reason = "throwaway prototype")]
pub(super) fn file_words(
    table: &NodeTable,
    src: &[u8],
    name_token: u16,
    codes: &[u32],
    words: &Words,
    memo: &mut Vec<u32>,
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
                let c = c as usize;
                if c >= memo.len() {
                    memo.resize(c + 1, UNSEEN);
                }
                if memo[c] == UNSEEN {
                    memo[c] = words.ids.get(text(i)).map_or(NO_WORD, |w| u32::from(*w));
                }
                memo[c]
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
