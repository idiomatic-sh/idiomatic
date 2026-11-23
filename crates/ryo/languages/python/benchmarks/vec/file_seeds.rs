//! Evaluate a file seed as an existential query. A disjunct enters at one of its
//! necessary roots. Only disjuncts that reach a fact-dependent stage ask for
//! columns, and one witness decides all disjuncts of a rule for this file.
use super::Key;
use super::facts::FactOp;
use super::funnel::{Ctx, FRule, Op, Plan, Rows, Stage, cnf_holds};
use ryo::languages::{FieldCode, KindBits, NodeTable, Position, node_table};
use std::collections::HashMap;

pub(super) fn visits_facts(op: &Op, visit: &mut impl FnMut(&FactOp)) {
    match op {
        Op::Fact(f) => visit(f),
        Op::Not(body) | Op::Exists { plan: body, .. } | Op::AncX { plan: body, .. } => {
            for child in body {
                visits_facts(child, visit);
            }
        }
        _ => {}
    }
}

pub(super) struct Index<'a> {
    pub(super) table: &'a NodeTable,
    pub(super) offsets: &'a [u32],
    pub(super) words: &'a [u32],
}

enum Bucket<'a> {
    Positions(&'a [Position]),
    Words(&'a [u32]),
}

impl Index<'_> {
    fn get(&self, key: Key) -> Bucket<'_> {
        match key {
            Key::Word(w) => {
                let w = usize::from(w);
                Bucket::Words(&self.words[self.offsets[w] as usize..self.offsets[w + 1] as usize])
            }
            Key::Kind(k) => Bucket::Positions(node_table::ids_of_kind(self.table, KindBits(k))),
            Key::Field(f) => Bucket::Positions(node_table::ids_of_field(self.table, FieldCode(f))),
        }
    }
}

impl Bucket<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Positions(ids) => ids.len(),
            Self::Words(ids) => ids.len(),
        }
    }

    fn enter(&self, rows: &mut Rows, r: usize, at: usize, len: usize) {
        match self {
            Self::Words(ids) => rows.enter(r, &ids[at..at + len]),
            Self::Positions(ids) => {
                rows.r = r;
                rows.d.clear();
                if r == 1 {
                    rows.d.extend(ids[at..at + len].iter().map(|p| p.0));
                } else {
                    rows.d.resize(len * r, 0);
                    for (row, id) in rows.d.chunks_exact_mut(r).zip(&ids[at..at + len]) {
                        row[0] = id.0;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Request {
    slot: usize,
    key: usize,
    values: bool,
}

struct Entry {
    syntax: std::cell::OnceCell<Vec<Stage>>,
    buckets: usize,
    reads: Vec<Request>,
}

struct Disjunct {
    /// The alternatives of disjuncts sharing this sequence of bucket sets all
    /// choose the same offset. Plan numbers and predicates remain rule-local.
    choice: usize,
    plans: Vec<usize>,
}

struct Rule {
    disjuncts: Vec<Disjunct>,
    entries: Vec<Entry>,
}

/// Which bucket and candidate a deferred check first reached.
#[derive(Default, Clone, Copy)]
struct Cursor {
    bucket: usize,
    start: usize,
}

#[derive(Clone, Copy)]
struct Pending {
    rule: usize,
    plan: usize,
    cursor: Cursor,
}

#[derive(Default)]
pub(super) struct Counts {
    pub(super) candidates: u64,
    pub(super) steps: u64,
    pub(super) witnesses: u64,
    pub(super) elements: u64,
    pub(super) deferred: u64,
}

pub(super) struct FileSeeds {
    syntax_only: bool,
    rules: Vec<Rule>,
    sets: Vec<Vec<Key>>,
    sizes: Vec<usize>,
    alternatives: Vec<Vec<usize>>,
    choices: Vec<Option<usize>>,
    pending: Vec<Pending>,
    rows: Rows,
    pub(super) counts: Counts,
}

impl FileSeeds {
    pub(super) fn new(rules: &[FRule], syntax_only: bool) -> Self {
        let mut sets = Vec::new();
        let mut by_set = HashMap::new();
        let mut alternatives = Vec::new();
        let mut by_alternatives = HashMap::new();
        let rules = rules
            .iter()
            .map(|rule| {
                let entries: Vec<Entry> = rule
                    .plans
                    .iter()
                    .map(|plan| {
                        let buckets = *by_set.entry(plan.keys.as_slice()).or_insert_with(|| {
                            let index = sets.len();
                            sets.push(plan.keys.clone());
                            index
                        });
                        let mut reads: Vec<Request> = Vec::new();
                        for op in &plan.ops {
                            visits_facts(op, &mut |f| {
                                let key = f.keys[0].0;
                                let values = !f.conds.is_empty() || f.keys.len() > 1;
                                if let Some(r) =
                                    reads.iter_mut().find(|r| r.slot == f.slot && r.key == key)
                                {
                                    r.values |= values;
                                } else {
                                    reads.push(Request {
                                        slot: f.slot,
                                        key,
                                        values,
                                    });
                                }
                            });
                        }
                        Entry {
                            buckets,
                            reads,
                            syntax: std::cell::OnceCell::new(),
                        }
                    })
                    .collect();
                let mut disjuncts = rule.alts.clone();
                let mut grouped = vec![false; rule.plans.len()];
                for group in &disjuncts {
                    for p in group {
                        grouped[*p] = true;
                    }
                }
                for (p, seen) in grouped.into_iter().enumerate() {
                    if !seen {
                        disjuncts.push(vec![p]);
                    }
                }
                let disjuncts = disjuncts
                    .into_iter()
                    .map(|plans| {
                        let buckets = plans
                            .iter()
                            .map(|&p| entries[p].buckets)
                            .collect::<Vec<_>>();
                        let choice = *by_alternatives.entry(buckets.clone()).or_insert_with(|| {
                            let index = alternatives.len();
                            alternatives.push(buckets);
                            index
                        });
                        Disjunct { choice, plans }
                    })
                    .collect();
                Rule { disjuncts, entries }
            })
            .collect();
        let sizes = vec![0; sets.len()];
        Self {
            syntax_only,
            rules,
            sets,
            sizes,
            choices: vec![None; alternatives.len()],
            alternatives,
            pending: Vec::new(),
            rows: Rows::default(),
            counts: Counts::default(),
        }
    }

    /// Use precisely the exhaustive evaluator's tie order. The cardinality of
    /// a given set of entry buckets is the same for every occurrence in a file.
    pub(super) fn choose(&mut self, index: &Index<'_>) {
        self.counts.elements += node_table::element_count(index.table).0 as u64;
        for (size, set) in self.sizes.iter_mut().zip(&self.sets) {
            *size = set.iter().map(|k| index.get(*k).len()).sum();
        }
        for (winner, options) in self.choices.iter_mut().zip(&self.alternatives) {
            let mut smallest = usize::MAX;
            *winner = None;
            for (offset, &set) in options.iter().enumerate() {
                let n = self.sizes[set];
                if n == 0 {
                    *winner = None;
                    break;
                }
                if n < smallest {
                    smallest = n;
                    *winner = Some(offset);
                }
            }
        }
    }

    pub(super) fn probe(
        &mut self,
        rules: &[FRule],
        index: &Index<'_>,
        ctx: &Ctx<'_>,
        alive: &[bool],
        words: &[u64],
        seeded: &mut [bool],
    ) {
        self.pending.clear();
        for (ri, rule) in rules.iter().enumerate() {
            seeded[ri] = alive[ri] && !rule.node;
            if !alive[ri] || !rule.node {
                continue;
            }
            for d in &self.rules[ri].disjuncts {
                let Some(offset) = self.choices[d.choice] else {
                    continue;
                };
                let pi = d.plans[offset];
                if !cnf_holds(&rule.plans[pi].words, words) {
                    continue;
                }
                let p = &rules[ri].plans[pi];
                let entry = &self.rules[ri].entries[pi];
                let stages = if self.syntax_only && !entry.reads.is_empty() {
                    entry.syntax.get_or_init(|| syntax_stages(p)).as_slice()
                } else {
                    &p.stages[..p.before_facts]
                };
                if let Some(cursor) = witness(
                    p,
                    index,
                    ctx,
                    stages,
                    Cursor::default(),
                    &mut self.rows,
                    &mut self.counts,
                ) {
                    if self.syntax_only || p.before_facts == p.stages.len() {
                        seeded[ri] = true;
                        self.counts.witnesses += 1;
                        break;
                    } else {
                        self.pending.push(Pending {
                            rule: ri,
                            plan: pi,
                            cursor,
                        });
                    }
                }
            }
        }
        self.pending.retain(|p| !seeded[p.rule]);
        self.counts.deferred += self.pending.len() as u64;
    }

    pub(super) fn requests(
        &self,
        needed: &mut [bool],
        valued: &mut [bool],
        keyed: &mut Vec<(usize, usize)>,
    ) {
        for p in &self.pending {
            for r in &self.rules[p.rule].entries[p.plan].reads {
                needed[r.slot] = true;
                valued[r.slot] |= r.values;
                if !keyed.contains(&(r.slot, r.key)) {
                    keyed.push((r.slot, r.key));
                }
            }
        }
    }

    pub(super) fn resolve(
        &mut self,
        rules: &[FRule],
        index: &Index<'_>,
        ctx: &Ctx<'_>,
        seeded: &mut [bool],
    ) {
        for p in &self.pending {
            if seeded[p.rule] {
                continue;
            }
            let plan = &rules[p.rule].plans[p.plan];
            if witness(
                plan,
                index,
                ctx,
                &plan.stages,
                p.cursor,
                &mut self.rows,
                &mut self.counts,
            )
            .is_some()
            {
                seeded[p.rule] = true;
                self.counts.witnesses += 1;
            }
        }
    }
}

/// A failed prefix rules out the whole batch. When later stages need facts,
/// resume at the start of the first possible batch so every candidate in it
/// still gets its exact complete check. Geometric batches find early witnesses
/// cheaply while amortizing kernel dispatch when the bucket has no witness.
fn witness(
    plan: &Plan,
    index: &Index<'_>,
    ctx: &Ctx<'_>,
    stages: &[Stage],
    from: Cursor,
    rows: &mut Rows,
    counts: &mut Counts,
) -> Option<Cursor> {
    for (k, key) in plan.keys.iter().enumerate().skip(from.bucket) {
        let ids = index.get(*key);
        let mut at = if k == from.bucket { from.start } else { 0 };
        let mut batch: usize = 1;
        while at < ids.len() {
            let cursor = Cursor {
                bucket: k,
                start: at,
            };
            if stages.is_empty() {
                counts.candidates += 1;
                return Some(cursor);
            }
            let len = batch.min(ids.len() - at);
            ids.enter(rows, plan.regs.max(1), at, len);
            counts.candidates += len as u64;
            for stage in stages {
                stage(ctx, rows, &mut counts.steps);
                if rows.d.is_empty() {
                    break;
                }
            }
            if !rows.d.is_empty() {
                return Some(cursor);
            }
            at += len;
            batch = batch.saturating_mul(2);
        }
    }
    None
}

/// Necessary conditions of a program over syntax alone. A top-level fact is a
/// predicate with no register writes, so removing it keeps every successful
/// binding. Stop before a fact-dependent block: negation reverses inclusion,
/// and a nested search can bind registers used by the continuation.
fn syntax_stages(plan: &Plan) -> Vec<Stage> {
    let mut stages = Vec::new();
    let mut begin = 0;
    for (i, op) in plan.ops.iter().enumerate() {
        let mut depends = false;
        visits_facts(op, &mut |_| depends = true);
        if !depends {
            continue;
        }
        // Preserve the original stage boundaries. In particular, never fuse
        // a register move with a predicate across an omitted fact.
        stages.extend(super::funnel::compile(&plan.ops[begin..i]).0);
        if !matches!(op, Op::Fact(_)) {
            return stages;
        }
        begin = i + 1;
    }
    stages.extend(super::funnel::compile(&plan.ops[begin..]).0);
    stages
}
