//! THROWAWAY (ryo-fmx3.1, step 4): the provider's derived facts written once a file as per-node columns. Each fact relation a
//! plan tests is filled through the walk's fillers (the provider's model is the source of truth); its rows are kept as flat
//! columns (texts interned into one run-wide id space, nodes as positions) with an index from a key node to its rows, so a
//! plan's test "this node has a row of R whose columns hold these values" is an index read and a few compares.

use ryo::languages::{self, CellKind, FilledColumns, Relation};
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::service as catalog;
use std::collections::HashMap;
use super::finish::{TokenEvent, TokenOrder};

/// The run's interned texts: an id a text, and for each id the regexes of the pack it matches (one bit a regex).
/// The run-wide text table, by the hash chosen on the command line.
pub(super) enum Table {
    Std(HashMap<Vec<u8>, u32>),
    Fx(rustc_hash::FxHashMap<Vec<u8>, u32>),
    Fold(HashMap<Vec<u8>, u32, foldhash::fast::RandomState>),
}

impl Default for Table {
    fn default() -> Self {
        Table::Std(HashMap::new())
    }
}

impl Table {
    pub(super) fn named(name: &str) -> Self {
        match name {
            "fx" => Table::Fx(rustc_hash::FxHashMap::default()),
            "fold" => Table::Fold(HashMap::with_hasher(foldhash::fast::RandomState::default())),
            _ => Table::Std(HashMap::new()),
        }
    }
    pub(super) fn get(&self, k: &[u8]) -> Option<u32> {
        match self {
            Table::Std(m) => m.get(k).copied(),
            Table::Fx(m) => m.get(k).copied(),
            Table::Fold(m) => m.get(k).copied(),
        }
    }
    fn insert(&mut self, k: Vec<u8>, v: u32) {
        let _ = match self {
            Table::Std(m) => m.insert(k, v),
            Table::Fx(m) => m.insert(k, v),
            Table::Fold(m) => m.insert(k, v),
        };
    }
    fn pairs(&self) -> Vec<(Vec<u8>, u32)> {
        match self {
            Table::Std(m) => m.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            Table::Fx(m) => m.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            Table::Fold(m) => m.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        }
    }
}

#[derive(Default)]
pub(super) struct Interner {
    pub(super) ids: Table,
    /// Text lookups and inserts made (each a hash of the text).
    pub(super) lookups: u64,
    pub(super) matches: Vec<u128>,
    pub(super) matched: Vec<bool>,
    pub(super) regexes: Vec<regex::bytes::Regex>,
    /// The texts by id (cols prototype: regex bits are computed after a direct fill, from the id alone).
    pub(super) arena: Vec<u8>,
    pub(super) ends: Vec<u32>,
    /// All the regexes as one set (cols prototype, `--regex-set`): one pass a text for every regex.
    pub(super) set: Option<regex::bytes::RegexSet>,
}

impl ryo_python::fixtures::columns::TextIds for Interner {
    #[inline]
    fn id(&mut self, text: &[u8]) -> u32 {
        Interner::id(self, text)
    }
}

impl Interner {
    pub(super) fn id(&mut self, text: &[u8]) -> u32 {
        self.lookups += 1;
        if let Some(i) = self.ids.get(text) {
            return i;
        }
        let i = self.matches.len() as u32;
        self.matches.push(0);
        self.matched.push(false);
        self.ids.insert(text.to_vec(), i);
        self.arena.extend_from_slice(text);
        self.ends.push(self.arena.len() as u32);
        i
    }

    /// Computes the regex bits of `id` when not yet computed (cols prototype).
    #[inline]
    pub(super) fn ensure_matched(&mut self, id: u32) {
        let i = id as usize;
        if self.matched.get(i).copied().unwrap_or(true) {
            return;
        }
        let start = if i == 0 { 0 } else { self.ends[i - 1] as usize };
        let text = &self.arena[start..self.ends[i] as usize];
        let mut bits = 0u128;
        if let Some(set) = &self.set {
            for r in set.matches(text).iter() {
                bits |= 1 << r;
            }
        } else {
            for (r, re) in self.regexes.iter().enumerate() {
                if re.is_match(text) {
                    bits |= 1 << r;
                }
            }
        }
        self.matches[i] = bits;
        self.matched[i] = true;
    }

    /// The id of a text of a column some regex tests: its regex bits are computed once, the first time.
    pub(super) fn id_matched(&mut self, text: &[u8]) -> u32 {
        let i = self.id(text);
        if !self.matched[i as usize] {
            let mut bits = 0u128;
            if let Some(set) = &self.set {
                for r in set.matches(text).iter() {
                    bits |= 1 << r;
                }
            } else {
                for (r, re) in self.regexes.iter().enumerate() {
                    if re.is_match(text) {
                        bits |= 1 << r;
                    }
                }
            }
            self.matches[i as usize] = bits;
            self.matched[i as usize] = true;
        }
        i
    }
    pub(super) fn is_matched(&self, id: usize) -> bool {
        self.matched.get(id).copied().unwrap_or(false)
    }

    /// A regex of the pack, by its number: each text interned before and after is tested against it once.
    pub(super) fn regex(&mut self, pattern: &str) -> usize {
        if let Some(at) = self.regexes.iter().position(|r| r.as_str() == pattern) {
            return at;
        }
        let re = regex::bytes::Regex::new(pattern).expect("regex");
        let at = self.regexes.len();
        assert!(at < 128, "at most 128 regexes");
        let _ = self.ids.pairs().len();
        for m in &mut self.matched {
            *m = false;
        }
        self.regexes.push(re);
        at
    }
}

#[derive(Clone, Debug)]
pub(super) enum Cond {
    In(usize, Vec<u64>),
    NotIn(usize, Vec<u64>),
    Re(usize, usize),
    Cmp(usize, std::cmp::Ordering, bool, i64),
}

/// A plan's fact test: a row of `slot`'s relation whose key columns hold the registers' nodes and whose columns pass `conds`.
#[derive(Clone, Debug)]
pub(super) struct FactOp {
    pub(super) slot: usize,
    pub(super) keys: Vec<(usize, usize)>,
    pub(super) conds: Vec<Cond>,
}

/// The fact relations the plans test, numbered by slot.
#[derive(Default)]
pub(super) struct Catalog {
    pub(super) relations: Vec<Relation>,
    pub(super) names: Vec<String>,
    kinds: Vec<&'static [CellKind]>,
    columns: Vec<Vec<String>>,
    /// The key columns each slot is entered by.
    pub(super) key_columns: Vec<Vec<usize>>,
    /// Per slot, the columns a regex tests: their texts get regex bits.
    pub(super) re_columns: Vec<Vec<usize>>,
    /// The enclosing kind sets the plans ask (step 5).
    pub(super) enc_sets: Vec<[u64; 4]>,
    /// Per slot, the columns some test reads beside its first key, one bit a catalog place (cols prototype).
    pub(super) read_columns: Vec<u64>,
}

impl Catalog {
    pub(super) fn column_name(&self, slot: usize, at: usize) -> String {
        self.columns[slot][at].clone()
    }

    pub(super) fn column(&self, slot: usize, name: &str) -> usize {
        self.columns[slot].iter().position(|c| c == name).expect("column")
    }

    /// A fact test of the spec: `keys` its (key column, register) pairs, `conds` its (column, op, values).
    pub(super) fn op_of(&mut self, name: &str, keys_in: &[(String, usize)], conds_in: &[(String, String, Vec<String>)], interner: &mut Interner) -> Option<FactOp> {
        let slot = match self.names.iter().position(|n| n == name) {
            Some(s) => s,
            None => {
                let relation = catalog::relation_named(NameRef(name))?;
                self.relations.push(relation);
                self.names.push(name.to_owned());
                self.kinds.push(catalog::kinds(relation));
                self.columns.push(catalog::columns(relation).into_iter().map(|c| catalog::column_name(c).0.to_owned()).collect());
                self.key_columns.push(Vec::new());
                self.re_columns.push(Vec::new());
                self.read_columns.push(0);
                self.names.len() - 1
            }
        };
        let col = |c: &str| self.columns[slot].iter().position(|x| x == c);
        let mut keys = Vec::new();
        for (k, r) in keys_in {
            let c = col(k)?;
            if self.kinds[slot][c] != CellKind::Node {
                return None;
            }
            keys.push((c, *r));
        }
        let mut conds = Vec::new();
        for (c, op, vals) in conds_in {
            let Some(at) = col(c) else { continue };
            let kind = self.kinds[slot][at];
            let op = op.as_str();
            let vals: Vec<&str> = vals.iter().map(String::as_str).collect();
            match op {
                "eq" | "ne" => {
                    let parsed: Option<Vec<u64>> = vals.iter().map(|x| literal(kind, x, interner)).collect();
                    // a literal the column cannot hold leaves the test out (a superset)
                    if let Some(p) = parsed {
                        conds.push(if op == "eq" { Cond::In(at, p) } else { Cond::NotIn(at, p) });
                    }
                }
                "re" if kind == CellKind::Symbol => {
                    if !self.re_columns[slot].contains(&at) {
                        self.re_columns[slot].push(at);
                    }
                    conds.push(Cond::Re(at, interner.regex(vals[0])));
                }
                "<" | "<=" | ">" | ">=" if kind == CellKind::Number => {
                    if let Ok(n) = vals[0].parse::<i64>() {
                        let (ord, or_equal) = match op {
                            "<" => (std::cmp::Ordering::Less, false),
                            "<=" => (std::cmp::Ordering::Less, true),
                            ">" => (std::cmp::Ordering::Greater, false),
                            _ => (std::cmp::Ordering::Greater, true),
                        };
                        conds.push(Cond::Cmp(at, ord, or_equal, n));
                    }
                }
                _ => {}
            }
        }
        if !self.key_columns[slot].contains(&keys[0].0) {
            self.key_columns[slot].push(keys[0].0);
        }
        for (c, _) in &keys[1..] {
            self.read_columns[slot] |= 1 << c;
        }
        for c in &conds {
            let (Cond::In(at, _) | Cond::NotIn(at, _) | Cond::Re(at, _) | Cond::Cmp(at, ..)) = c;
            self.read_columns[slot] |= 1 << at;
        }
        Some(FactOp { slot, keys, conds })
    }
}

/// A literal as the cell a column of `kind` holds, or nothing when the column cannot hold it.
fn literal(kind: CellKind, x: &str, interner: &mut Interner) -> Option<u64> {
    match kind {
        CellKind::Symbol => Some(u64::from(interner.id(x.as_bytes()))),
        CellKind::Number => x.parse::<i64>().ok().map(i64::cast_unsigned),
        CellKind::Flag => match x {
            "true" | "raised" | "1" => Some(1),
            "false" | "lowered" | "0" => Some(0),
            _ => None,
        },
        _ => None,
    }
}

/// One file's fact columns.
#[derive(Default)]
pub(super) struct Store {
    /// Once-a-file remap of the file's word codes to run ids (u32::MAX: not seen yet); on with `remap_on`.
    pub(super) remap: Vec<u32>,
    pub(super) remap_on: bool,
    /// Index buffers kept across files.
    pub(super) spare: Vec<(Vec<u32>, Vec<u32>)>,
    /// Per slot: answered for this file (a test on an unanswered relation passes: a superset).
    pub(super) answered: Vec<bool>,
    pub(super) rows: Vec<usize>,
    pub(super) cells: Vec<Vec<Vec<u64>>>,
    /// Per slot and key column: offsets by node, then the rows.
    pub(super) index: Vec<Vec<(usize, Vec<u32>, Vec<u32>)>>,
    /// cols prototype: the provider's direct tables are read instead of `cells`/`index`.
    pub(super) direct_on: bool,
    pub(super) direct: ryo_python::fixtures::columns::FactColumns,
    /// Per slot: (key column, table number in `direct`).
    pub(super) direct_of: Vec<Vec<(usize, usize)>>,
    /// Direct token order: next and previous token by node (u32::MAX: none), and the slot and next-token id.
    pub(super) next_token: Vec<u32>,
    pub(super) prev_token: Vec<u32>,
    pub(super) token_slot: Option<(usize, usize, usize, u64)>,
    /// Reads of a direct table whose column was not written (must stay 0) and of other link kinds.
    pub(super) unwritten: std::cell::Cell<u64>,
}

impl Store {
    pub(super) fn reset(&mut self, slots: usize) {
        self.remap.clear();
        self.answered.clear();
        self.answered.resize(slots, false);
        self.rows.clear();
        self.rows.resize(slots, 0);
        self.cells.resize_with(slots, Vec::new);
        self.index.resize_with(slots, Vec::new);
        self.direct_of.resize_with(slots, Vec::new);
        for d in &mut self.direct_of {
            d.clear();
        }
        for i in &mut self.index {
            for (_, o, r) in i.drain(..) {
                self.spare.push((o, r));
            }
        }
    }

    /// Writes one filled relation as columns, interning its texts, and indexes it by its key columns.
    pub(super) fn write(&mut self, cat: &Catalog, slot: usize, filled: &FilledColumns, words: &languages::FileWords, interner: &mut Interner, elements: usize) {
        let kinds = cat.kinds[slot];
        let n = filled.0.first().map_or(0, |c| c.0.len());
        self.rows[slot] = n;
        let cells = &mut self.cells[slot];
        cells.resize_with(kinds.len(), Vec::new);
        for (c, kind) in kinds.iter().enumerate() {
            let matched = cat.re_columns[slot].contains(&c);
            let out = &mut cells[c];
            out.clear();
            let Some(column) = filled.0.get(c) else {
                out.resize(n, u64::MAX);
                continue;
            };
            match kind {
                CellKind::Symbol if self.remap_on => {
                    let remap = &mut self.remap;
                    // the file's own codes are dense below `words_end`; reserved words (unknown values) above it are interned
                    let end = languages::facts::words_end(words).0 as usize;
                    if remap.len() < end {
                        remap.resize(end, u32::MAX);
                    }
                    out.extend(column.0.iter().map(|w| {
                        let code = w.0 as usize;
                        if code >= remap.len() {
                            let t = languages::facts::text(words, *w);
                            return u64::from(if matched { interner.id_matched(t.0.as_bytes()) } else { interner.id(t.0.as_bytes()) });
                        }
                        if remap[code] == u32::MAX {
                            let t = languages::facts::text(words, *w);
                            remap[code] = if matched { interner.id_matched(t.0.as_bytes()) } else { interner.id(t.0.as_bytes()) };
                        }
                        if matched {
                            let id = remap[code] as usize;
                            if !interner.is_matched(id) {
                                let t = languages::facts::text(words, *w);
                                let _ = interner.id_matched(t.0.as_bytes());
                            }
                        }
                        u64::from(remap[code])
                    }));
                }
                CellKind::Symbol => out.extend(column.0.iter().map(|w| {
                    let t = languages::facts::text(words, *w);
                    u64::from(if matched { interner.id_matched(t.0.as_bytes()) } else { interner.id(t.0.as_bytes()) })
                })),
                CellKind::Node => out.extend(column.0.iter().map(|w| w.0 & 0xffff_ffff)),
                _ => out.extend(column.0.iter().map(|w| w.0)),
            }
        }
        self.index_slot(cat, slot, elements);
        self.answered[slot] = true;
    }

    /// Indexes one written slot by each of its key columns.
    pub(super) fn index_slot(&mut self, cat: &Catalog, slot: usize, elements: usize) {
        let cells = &self.cells[slot];
        let mut spare = std::mem::take(&mut self.spare);
        for &kc in &cat.key_columns[slot] {
            let (mut offsets, mut rows) = spare.pop().unwrap_or_default();
            offsets.clear();
            offsets.resize(elements + 1, 0);
            for &node in &cells[kc] {
                if (node as usize) < elements {
                    offsets[node as usize + 1] += 1;
                }
            }
            for i in 0..elements {
                offsets[i + 1] += offsets[i];
            }
            let mut fill: Vec<u32> = offsets[..elements].to_vec();
            rows.clear();
            rows.resize(offsets[elements] as usize, 0);
            for (r, &node) in cells[kc].iter().enumerate() {
                if (node as usize) < elements {
                    rows[fill[node as usize] as usize] = r as u32;
                    fill[node as usize] += 1;
                }
            }
            self.index[slot].push((kc, offsets, rows));
        }
        self.spare = spare;
    }

    /// Token links straight from the node table's token order (step 5): next token, matching and immediate enclosing
    /// delimiter, with the provider's own rules (an error token or an unmatched closer stops the pairing).
    pub(super) fn token_links(&mut self, cat: &Catalog, slot: usize, table: &ryo::languages::NodeTable, tokens: &dyn TokenOrder, link_ids: [u64; 3], elements: usize) {
        let width = cat.kinds[slot].len();
        let cells = &mut self.cells[slot];
        cells.resize_with(width, Vec::new);
        for c in cells.iter_mut() {
            c.clear();
        }
        let (src, dst, kind) = (cat.column(slot, "source"), cat.column(slot, "target"), cat.column(slot, "link-kind"));
        let mut push = |cells: &mut Vec<Vec<u64>>, s: u32, t: u32, k: u64| {
            for (c, col) in cells.iter_mut().enumerate() {
                col.push(if c == src { u64::from(s) } else if c == dst { u64::from(t) } else if c == kind { k } else { 0 });
            }
        };
        // redo-general: the provider's token order (`provider_tokens.rs`)
        tokens.links(table, &mut |e| match e {
            TokenEvent::Token(_) => {}
            TokenEvent::Next(p, at) => push(cells, p, at, link_ids[0]),
            TokenEvent::Matching(o, c) => push(cells, o, c, link_ids[1]),
            TokenEvent::Enclosing(at, t) => push(cells, at, t, link_ids[2]),
        });
        self.rows[slot] = cells.first().map_or(0, Vec::len);
        self.index_slot(cat, slot, elements);
        self.answered[slot] = true;
    }

    /// Next and previous token straight from the node table's token order, as two per-node columns (cols prototype): the
    /// provider's rule kept (an error token resets the chain).
    pub(super) fn token_order(&mut self, table: &ryo::languages::NodeTable, tokens: &dyn TokenOrder, elements: usize) {
        self.next_token.clear();
        self.next_token.resize(elements, u32::MAX);
        self.prev_token.clear();
        self.prev_token.resize(elements, u32::MAX);
        // redo-general: the provider's token order (`provider_tokens.rs`)
        let (next, prev) = (&mut self.next_token, &mut self.prev_token);
        tokens.links(table, &mut |e| {
            if let TokenEvent::Next(p, at) = e {
                next[p as usize] = at;
                prev[at as usize] = p;
            }
        });
    }

    #[inline]
    fn holds_direct(&self, op: &FactOp, regs: &[u32], matches: &[u128]) -> bool {
        let (kc, reg) = op.keys[0];
        let node = regs[reg] as usize;
        if let Some((slot, src, dst, next_id)) = self.token_slot
            && slot == op.slot
        {
            if !op.conds.iter().all(|c| matches!(c, Cond::In(_, v) if v.contains(&next_id))) {
                self.unwritten.set(self.unwritten.get() + 1);
                return true;
            }
            let (column, other) = if kc == src { (&self.next_token, dst) } else { (&self.prev_token, src) };
            let Some(&t) = column.get(node) else { return false };
            if t == u32::MAX {
                return false;
            }
            return op.keys[1..].iter().all(|(c, g)| *c == other && t == regs[*g]);
        }
        let Some(&(_, at)) = self.direct_of[op.slot].iter().find(|(c, _)| *c == kc) else { return true };
        let t = &self.direct.tables[at];
        if !t.answered {
            return true;
        }
        if !t.present.get(node / 64).is_some_and(|w| w >> (node % 64) & 1 == 1) {
            return false;
        }
        if op.keys.len() == 1 && op.conds.is_empty() {
            return true;
        }
        let rows = t.rows_of(node);
        let cell = |c: usize, r: usize| -> Option<u64> { t.columns.get(c).and_then(|col| col.get(r)).map(|v| u64::from(*v)) };
        rows.into_iter().any(|r| {
            let r = t.row(r);
            let mut missing = false;
            let ok = op.keys[1..].iter().all(|(c, g)| match cell(*c, r) {
                Some(v) => v == u64::from(regs[*g]),
                None => {
                    missing = true;
                    true
                }
            }) && op.conds.iter().all(|cond| {
                let c = match cond {
                    Cond::In(c, _) | Cond::NotIn(c, _) | Cond::Re(c, _) | Cond::Cmp(c, ..) => *c,
                };
                let Some(v) = cell(c, r) else {
                    missing = true;
                    return true;
                };
                match cond {
                    Cond::In(_, vals) => vals.contains(&v),
                    Cond::NotIn(_, vals) => !vals.contains(&v),
                    Cond::Re(_, bit) => matches.get(v as usize).is_some_and(|m| m >> bit & 1 == 1),
                    Cond::Cmp(_, ord, or_equal, n) => {
                        let x = i64::from(v as u32 as i32);
                        let o = x.cmp(n);
                        o == *ord || (*or_equal && o == std::cmp::Ordering::Equal)
                    }
                }
            });
            if missing {
                self.unwritten.set(self.unwritten.get() + 1);
            }
            ok
        })
    }

    #[inline]
    pub(super) fn holds(&self, op: &FactOp, regs: &[u32], matches: &[u128]) -> bool {
        if !self.answered.get(op.slot).copied().unwrap_or(false) {
            return true;
        }
        if self.direct_on {
            return self.holds_direct(op, regs, matches);
        }
        let (kc, reg) = op.keys[0];
        let node = regs[reg] as usize;
        let Some((_, offsets, rows)) = self.index[op.slot].iter().find(|(c, _, _)| *c == kc) else { return true };
        if node + 1 >= offsets.len() {
            return false;
        }
        let cells = &self.cells[op.slot];
        rows[offsets[node] as usize..offsets[node + 1] as usize].iter().any(|&r| {
            let r = r as usize;
            op.keys[1..].iter().all(|(c, g)| cells[*c][r] == u64::from(regs[*g]))
                && op.conds.iter().all(|cond| match cond {
                    Cond::In(c, vals) => vals.contains(&cells[*c][r]),
                    Cond::NotIn(c, vals) => !vals.contains(&cells[*c][r]),
                    Cond::Re(c, bit) => matches.get(cells[*c][r] as usize).is_some_and(|m| m >> bit & 1 == 1),
                    Cond::Cmp(c, ord, or_equal, n) => {
                        let x = cells[*c][r].cast_signed();
                        let o = x.cmp(n);
                        o == *ord || (*or_equal && o == std::cmp::Ordering::Equal)
                    }
                })
        })
    }
}
