//! redo-binding pass 4 (THROWAWAY, bead ryo-fmx3.1): the `deferred-resolution` fact. For every read whose evaluation is
//! postponed (`deferred-visit`), the event it sees once the module has run (PEP 563: the module's globals, else the
//! nearest namespace of its lookup path holding the name at the end; annotation-only declarations count in a stub, PEP
//! 484); `absent` where nothing answers. Every row carries the file's last schedule point. Written from binding-read,
//! binding-event, lookup-scope, implicit-symbol and the facts namespace-final and deferred-visit; proved by the equality
//! probe. Pass 5 (PRINCIPLE): Ruff's stub convention (a stub read whose name some namespace finally deletes skips the
//! module's final bind, and a forward deletion gives back the event before it) and its unvisited reads are rule text
//! (`deferred-read-events`), not this fact.

use super::finish;
use ryo::languages::{NodeTable, Position, node_table};
use rustc_hash::{FxHashMap, FxHashSet};

fn col(t: &finish::Table, n: &str) -> Result<usize, String> {
    t.names.iter().position(|c| c == n).ok_or_else(|| format!("no column {n}"))
}

pub(super) struct Inputs<'t> {
    pub(super) reads: &'t finish::Table,
    pub(super) events: &'t finish::Table,
    pub(super) lookup: &'t finish::Table,
    pub(super) implicit: &'t finish::Table,
    pub(super) finals: &'t finish::Table,
    pub(super) visits: &'t finish::Table,
    pub(super) stub: bool,
}

/// One row: reference, occurrence, kind (a symbol), point.
pub(super) type Row = (u64, u64, u64, u64);

pub(super) fn rows(run: &mut finish::Run, table: &NodeTable, inp: &Inputs<'_>) -> Result<Vec<Row>, String> {
    let mut out: Vec<Row> = Vec::new();
    let s = |run: &mut finish::Run, t: &[u8]| u64::from(run.sym(t));
    let (s_read, s_bind, s_annotation, s_absent) = (s(run, b"read"), s(run, b"bind"), s(run, b"annotation"), s(run, b"absent"));
    let (s_deferred, s_builtin) = (s(run, b"deferred"), s(run, b"builtin"));
    let k_module = run.sym(b"module");
    // the file's last point
    let ev = inp.events;
    let (e_occ, e_ord, e_kind) = (col(ev, "occurrence")?, col(ev, "ordinal")?, col(ev, "kind")?);
    // the old text's final points: every event's point paired with the next unpaired greater one (an ordered self-match
    // over the multiset of points); the points left unpaired. With unique points that is the last one; a statement that
    // makes several events at one point (`global a, b, c`) leaves (copies - 1) more points unpaired at the end, and the old
    // text gives each answer once per such point. Kept so the rows (and the findings) stay the same.
    let mut points: Vec<i64> = (0..ev.n).map(|r| ev.cols[e_ord][r] as i64).collect();
    points.sort_unstable();
    let mut paired: FxHashSet<i64> = FxHashSet::default();
    let mut at = 0;
    for &o in &points {
        while at < points.len() && points[at] <= o {
            at += 1;
        }
        if at < points.len() {
            paired.insert(o);
            at += 1;
        }
    }
    let mut finals_points: Vec<u64> = points.iter().copied().filter(|p| !paired.contains(p)).map(|p| p as u64).collect();
    finals_points.dedup();
    if finals_points.is_empty() {
        return Ok(out);
    }
    // deferred references
    let (v_id, v_visit) = (col(inp.visits, "id")?, col(inp.visits, "visit")?);
    let mut visit: FxHashSet<u64> = FxHashSet::default();
    for r in 0..inp.visits.n {
        if inp.visits.cols[v_visit][r] == s_deferred {
            visit.insert(inp.visits.cols[v_id][r]);
        }
    }
    let (b_ref, b_name, b_scope) = (col(inp.reads, "reference")?, col(inp.reads, "name")?, col(inp.reads, "scope")?);
    let mut read_points: FxHashMap<u64, Vec<usize>> = FxHashMap::default();
    for r in 0..ev.n {
        if ev.cols[e_kind][r] == s_read {
            read_points.entry(ev.cols[e_occ][r]).or_default().push(r);
        }
    }
    // deferred reads: (id, name, initial scope)
    let mut deferred_reads: Vec<(u64, u64, u64)> = Vec::new();
    let mut deferred_ids: Vec<u64> = Vec::new();
    {
        let mut seen: FxHashSet<(u64, u64, u64)> = FxHashSet::default();
        let mut seen_ids: FxHashSet<u64> = FxHashSet::default();
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
        .filter(|p| run.kind_sym[usize::from(node_table::kind_of(table, Position(*p)).0)] == k_module)
        .map(u64::from)
        .collect();
    let fin = inp.finals;
    let (f_ns, f_name, f_event, f_kind) = (col(fin, "namespace")?, col(fin, "name")?, col(fin, "event")?, col(fin, "kind")?);
    let mut final_of: FxHashMap<(u64, u64), (u64, u64)> = FxHashMap::default();
    let mut held_names: FxHashSet<u64> = FxHashSet::default();
    for r in 0..fin.n {
        let _ = final_of.insert((fin.cols[f_ns][r], fin.cols[f_name][r]), (fin.cols[f_event][r], fin.cols[f_kind][r]));
        held_names.insert(fin.cols[f_name][r]);
    }
    let (i_name, i_kind, i_major, i_minor) = (col(inp.implicit, "name")?, col(inp.implicit, "kind")?, col(inp.implicit, "since-major")?, col(inp.implicit, "since-minor")?);
    let mut builtins: FxHashSet<u64> = FxHashSet::default();
    for r in 0..inp.implicit.n {
        let (major, minor) = (inp.implicit.cols[i_major][r] as i64, inp.implicit.cols[i_minor][r] as i64);
        if inp.implicit.cols[i_kind][r] == s_builtin && (major < 3 || (major == 3 && minor <= 10)) {
            builtins.insert(inp.implicit.cols[i_name][r]);
        }
    }
    let (l_ref, l_scope, l_depth) = (col(inp.lookup, "reference")?, col(inp.lookup, "scope")?, col(inp.lookup, "depth")?);
    let mut paths: FxHashMap<u64, Vec<(i64, u64)>> = FxHashMap::default();
    for r in 0..inp.lookup.n {
        paths.entry(inp.lookup.cols[l_ref][r]).or_default().push((inp.lookup.cols[l_depth][r] as i64, inp.lookup.cols[l_scope][r]));
    }
    let mut results: FxHashSet<(u64, u64, u64)> = FxHashSet::default();
    for &(id, name, _) in &deferred_reads {
        // the module's final bind
        let mut finals = Vec::new();
        for m in &modules {
            if let Some(&(event, kind)) = final_of.get(&(*m, name))
                && kind == s_bind
            {
                finals.push(event);
            }
        }
        if !finals.is_empty() {
            for e in finals {
                results.insert((id, e, s_bind));
            }
            continue;
        }
        if !held_names.contains(&name) {
            continue;
        }
        if modules.iter().any(|m| !final_of.contains_key(&(*m, name))) && builtins.contains(&name) {
            continue;
        }
        // the nearest namespace of the lookup path holding the name at the end
        let mut best: Option<(i64, u64, u64, u64)> = None;
        if let Some(ps) = paths.get(&id) {
            for &(depth, scope) in ps {
                if let Some(&(event, kind)) = final_of.get(&(scope, name))
                    && (kind != s_annotation || inp.stub)
                    && best.is_none_or(|b| depth < b.0)
                {
                    best = Some((depth, event, scope, kind));
                }
            }
        }
        let Some((_, occurrence, _, kind)) = best else { continue };
        results.insert((id, occurrence, kind));
    }
    let answered: FxHashSet<u64> = results.iter().map(|r| r.0).collect();
    for &point in &finals_points {
        for &(reference, occurrence, kind) in &results {
            out.push((reference, occurrence, kind, point));
        }
        for &id in &deferred_ids {
            if !answered.contains(&id) {
                out.push((id, id, s_absent, point));
            }
        }
    }
    Ok(out)
}
