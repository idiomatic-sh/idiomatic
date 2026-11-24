//! `deferred-resolution` fact. For every read whose evaluation is
//! postponed (`deferred-visit`), what Python answers once the module has words (PEP 563: a postponed annotation is
//! evaluated against the module's globals, then the builtins): the module's final bind; else the builtin, when the
//! file's `language-target` has it (`implicit-symbol`'s since-version); `absent` otherwise. Every row carries the file's
//! final point (the largest schedule point of its events, once). Written from binding-read, binding-event,
//! implicit-symbol, language-target and the facts namespace-final and deferred-visit; proved by the equality probe.
//! Pass 5 (PRINCIPLE): Ruff's stub convention and its unvisited reads are rule text (`deferred-read-events`). Pass 6
//! (GENERALITY): three more parts are rule text there: a name some namespace holds that the module does not is answered
//! from the nearest enclosing namespace of the lookup path holding it at the end (a checker's lexical model; Python's
//! `get_type_hints` sees the module and, for a class, its own namespace only), a builtin answer is dropped where the
//! module holds no final row for the name, and the per-point repetition of the old text (K1).

use super::{InputTable, Vocabulary, col};
use ryo::languages::{NodeTable, Position, node_table};
use std::collections::{HashMap, HashSet};

pub(super) struct Inputs<'t> {
    pub(super) reads: &'t InputTable,
    pub(super) events: &'t InputTable,
    pub(super) implicit: &'t InputTable,
    pub(super) targets: &'t InputTable,
    pub(super) finals: &'t InputTable,
    pub(super) visits: &'t InputTable,
}

/// One row: reference, occurrence, kind (a symbol), point.
pub(super) type Row = (u64, u64, u64, u64);

pub(super) fn rows(
    words: &mut Vocabulary,
    table: &NodeTable,
    inp: &Inputs<'_>,
) -> Result<Vec<Row>, String> {
    let mut out: Vec<Row> = Vec::new();
    let s = |words: &mut Vocabulary, t: &[u8]| u64::from(words.symbol(t));
    let (s_read, s_bind, s_absent) = (s(words, b"read"), s(words, b"bind"), s(words, b"absent"));
    let (s_deferred, s_builtin) = (s(words, b"deferred"), s(words, b"builtin"));
    let k_module = words.symbol(b"module");
    // the file's last point
    let ev = inp.events;
    let (e_occ, e_ord, e_kind) = (
        col(ev, "occurrence")?,
        col(ev, "ordinal")?,
        col(ev, "kind")?,
    );
    // the file's final point: its events' largest point, once
    let Some(final_point) = (0..ev.n).map(|r| ev.cols[e_ord][r] as i64).max() else {
        return Ok(out);
    };
    // deferred references
    let (v_id, v_visit) = (col(inp.visits, "id")?, col(inp.visits, "visit")?);
    let mut visit: HashSet<u64> = HashSet::default();
    for r in 0..inp.visits.n {
        if inp.visits.cols[v_visit][r] == s_deferred {
            visit.insert(inp.visits.cols[v_id][r]);
        }
    }
    let (b_ref, b_name, b_scope) = (
        col(inp.reads, "reference")?,
        col(inp.reads, "name")?,
        col(inp.reads, "scope")?,
    );
    let mut read_points: HashMap<u64, Vec<usize>> = HashMap::default();
    for r in 0..ev.n {
        if ev.cols[e_kind][r] == s_read {
            read_points.entry(ev.cols[e_occ][r]).or_default().push(r);
        }
    }
    // deferred reads: (id, name, initial scope)
    let mut deferred_reads: Vec<(u64, u64, u64)> = Vec::new();
    let mut deferred_ids: Vec<u64> = Vec::new();
    {
        let mut seen: HashSet<(u64, u64, u64)> = HashSet::default();
        let mut seen_ids: HashSet<u64> = HashSet::default();
        for r in 0..inp.reads.n {
            let id = inp.reads.cols[b_ref][r];
            if !visit.contains(&id) {
                continue;
            }
            if seen_ids.insert(id) {
                deferred_ids.push(id);
            }
            if read_points.contains_key(&id) {
                let t = (id, inp.reads.cols[b_name][r], inp.reads.cols[b_scope][r]);
                if seen.insert(t) {
                    deferred_reads.push(t);
                }
            }
        }
    }
    if deferred_ids.is_empty() {
        return Ok(out);
    }
    let modules: Vec<u64> = (0..node_table::element_count(table).0 as u32)
        .filter(|p| {
            words.kind_sym[usize::from(node_table::kind_of(table, Position(*p)).0)] == k_module
        })
        .map(u64::from)
        .collect();
    let fin = inp.finals;
    let (f_ns, f_name, f_event, f_kind) = (
        col(fin, "namespace")?,
        col(fin, "name")?,
        col(fin, "event")?,
        col(fin, "kind")?,
    );
    let mut final_of: HashMap<(u64, u64), (u64, u64)> = HashMap::default();
    for r in 0..fin.n {
        let _ = final_of.insert(
            (fin.cols[f_ns][r], fin.cols[f_name][r]),
            (fin.cols[f_event][r], fin.cols[f_kind][r]),
        );
    }
    // the file's target (no target row: no builtin is known to be there)
    let (t_major, t_minor) = (col(inp.targets, "major")?, col(inp.targets, "minor")?);
    let target: Option<(i64, i64)> = (0..inp.targets.n)
        .map(|r| {
            (
                inp.targets.cols[t_major][r] as i64,
                inp.targets.cols[t_minor][r] as i64,
            )
        })
        .min();
    let (i_name, i_kind, i_major, i_minor) = (
        col(inp.implicit, "name")?,
        col(inp.implicit, "kind")?,
        col(inp.implicit, "since-major")?,
        col(inp.implicit, "since-minor")?,
    );
    let mut builtins: HashSet<u64> = HashSet::default();
    for r in 0..inp.implicit.n {
        let since = (
            inp.implicit.cols[i_major][r] as i64,
            inp.implicit.cols[i_minor][r] as i64,
        );
        if inp.implicit.cols[i_kind][r] == s_builtin && target.is_some_and(|t| since <= t) {
            builtins.insert(inp.implicit.cols[i_name][r]);
        }
    }
    let mut results: HashSet<(u64, u64, u64)> = HashSet::default();
    for &(id, name, _) in &deferred_reads {
        // the module's final bind, else the builtin
        let mut bound = false;
        for m in &modules {
            if let Some(&(event, kind)) = final_of.get(&(*m, name))
                && kind == s_bind
            {
                results.insert((id, event, s_bind));
                bound = true;
            }
        }
        if !bound && builtins.contains(&name) {
            results.insert((id, id, s_builtin));
        }
    }
    let answered: HashSet<u64> = results.iter().map(|r| r.0).collect();
    let point = final_point as u64;
    for &(reference, occurrence, kind) in &results {
        out.push((reference, occurrence, kind, point));
    }
    for &id in &deferred_ids {
        if !answered.contains(&id) {
            out.push((id, id, s_absent, point));
        }
    }
    Ok(out)
}
