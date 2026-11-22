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
}

#[derive(Default)]
pub(super) struct Atoms {
    pub(super) list: Vec<Atom>,
    pub(super) names: Vec<String>,
    ids: HashMap<String, u32>,
    pub(super) bytes: Vec<Vec<u8>>,
    /// The atom of each byte literal.
    pub(super) byte_atom: Vec<u32>,
    pub(super) paths: Vec<regex::Regex>,
    pub(super) path_atom: Vec<u32>,
    pub(super) rows: Vec<RowTest>,
    pub(super) row_atom: Vec<u32>,
}

pub(super) type Cnf = Vec<Vec<u32>>;

pub(super) fn holds(cnf: &Cnf, bits: &[u64]) -> bool {
    cnf.iter().all(|cl| cl.iter().any(|a| funnel::bit32(bits, *a)))
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
    pub(super) plan: funnel::Plan,
}

pub(super) enum EKey {
    Keys(Vec<Key>),
    Fact(facts::FactOp),
}

pub(super) struct Group {
    pub(super) req: Cnf,
    pub(super) words: Vec<Vec<u16>>,
    pub(super) alts: Vec<Alt>,
}

pub(super) struct SRule {
    pub(super) req: Vec<Cnf>,
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
                        self.atoms.rows.push(RowTest { slot, conds });
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
        v.as_array().expect("cnf").iter().map(|cl| cl.as_array().expect("clause").iter().map(|a| self.atom(a.as_str().expect("atom"))).collect()).collect()
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
    Some(Alt { level, key, plan })
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
        let mut slots = Vec::new();
        let mut groups = Vec::new();
        let specs: Vec<(Cnf, Vec<Vec<u16>>, Vec<(u8, serde_json::Value)>)> = if census {
            r["plans"]
                .as_array()
                .expect("plans")
                .iter()
                .map(|p| {
                    let greq = if p["req"].is_array() { l.cnf(&p["req"]) } else { Vec::new() };
                    let level = if p["bucket"].as_str() == Some("words") { 0 } else { 2 };
                    (greq, l.words.cnf(&p["words"]), vec![(level, p.clone())])
                })
                .collect()
        } else {
            r["entries"]
                .as_array()
                .expect("entries")
                .iter()
                .map(|e| {
                    let alts = e["alts"].as_array().expect("alts").iter().map(|a| (a["level"].as_u64().expect("level") as u8, a.clone())).collect();
                    (l.cnf(&e["req"]), l.words.cnf(&e["words"]), alts)
                })
                .collect()
        };
        for (greq, words, alts) in specs {
            let mut fl = funnel::FactLoad { catalog: &mut *l.catalog, interner: &mut *l.interner, slots: std::mem::take(&mut slots) };
            let alts: Vec<Alt> = alts.iter().filter_map(|(level, p)| alt_of(p, *level, vocab, &mut missing, &mut fl, l.words)).collect();
            slots = fl.slots;
            if !alts.is_empty() {
                groups.push(Group { req: greq, words, alts });
            }
        }
        let mut req_slots: Vec<usize> = Vec::new();
        for c in &req {
            for cl in c {
                for a in cl {
                    if let Atom::Row(i) = l.atoms.list[*a as usize] {
                        let s = l.atoms.rows[i].slot;
                        if !req_slots.contains(&s) {
                            req_slots.push(s);
                        }
                    }
                }
            }
        }
        out.push(SRule { req, groups, slots, req_slots });
    }
    missing.sort();
    missing.dedup();
    if !missing.is_empty() {
        eprintln!("spec: names not in the vocabulary: {missing:?}");
    }
    out
}
