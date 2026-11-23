//! THROWAWAY (redo-binding, ryo-fmx3.1, pass 6): `order`, rows ordered by a list of keys, each with its own comparison.
//!
//! `order(from=R, keys=["g", ...], by=[["k1", "number"], ["k2", "natural-folded"], ["k3", "text", "desc"], ...],
//! position="p", rank="rank", step="step")`: every row of `R` once, with two new number columns. Within each group (equal
//! `keys`), `rank` is the row's place (0-based) when the group is ordered by the `by` keys in turn (a later key only breaks
//! the ties of the earlier ones; rows equal on every key keep their `position` order); `step` compares the row's keys
//! with those of the row just before it in `position` order: 0 when they order before it (the pair descends), 1 when
//! equal, 2 when after; the first row of its group gets 2 (a number cell cannot hold -1: it is the null). So
//! `select ... where step = 0` holds the rows that break a
//! non-descending order, and `rank` with `group-ordinal` over `position` tells whether a row is where an ordering puts it.
//! Comparisons: `number` (signed), `text` (bytes), `natural` (digit runs read as numbers), `natural-folded` (natural on the
//! lower-cased texts; when the lower-cased texts are the same bytes, natural on the texts themselves; otherwise equal).
//! `desc` reverses a key. A null key value orders first. No rule id: the keys and comparisons are the arguments.
//! MERGE NOTE: the table built here is the input's columns plus two pushed number columns (`out.push_row`), the same shape
//! as `prefixes` / `group-ordinal`, so it re-points at the pooled tables as they do.

use crate::finish::TableAccess as _;
use std::cmp::Ordering;

use super::{Exec, File, NULL, Provider, R, Run, Table, Ty, s, strs};
use rustc_hash::FxHashMap;

#[derive(Clone, Copy)]
enum Cmp {
    Number,
    Text,
    Natural,
    NaturalFolded,
}

/// A text's natural-order units, as `natural_lt` splits it (whitespace dropped, digit runs kept whole, every other
/// character a unit of its own); `natural_cmp` compares two unit lists as `natural_lt` does.
fn units(s: &str) -> Vec<(bool, String)> {
    let mut out: Vec<(bool, String)> = Vec::new();
    let mut cur = String::new();
    let mut digits = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !cur.is_empty() {
                out.push((digits, std::mem::take(&mut cur)));
            }
            continue;
        }
        if ch.is_ascii_digit() {
            if !digits && !cur.is_empty() {
                out.push((false, std::mem::take(&mut cur)));
            }
            digits = true;
            cur.push(ch);
        } else {
            if digits && !cur.is_empty() {
                out.push((true, std::mem::take(&mut cur)));
            }
            digits = false;
            out.push((false, ch.to_string()));
        }
    }
    if !cur.is_empty() {
        out.push((digits, cur));
    }
    out
}

fn natural_cmp(x: &[(bool, String)], y: &[(bool, String)]) -> Ordering {
    for (p, q) in x.iter().zip(y.iter()) {
        let ord = match (p.0, q.0) {
            (true, true) if p.1.starts_with('0') || q.1.starts_with('0') => p.1.cmp(&q.1),
            (true, true) => p.1.len().cmp(&q.1.len()).then_with(|| p.1.cmp(&q.1)),
            _ => p.1.cmp(&q.1),
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    x.len().cmp(&y.len())
}

/// One key cell, prepared once a row: the comparison reads only these.
enum Key {
    Null,
    Number(i64),
    Text(Vec<u8>),
    Natural(Vec<(bool, String)>),
    Folded { lower: String, lower_units: Vec<(bool, String)>, units: Vec<(bool, String)> },
}

fn prepare(run: &Run, ty: Ty, cmp: Cmp, v: u64) -> Key {
    if v == NULL {
        return Key::Null;
    }
    let text = || -> Vec<u8> {
        match ty {
            Ty::Symbol => run.text(v as u32).to_vec(),
            Ty::File => run.text(run.path_sym[v as usize]).to_vec(),
            Ty::Number => (v as i64).to_string().into_bytes(),
            _ => v.to_string().into_bytes(),
        }
    };
    match cmp {
        Cmp::Number => Key::Number(v as i64),
        Cmp::Text => Key::Text(text()),
        Cmp::Natural => Key::Natural(units(&String::from_utf8_lossy(&text()))),
        Cmp::NaturalFolded => {
            let raw = String::from_utf8_lossy(&text()).into_owned();
            let lower = raw.to_lowercase();
            Key::Folded { lower_units: units(&lower), lower, units: units(&raw) }
        }
    }
}

fn compare(x: &Key, y: &Key) -> Ordering {
    match (x, y) {
        (Key::Null, Key::Null) => Ordering::Equal,
        (Key::Null, _) => Ordering::Less,
        (_, Key::Null) => Ordering::Greater,
        (Key::Number(a), Key::Number(b)) => a.cmp(b),
        (Key::Text(a), Key::Text(b)) => a.cmp(b),
        (Key::Natural(a), Key::Natural(b)) => natural_cmp(a, b),
        // natural on the lower-cased texts; the texts themselves only when the lower-cased ones are the same bytes
        (Key::Folded { lower: la, lower_units: a, units: ra }, Key::Folded { lower: lb, lower_units: b, units: rb }) => {
            match natural_cmp(a, b) {
                Ordering::Equal if la == lb => natural_cmp(ra, rb),
                o => o,
            }
        }
        _ => Ordering::Equal,
    }
}

impl Exec<'_> {
    pub(super) fn order(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Table> {
        let a = &self.plan.nodes[node].j["args"];
        let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
        let keys: Vec<usize> = strs(&a["keys"])?.map(|c| from.col(c)).collect::<R<_>>()?;
        let pc = from.col(s(&a["position"])?)?;
        let mut by: Vec<(usize, Cmp, bool)> = Vec::new();
        for k in a["by"].as_array().ok_or("order: by")? {
            let k = k.as_array().ok_or("order: a key is [column, comparison, direction?]")?;
            let c = from.col(s(&k[0])?)?;
            let cmp = match s(&k[1])? {
                "number" => Cmp::Number,
                "text" => Cmp::Text,
                "natural" => Cmp::Natural,
                "natural-folded" => Cmp::NaturalFolded,
                other => return Err(format!("order: comparison {other}")),
            };
            let desc = match k.get(2).map(s).transpose()? {
                None | Some("asc") => false,
                Some("desc") => true,
                Some(other) => return Err(format!("order: direction {other}")),
            };
            by.push((c, cmp, desc));
        }
        let mut groups: FxHashMap<Vec<u64>, Vec<usize>> = FxHashMap::default();
        for i in 0..from.rows.0 {
            groups.entry(keys.iter().map(|c| from.cols[*c][i]).collect()).or_default().push(i);
        }
        // each key cell is prepared the first time a comparison needs it (most pairs are told apart by an early key,
        // or hold the same value, which orders equal without preparing anything)
        let prepared: Vec<Vec<std::cell::OnceCell<Key>>> = by.iter().map(|_| (0..from.rows.0).map(|_| std::cell::OnceCell::new()).collect()).collect();
        let cmp_rows = |i: usize, j: usize| -> Ordering {
            for (k, &(c, cmp, desc)) in by.iter().enumerate() {
                let (x, y) = (from.cols[c][i], from.cols[c][j]);
                if x == y {
                    continue;
                }
                let a = prepared[k][i].get_or_init(|| prepare(run, from.tys[c], cmp, x));
                let b = prepared[k][j].get_or_init(|| prepare(run, from.tys[c], cmp, y));
                let o = compare(a, b);
                if o != Ordering::Equal {
                    return if desc { o.reverse() } else { o };
                }
            }
            Ordering::Equal
        };
        let mut rank = vec![0u64; from.rows.0];
        let mut step = vec![2u64; from.rows.0];
        for rows in groups.values_mut() {
            rows.sort_by_key(|i| from.cols[pc][*i] as i64);
            for w in 1..rows.len() {
                step[rows[w]] = match cmp_rows(rows[w], rows[w - 1]) {
                    Ordering::Less => 0,
                    Ordering::Equal => 1,
                    Ordering::Greater => 2,
                };
            }
            let mut sorted = rows.clone();
            sorted.sort_by(|i, j| cmp_rows(*i, *j));
            for (k, i) in sorted.iter().enumerate() {
                rank[*i] = k as u64;
            }
        }
        let schema = self.schema(run, node)?;
        let mut out = run.storage.copy(&from);
        out.names = schema.names.clone();
        out.tys = schema.tys.clone();
        out.cols.push(rank);
        out.cols.push(step);
        self.stats.touched[node] += from.rows.0 as u64;
        Ok(out)
    }
}
