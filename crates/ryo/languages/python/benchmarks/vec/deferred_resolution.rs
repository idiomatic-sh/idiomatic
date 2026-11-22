//! redo-binding pass 4 (THROWAWAY, bead ryo-fmx3.1): the `deferred-resolution` fact. For every read whose evaluation is
//! postponed (`deferred-visit`), the event it sees once the module has run (PEP 563: the module's globals, else the
//! nearest namespace of its lookup path holding the name at the end); in a stub, a final deletion found that way that
//! comes after the read in the source gives back the event before it (a stub is never run in order: PEP 484), a module
//! deletion with nothing before it gives back the builtin; `absent` where nothing answers (and for an unvisited read).
//! Every row carries the file's last schedule point. The same rows `deferred-read-events` made from `binding-read`;
//! written from binding-read, binding-event, lookup-scope, implicit-symbol and the facts namespace-final and
//! deferred-visit; proved by the equality probe.

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
    let (s_read, s_bind, s_delete, s_annotation, s_cleanup, s_absent) =
        (s(run, b"read"), s(run, b"bind"), s(run, b"delete"), s(run, b"annotation"), s(run, b"cleanup"), s(run, b"absent"));
    let (s_deferred, s_unvisited, s_builtin) = (s(run, b"deferred"), s(run, b"unvisited"), s(run, b"builtin"));
    let k_module = run.sym(b"module");
    // the file's last point
    let ev = inp.events;
    let (e_occ, e_scope, e_name, e_ord, e_kind) = (col(ev, "occurrence")?, col(ev, "scope")?, col(ev, "name")?, col(ev, "ordinal")?, col(ev, "kind")?);
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
    // deferred / unvisited references
    let (v_id, v_visit) = (col(inp.visits, "id")?, col(inp.visits, "visit")?);
    let mut visit: FxHashMap<u64, (bool, bool)> = FxHashMap::default();
    for r in 0..inp.visits.n {
        let e = visit.entry(inp.visits.cols[v_id][r]).or_default();
        let v = inp.visits.cols[v_visit][r];
        e.0 |= v == s_deferred;
        e.1 |= v == s_unvisited;
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
            let Some(&(d, u)) = visit.get(&id) else { continue };
            if (d || u) && seen_ids.insert(id) {
                deferred_ids.push(id);
            }
            if d && read_points.contains_key(&id) {
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
    let mut deleted_names: FxHashSet<u64> = FxHashSet::default();
    for r in 0..fin.n {
        let _ = final_of.insert((fin.cols[f_ns][r], fin.cols[f_name][r]), (fin.cols[f_event][r], fin.cols[f_kind][r]));
        held_names.insert(fin.cols[f_name][r]);
        if fin.cols[f_kind][r] == s_delete {
            deleted_names.insert(fin.cols[f_name][r]);
        }
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
    let start = |x: u64| node_table::span_of(table, Position(x as u32)).start.0;
    // events by (scope, name): (ordinal, occurrence, kind)
    let mut by_scope_name: FxHashMap<(u64, u64), Vec<(i64, u64, u64)>> = FxHashMap::default();
    let mut event_row: FxHashMap<u64, Vec<usize>> = FxHashMap::default();
    for r in 0..ev.n {
        by_scope_name.entry((ev.cols[e_scope][r], ev.cols[e_name][r])).or_default().push((ev.cols[e_ord][r] as i64, ev.cols[e_occ][r], ev.cols[e_kind][r]));
        event_row.entry(ev.cols[e_occ][r]).or_default().push(r);
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
        let deleted_stub = inp.stub && deleted_names.contains(&name);
        let has_final = !finals.is_empty() && !deleted_stub;
        if has_final {
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
        let Some((_, occurrence, scope, kind)) = best else { continue };
        // a stub's forward deletion gives back what came before it
        if kind == s_delete && inp.stub {
            let mut restored = false;
            let mut dropped = false;
            for &r in event_row.get(&occurrence).map_or(&[][..], Vec::as_slice) {
                let (d_name, d_point) = (ev.cols[e_name][r], ev.cols[e_ord][r] as i64);
                if start(id) >= start(occurrence) {
                    continue;
                }
                let options: Vec<(i64, u64, u64)> = by_scope_name
                    .get(&(scope, d_name))
                    .map_or(&[][..], Vec::as_slice)
                    .iter()
                    .copied()
                    .filter(|(o, _, k)| *o < d_point && (*k == s_bind || *k == s_annotation || *k == s_delete || *k == s_cleanup))
                    .collect();
                if let Some(max) = options.iter().map(|o| o.0).max() {
                    for (o, occ, k) in &options {
                        if *o == max && *k == s_bind {
                            restored = true;
                            // the event at that point of that scope and name
                            for (o2, occ2, k2) in by_scope_name.get(&(scope, d_name)).map_or(&[][..], Vec::as_slice) {
                                if *o2 == *o {
                                    results.insert((id, *occ2, *k2));
                                }
                            }
                            let _ = occ;
                        }
                    }
                } else if modules.contains(&scope) && builtins.contains(&d_name) {
                    dropped = true;
                }
            }
            if restored || dropped {
                continue;
            }
        }
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
