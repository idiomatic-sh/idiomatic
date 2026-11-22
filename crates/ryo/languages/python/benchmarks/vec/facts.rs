//! THROWAWAY (ryo-fmx3.1, step 4): the provider's derived facts written once a file as per-node columns. Each fact relation a
//! plan tests is filled through the walk's fillers (the provider's model is the source of truth); its rows are kept as flat
//! columns (texts interned into one run-wide id space, nodes as positions) with an index from a key node to its rows, so a
//! plan's test "this node has a row of R whose columns hold these values" is an index read and a few compares.

use ryo::languages::{self, CellKind, FilledColumns, Relation};
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::service as catalog;
use std::collections::HashMap;

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
    fn get(&self, k: &[u8]) -> Option<u32> {
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
        i
    }

    /// The id of a text of a column some regex tests: its regex bits are computed once, the first time.
    pub(super) fn id_matched(&mut self, text: &[u8]) -> u32 {
        let i = self.id(text);
        if !self.matched[i as usize] {
            let mut bits = 0u128;
            for (r, re) in self.regexes.iter().enumerate() {
                if re.is_match(text) {
                    bits |= 1 << r;
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
}

impl Catalog {
    pub(super) fn column(&self, slot: usize, name: &str) -> usize {
        self.columns[slot].iter().position(|c| c == name).expect("column")
    }

    pub(super) fn op(&mut self, v: &serde_json::Value, interner: &mut Interner) -> Option<FactOp> {
        let name = v["rel"].as_str().expect("rel");
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
                self.names.len() - 1
            }
        };
        let col = |c: &str| self.columns[slot].iter().position(|x| x == c);
        let mut keys = Vec::new();
        for k in v["keys"].as_array().expect("keys") {
            let c = col(k[0].as_str().expect("key column"))?;
            if self.kinds[slot][c] != CellKind::Node {
                return None;
            }
            keys.push((c, k[1].as_u64().expect("reg") as usize));
        }
        let mut conds = Vec::new();
        for c in v["conds"].as_array().expect("conds") {
            let Some(at) = col(c[0].as_str().expect("column")) else { continue };
            let kind = self.kinds[slot][at];
            let op = c[1].as_str().expect("op");
            let vals: Vec<&str> = c[2].as_array().expect("vals").iter().map(|x| x.as_str().expect("val")).collect();
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

/// The token kinds of the node table and the three bracket pairs, by code.
pub(super) struct TokenKinds {
    pub(super) first: u16,
    pub(super) last: u16,
    pub(super) error: u16,
    pub(super) pairs: [(u16, u16); 3],
}

impl TokenKinds {
    fn is_token(&self, k: u16) -> bool {
        k >= self.first && k <= self.last
    }
    fn closer(&self, k: u16) -> Option<u16> {
        self.pairs.iter().find(|(o, _)| *o == k).map(|(_, c)| *c)
    }
    fn is_closer(&self, k: u16) -> bool {
        self.pairs.iter().any(|(_, c)| *c == k)
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
    pub(super) fn token_links(&mut self, cat: &Catalog, slot: usize, table: &ryo::languages::NodeTable, tokens: &TokenKinds, link_ids: [u64; 3], elements: usize) {
        use ryo::languages::{KindBits, Position, node_table};
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
        let start = node_table::node_count(table).0 as u32;
        let end = node_table::own_end(table).0;
        let mut stack: Vec<(u16, u32)> = Vec::new();
        let mut previous: Option<u32> = None;
        let mut lost = false;
        for at in start..end {
            let k = node_table::kind_of(table, Position(at)).0;
            if !tokens.is_token(k) {
                continue;
            }
            if k == tokens.error {
                lost = true;
                previous = None;
                stack.clear();
                continue;
            }
            let mut paired = None;
            let mut enclosing = stack.last().map(|o| o.1);
            if let Some(close) = tokens.closer(k) {
                stack.push((close, at));
            } else if tokens.is_closer(k) {
                match stack.pop() {
                    Some((close, open)) if close == k => {
                        paired = Some((open, at));
                        enclosing = stack.last().map(|o| o.1);
                    }
                    _ => {
                        lost = true;
                        stack.clear();
                        enclosing = None;
                    }
                }
            }
            if let Some(p) = previous.replace(at) {
                push(cells, p, at, link_ids[0]);
            }
            if let Some((o, c)) = paired {
                push(cells, o, c, link_ids[1]);
            }
            if let Some(t) = enclosing {
                push(cells, at, t, link_ids[2]);
            }
        }
        let _ = (lost, KindBits(0));
        self.rows[slot] = cells.first().map_or(0, Vec::len);
        self.index_slot(cat, slot, elements);
        self.answered[slot] = true;
    }

    #[inline]
    pub(super) fn holds(&self, op: &FactOp, regs: &[u32], matches: &[u128]) -> bool {
        if !self.answered.get(op.slot).copied().unwrap_or(false) {
            return true;
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
