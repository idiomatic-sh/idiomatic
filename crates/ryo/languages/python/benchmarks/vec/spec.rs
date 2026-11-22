//! THROWAWAY (ryo-fmx3.1, spec step): the planner's derived requirements and entries (`build/spec/scripts/spec.py`): each
//! rule's `req` (per branch a CNF over file atoms) tested per file as atom bits, and each branch's `entries` (alternative
//! roots, each with its level in the fixed specificity order) chosen per file by exact bucket sizes.

use super::{Key, Vocab, facts, funnel};
use std::collections::HashMap;

/// One file atom of a requirement.
#[derive(Clone, Debug)]
pub(super) enum Atom {
    Kind(u16),
    Field(u16),
    Word(u16),
    /// A byte literal of the source (its bit set by the byte scan).
    Bytes,
    /// The file's path (relative to the root) matches (or, negated, does not match) a regex.
    Path(usize, bool),
    /// A row of a provider relation with these column tests (index into `Atoms::rows`).
    Row(usize),
    /// A name the vocabulary or catalog does not know: never tested (always present: a superset).
    Unknown,
}

/// A presence test: some row of the slot passes the conds.
pub(super) struct RowTest {
    pub(super) slot: usize,
    pub(super) conds: Vec<facts::Cond>,
    /// The `ColWant` of each cond.
    cols: Vec<usize>,
}

#[derive(Default)]
pub(super) struct Atoms {
    pub(super) list: Vec<Atom>,
    pub(super) names: Vec<String>,
    ids: HashMap<String, u32>,
    pub(super) bytes: Vec<Vec<u8>>,
    /// One searcher a byte literal (memmem-style prefilter; a literal is found once, then the search stops).
    pub(super) finders: Vec<aho_corasick::AhoCorasick>,
    /// Single-byte literals (from the file's byte set) and the others (by their searcher).
    pub(super) single: Vec<(u8, u32)>,
    pub(super) multi: Vec<usize>,
    /// The atom of each byte literal.
    pub(super) byte_atom: Vec<u32>,
    pub(super) paths: Vec<regex::Regex>,
    pub(super) path_atom: Vec<u32>,
    pub(super) rows: Vec<RowTest>,
    pub(super) row_atom: Vec<u32>,
    /// The columns the row tests read, one summary each per file (`prepare`).
    pub(super) cols: Vec<ColWant>,
    /// The pack's distinct clauses (any-of over atoms), each tested once a file.
    pub(super) clauses: Vec<Vec<u32>>,
    clause_ids: HashMap<Vec<u32>, u32>,
}

/// One (slot, column) the row tests read: the literal values they ask (each a bit of the file's summary).
pub(super) struct ColWant {
    pub(super) slot: usize,
    pub(super) col: usize,
    pub(super) wanted: HashMap<u64, usize>,
    /// The same, dense by value (index + 1; values beyond it are not wanted).
    dense: Vec<u32>,
    need_re: bool,
    need_range: bool,
}

/// A file's summary of one column: which wanted values occur.
#[derive(Default, Clone)]
pub(super) struct ColSummary {
    pub(super) present: Vec<bool>,
}

impl Atoms {
    /// The clause bits of a file from its atom bits.
    pub(super) fn clause_bits(&self, atoms: &[u64], out: &mut Vec<u64>) {
        out.clear();
        out.resize(self.clauses.len().div_ceil(64).max(1), 0);
        for (i, cl) in self.clauses.iter().enumerate() {
            if cl.iter().any(|a| funnel::bit32(atoms, *a)) {
                out[i / 64] |= 1 << (i % 64);
            }
        }
    }

    /// Groups the row tests' columns (after load).
    pub(super) fn prepare(&mut self) {
        let mut cols: Vec<ColWant> = Vec::new();
        for t in &mut self.rows {
            t.cols.clear();
            for c in &t.conds {
                let (col, vals): (usize, &[u64]) = match c {
                    facts::Cond::In(col, v) | facts::Cond::NotIn(col, v) => (*col, v.as_slice()),
                    facts::Cond::Re(col, _) | facts::Cond::Cmp(col, ..) => (*col, &[]),
                };
                let at = match cols.iter().position(|w| w.slot == t.slot && w.col == col) {
                    Some(a) => a,
                    None => {
                        cols.push(ColWant { slot: t.slot, col, wanted: HashMap::new(), dense: Vec::new(), need_re: false, need_range: false });
                        cols.len() - 1
                    }
                };
                t.cols.push(at);
                cols[at].need_re |= matches!(c, facts::Cond::Re(..));
                cols[at].need_range |= matches!(c, facts::Cond::Cmp(..));
                for v in vals {
                    let n = cols[at].wanted.len();
                    let _ = cols[at].wanted.entry(*v).or_insert(n);
                }
            }
        }
        for w in &mut cols {
            let top = w.wanted.keys().copied().max().map_or(0, |m| m as usize + 1);
            if top <= 1 << 22 {
                w.dense = vec![0; top];
                for (v, k) in &w.wanted {
                    w.dense[*v as usize] = *k as u32 + 1;
                }
            }
        }
        self.cols = cols;
    }

    /// The summaries of every wanted column of the answered slots, then each row test from them: a row of the slot
    /// whose column passes each test (tested column by column: a necessary condition of one row passing them all).
    pub(super) fn test_rows(&self, store: &facts::Store, matches: &[u128], bits: &mut [u64]) -> (u64, u64) {
        let mut sums: Vec<Option<ColSummary>> = vec![None; self.cols.len()];
        for (i, w) in self.cols.iter().enumerate() {
            if !store.answered(w.slot) {
                continue;
            }
            let mut s = ColSummary { present: vec![false; w.wanted.len()] };
            if let Some(cells) = store.cells.get(w.slot).and_then(|c| c.get(w.col)) {
                let cells = &cells[..store.rows[w.slot].min(cells.len())];
                if !w.wanted.is_empty() {
                    let mut left = w.wanted.len();
                    for &x in cells {
                        let found = if w.dense.is_empty() { w.wanted.get(&x).copied() } else { w.dense.get(x as usize).and_then(|k| k.checked_sub(1)).map(|k| k as usize) };
                        if let Some(k) = found {
                            if !s.present[k] {
                                s.present[k] = true;
                                left -= 1;
                                if left == 0 {
                                    break;
                                }
                            }
                        }
                    }
                }
                let _ = (w.need_re, w.need_range, matches);
            }
            sums[i] = Some(s);
        }
        let (mut tests, mut present) = (0u64, 0u64);
        for (ti, t) in self.rows.iter().enumerate() {
            if !store.answered(t.slot) {
                continue;
            }
            tests += 1;
            let mut ok = store.rows[t.slot] > 0;
            for (c, at) in t.conds.iter().zip(&t.cols) {
                if !ok {
                    break;
                }
                let at = *at;
                let Some(s) = &sums[at] else { continue };
                let w = &self.cols[at];
                ok = match c {
                    facts::Cond::In(_, vals) => vals.iter().any(|v| w.wanted.get(v).is_some_and(|k| s.present[*k])),
                    // some value outside the list: unless every row's value is one of them (not tracked: kept)
                    facts::Cond::NotIn(..) => true,
                    // a regex or a comparison: the column's cells until one passes (usually the first few)
                    facts::Cond::Re(col, bit) => column(store, t.slot, *col).iter().any(|x| matches.get(*x as usize).is_some_and(|m| m >> bit & 1 == 1)),
                    facts::Cond::Cmp(col, ord, or_equal, n) => column(store, t.slot, *col).iter().any(|x| {
                        let o = x.cast_signed().cmp(n);
                        o == *ord || (*or_equal && o == std::cmp::Ordering::Equal)
                    }),
                };
            }
            if ok {
                present += 1;
            } else {
                let a = self.row_atom[ti];
                bits[a as usize / 64] &= !(1u64 << (a % 64));
            }
        }
        (tests, present)
    }
}

/// A branch's requirement: the clauses (ids into `Atoms::clauses`) that must all hold.
pub(super) type Cnf = Vec<u32>;

/// Every clause holds (`bits`: the file's clause bits).
pub(super) fn holds(cnf: &Cnf, bits: &[u64]) -> bool {
    cnf.iter().all(|c| funnel::bit32(bits, *c))
}

/// Which atom kind a clause names first, for the report: the cheapest test that removes the pair.
pub(super) fn class(a: &Atom) -> &'static str {
    match a {
        Atom::Kind(_) => "kind",
        Atom::Field(_) => "field",
        Atom::Word(_) => "word",
        Atom::Bytes => "bytes",
        Atom::Path(..) => "path",
        Atom::Row(_) => "row",
        Atom::Unknown => "unknown",
    }
}

pub(super) struct Alt {
    pub(super) level: u8,
    pub(super) key: EKey,
    /// The bucket's identity (branches entered at the same one share it).
    pub(super) key_text: String,
    pub(super) plan: funnel::Plan,
}

pub(super) enum EKey {
    Keys(Vec<Key>),
    Fact(facts::FactOp),
}

pub(super) struct Group {
    pub(super) req: Cnf,
    pub(super) words: Vec<Vec<u16>>,
    /// Indices into the rule's `alts`.
    pub(super) alts: Vec<usize>,
}

pub(super) struct SRule {
    pub(super) req: Vec<Cnf>,
    /// The rule's entry alternatives, each compiled once (branches share them).
    pub(super) alts: Vec<Alt>,
    pub(super) groups: Vec<Group>,
    /// Fact slots the entries test (in their ops or as their bucket).
    pub(super) slots: Vec<usize>,
    /// Slots the requirement's row atoms read.
    pub(super) req_slots: Vec<usize>,
}

impl SRule {
    pub(super) fn alive(&self, bits: &[u64]) -> bool {
        self.req.is_empty() || self.req.iter().any(|c| holds(c, bits))
    }
}

pub(super) struct Load<'l> {
    pub(super) vocab: &'l Vocab,
    pub(super) words: &'l mut funnel::Words,
    pub(super) catalog: &'l mut facts::Catalog,
    pub(super) interner: &'l mut facts::Interner,
    pub(super) atoms: Atoms,
}

impl Load<'_> {
    fn atom(&mut self, s: &str) -> u32 {
        if let Some(i) = self.atoms.ids.get(s) {
            return *i;
        }
        let id = self.atoms.list.len() as u32;
        let (tag, rest) = s.split_at(2);
        let atom = match tag {
            "k:" => self.vocab.kinds.get(rest).map_or(Atom::Unknown, |c| Atom::Kind(*c)),
            "f:" => self.vocab.fields.get(rest).map_or(Atom::Unknown, |c| Atom::Field(*c)),
            "w:" => Atom::Word(self.words.id(rest)),
            "b:" => {
                let lit: String = serde_json::from_str(rest).expect("byte literal");
                self.atoms.bytes.push(lit.into_bytes());
                self.atoms.byte_atom.push(id);
                Atom::Bytes
            }
            "p:" | "P:" => match regex::Regex::new(rest) {
                Ok(re) => {
                    self.atoms.paths.push(re);
                    self.atoms.path_atom.push(id);
                    Atom::Path(self.atoms.paths.len() - 1, tag == "P:")
                }
                Err(_) => Atom::Unknown,
            },
            "r:" => {
                let v: serde_json::Value = serde_json::from_str(rest).expect("row atom");
                match self.catalog.slot(v[0].as_str().expect("relation")) {
                    Some(slot) => {
                        let conds = self.catalog.conds(slot, &v[1], self.interner);
                        self.atoms.rows.push(RowTest { slot, conds, cols: Vec::new() });
                        self.atoms.row_atom.push(id);
                        Atom::Row(self.atoms.rows.len() - 1)
                    }
                    None => Atom::Unknown,
                }
            }
            other => panic!("atom {other}"),
        };
        self.atoms.list.push(atom);
        self.atoms.names.push(s.to_owned());
        self.atoms.ids.insert(s.to_owned(), id);
        id
    }

    fn cnf(&mut self, v: &serde_json::Value) -> Cnf {
        let mut out: Cnf = Vec::new();
        for cl in v.as_array().expect("cnf") {
            let mut ids: Vec<u32> = cl.as_array().expect("clause").iter().map(|a| self.atom(a.as_str().expect("atom"))).collect();
            ids.sort_unstable();
            ids.dedup();
            let n = self.atoms.clauses.len() as u32;
            let id = *self.atoms.clause_ids.entry(ids.clone()).or_insert(n);
            if id == n {
                self.atoms.clauses.push(ids);
            }
            out.push(id);
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// Loads `req` and `entries` of each rule (same order as `funnel::load`). `census` entries are the rule's `plans`, one
/// group each, with their `req`.
fn alt_of(p: &serde_json::Value, level: u8, vocab: &Vocab, missing: &mut Vec<String>, fl: &mut funnel::FactLoad<'_>, words: &mut funnel::Words) -> Option<Alt> {
    let bucket = p["bucket"].as_str().expect("bucket");
    let key = if bucket == "fact" {
        let k = &p["keys"];
        let op = serde_json::json!({"rel": k["rel"], "keys": [[k["key"], 0]], "conds": k["conds"]});
        let f = fl.catalog.op(&op, fl.interner)?;
        if !fl.slots.contains(&f.slot) {
            fl.slots.push(f.slot);
        }
        EKey::Fact(f)
    } else {
        EKey::Keys(
            p["keys"]
                .as_array()
                .expect("keys")
                .iter()
                .filter_map(|k| {
                    let k = k.as_str().expect("key");
                    match bucket {
                        "words" => Some(Key::Word(words.id(k))),
                        "fields" => vocab.fields.get(k).map(|c| Key::Field(*c)),
                        _ => vocab.kinds.get(k).map(|c| Key::Kind(*c)),
                    }
                })
                .collect(),
        )
    };
    let ops = funnel::ops_of(&p["ops"], vocab, missing, fl);
    let stages = funnel::compile(&ops);
    let plan = funnel::Plan { keys: Vec::new(), ops, regs: p["regs"].as_u64().expect("regs") as usize, stages, knock: None, words: Vec::new() };
    let key_text = serde_json::to_string(&serde_json::json!([p["bucket"], p["keys"]])).expect("key");
    Some(Alt { level, key, key_text, plan })
}

/// Loads `req` and `entries` of each rule (same order as `funnel::load`). `census` entries are the rule's `plans`, one
/// group each, with their `req`.
pub(super) fn load(path: &str, census: bool, l: &mut Load<'_>) -> Vec<SRule> {
    let mut out = Vec::new();
    let mut missing = Vec::new();
    let vocab = l.vocab;
    for line in std::fs::read_to_string(path).expect("plans").lines() {
        let r: serde_json::Value = serde_json::from_str(line).expect("json");
        let req: Vec<Cnf> = r["req"].as_array().expect("req (a spec.py plan file)").iter().map(|c| l.cnf(c)).collect();
        let mut groups = Vec::new();
        // the alternatives: the shared table (`alts` + `alt_ids`, scripts/share.py), the inline form, or the census plans
        let mut table: Vec<(u8, &serde_json::Value)> = Vec::new();
        let mut specs: Vec<(Cnf, Vec<Vec<u16>>, Vec<usize>)> = Vec::new();
        if census {
            for p in r["plans"].as_array().expect("plans") {
                let greq = if p["req"].is_array() { l.cnf(&p["req"]) } else { Vec::new() };
                let level = if p["bucket"].as_str() == Some("words") { 0 } else { 2 };
                table.push((level, p));
                specs.push((greq, l.words.cnf(&p["words"]), vec![table.len() - 1]));
            }
        } else if let Some(shared) = r["alts"].as_array() {
            table = shared.iter().map(|a| (a["level"].as_u64().expect("level") as u8, a)).collect();
            for e in r["entries"].as_array().expect("entries") {
                let ids = e["alt_ids"].as_array().expect("alt ids").iter().map(|x| x.as_u64().expect("id") as usize).collect();
                specs.push((l.cnf(&e["req"]), l.words.cnf(&e["words"]), ids));
            }
        } else {
            for e in r["entries"].as_array().expect("entries") {
                let mut ids = Vec::new();
                for a in e["alts"].as_array().expect("alts") {
                    table.push((a["level"].as_u64().expect("level") as u8, a));
                    ids.push(table.len() - 1);
                }
                specs.push((l.cnf(&e["req"]), l.words.cnf(&e["words"]), ids));
            }
        }
        let mut fl = funnel::FactLoad { catalog: &mut *l.catalog, interner: &mut *l.interner, slots: Vec::new() };
        let mut alts: Vec<Alt> = Vec::new();
        let mut remap: Vec<Option<usize>> = Vec::new();
        for (level, p) in &table {
            match alt_of(p, *level, vocab, &mut missing, &mut fl, l.words) {
                Some(a) => {
                    remap.push(Some(alts.len()));
                    alts.push(a);
                }
                None => remap.push(None),
            }
        }
        let slots = fl.slots;
        for (greq, words, ids) in specs {
            let ids: Vec<usize> = ids.iter().filter_map(|i| remap[*i]).collect();
            if !ids.is_empty() {
                groups.push(Group { req: greq, words, alts: ids });
            }
        }
        let mut req_slots: Vec<usize> = Vec::new();
        for c in &req {
            for cl in c {
                for a in &l.atoms.clauses[*cl as usize] {
                    if let Atom::Row(i) = l.atoms.list[*a as usize] {
                        let s = l.atoms.rows[i].slot;
                        if !req_slots.contains(&s) {
                            req_slots.push(s);
                        }
                    }
                }
            }
        }
        out.push(SRule { req, alts, groups, slots, req_slots });
    }
    missing.sort();
    missing.dedup();
    if !missing.is_empty() {
        eprintln!("spec: names not in the vocabulary: {missing:?}");
    }
    l.atoms.prepare();
    // each branch tests its clauses in the fixed specificity order (a path, a byte literal, a word, a provider row, a
    // kind, a field: the clause ranks by its least specific atom), so a dead branch fails at its first clause
    let rank = |a: &Atom| match a {
        Atom::Path(..) => 0,
        Atom::Bytes => 1,
        Atom::Word(_) => 2,
        Atom::Row(_) => 3,
        Atom::Kind(_) => 4,
        Atom::Field(_) => 5,
        Atom::Unknown => 6,
    };
    let key: Vec<(u8, usize)> = l.atoms.clauses.iter().map(|cl| (cl.iter().map(|a| rank(&l.atoms.list[*a as usize])).max().unwrap_or(6), cl.len())).collect();
    for r in &mut out {
        for c in r.req.iter_mut().chain(r.groups.iter_mut().map(|g| &mut g.req)) {
            c.sort_by_key(|i| (key[*i as usize], *i));
        }
    }
    l.atoms.finders = l.atoms.bytes.iter().map(|b| aho_corasick::AhoCorasick::new([b]).expect("byte literal")).collect();
    for (i, b) in l.atoms.bytes.iter().enumerate() {
        if let [one] = b.as_slice() {
            l.atoms.single.push((*one, l.atoms.byte_atom[i]));
        } else {
            l.atoms.multi.push(i);
        }
    }
    out
}

fn column(store: &facts::Store, slot: usize, col: usize) -> &[u64] {
    store.cells.get(slot).and_then(|c| c.get(col)).map_or(&[], |c| &c[..store.rows[slot].min(c.len())])
}
