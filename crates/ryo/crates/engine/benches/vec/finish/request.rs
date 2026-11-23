//! redo-fills pass 3 (THROWAWAY, bead ryo-fmx3.1): ONE request a file. When a file's second half starts, the rules alive
//! in it (the first half's survivors and skip: the emits the finisher runs) say, through the plan, which base relations
//! their steps can read. The executor sends the provider one request for exactly that set (relations, the columns read,
//! the scope) and keeps the one answer; every later fill is a slot read of it (`answered`): no call into the provider.
//! The host's relations of the same set are the host's answer at the same point. The form is the language's and the
//! scope's: nothing here names a relation, a language or a file beyond the request's scope.

use super::request_guards::{ALWAYS, Guards};
use super::{E, Exec, File, Native, Provider, R, Run, Scope, Supplier, Table, host, native_of};
use rustc_hash::FxHashMap;
use std::rc::Rc;
use ryo::languages::ColumnProjection as Columns;

/// One relation of a request: its name and the columns its readers can read (bit `c`: the catalog's column `c`; every bit:
/// every column).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Asked<'r> {
    pub(crate) rel: &'r str,
    pub(crate) columns: &'r Columns,
    pub(crate) scope: Scope<'r>,
}

/// What the executor asks a provider once for a scope: every relation the scope's alive rules can read. The answer is one
/// table a relation, in the request's order.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Request<'r> {
    pub(crate) relations: &'r [Asked<'r>],
}

impl Request<'_> {
    /// File and closed-project evaluation consume the same ordered provider reply.
    pub(super) fn read_into(&self, run: &mut Run, provider: &mut dyn Provider, out: &mut Vec<R<Table>>) -> R<()> {
        out.clear();
        provider.answer(run, self, out)?;
        if out.len() != self.relations.len() {
            return Err(format!("the provider answered {} relations of {}", out.len(), self.relations.len()));
        }
        Ok(())
    }
}

/// The run's request state: the relations a request can name (by number), each emit root's reach over them (made at its
/// first file), and the counts.
#[derive(Default)]
pub(crate) struct Requests {
    /// The relations a request can name, by number, and their number by name.
    names: Vec<String>,
    by_name: FxHashMap<String, u16>,
    /// Who supplies each (the host or the provider), by number.
    host: Vec<bool>,
    /// One immutable empty table per declared schema, shared by certified absences.
    empty: Vec<Option<Rc<Table>>>,
    run_columns: Vec<Columns>,
    suppliers: Vec<Supplier>,
    pub(super) base_slots: Vec<Option<u16>>,
    pub(super) proven_empty: Vec<bool>,
    pub(super) guards: Guards,
    pub(super) empty_views: FxHashMap<usize, std::rc::Rc<super::ViewSpec>>,
    nodes: Vec<Option<u32>>,
    graph: Vec<Entry>,
    shared: FxHashMap<Entry, u32>,
    reads: Vec<(u16, Columns)>,
    read_ids: FxHashMap<(u16, Columns), u32>,
    visited: Vec<bool>,
    pending: Vec<u32>,
    masks: Vec<Option<Columns>>,
    asked: Vec<u16>,
    answer: Vec<Result<Table, String>>,
    // A run-scoped answer counts once; any file can consume it.
    run_unused: Vec<Option<u64>>,
    pub(super) counts: Counts,
}

/// Deterministic counts of the run's requests.
#[derive(Default, Debug)]
pub(crate) struct Counts {
    pub(super) files: u64,
    idle_files: u64,
    pub(super) provider_calls: u64,
    pub(super) provider_relations: u64,
    pub(super) host_relations: u64,
    pub(super) rows: u64,
    /// Relations the provider refused for the file (a step reading one faults, as the language says).
    pub(super) refused: u64,
    /// Relations answered that no step of the file asked (rows and size never consulted).
    pub(super) never_asked: u64,
    pub(super) never_asked_rows: u64,
    misses: u64,
    reads: u64,
    request_cpu: f64,
    mask_cpu: f64,
    answer_cpu: f64,
    read_cpu: f64,
}

impl Requests {
    fn number(&mut self, run: &Run, rel: &str) -> u16 {
        if let Some(n) = self.by_name.get(rel) {
            return *n;
        }
        let n = self.names.len() as u16;
        self.names.push(rel.to_owned());
        self.by_name.insert(rel.to_owned(), n);
        let supplier = run.supplier(rel);
        self.host.push(matches!(supplier, Supplier::Host));
        self.suppliers.push(supplier);
        n
    }

    /// The counts' line (printed at the run's end).
    pub(super) fn line(&self) -> String {
        let c = &self.counts;
        format!(
            "REQUEST files {} provider-calls {} provider-relations {} host-relations {} rows {} refused {} never-asked {} ({} rows); relations nameable {}; missing {} first reads {}; request {:.6} answer {:.6} read {:.6} s; selecting columns {:.6} s; file predicates {} leaves {} gates; files without active emits {idle}",
            c.files,
            c.provider_calls,
            c.provider_relations,
            c.host_relations,
            c.rows,
            c.refused,
            c.never_asked + self.run_unused.iter().flatten().count() as u64,
            c.never_asked_rows + self.run_unused.iter().flatten().sum::<u64>(),
            self.names.len(),
            c.misses,
            c.reads,
            c.request_cpu,
            c.answer_cpu,
            c.read_cpu, c.mask_cpu, self.guards.circuit.counts().0, self.guards.circuit.counts().1,
            idle = c.idle_files
        )
    }
}

/// A materialized operator reads its inputs whole. A select reads its view sources
/// in place, through their predicates and the columns of its expressions.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
enum Action {
    Read(u32),
    Inputs([u32; 2]),
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct Entry {
    guard: u32,
    action: Action,
}

// A native input contributes no provider request.
const NONE: u32 = u32::MAX;

impl Requests {
    fn add(&mut self, guard: u32, action: Action) -> u32 {
        let entry = Entry { guard, action };
        *self.shared.entry(entry).or_insert_with(|| {
            let id = self.graph.len() as u32;
            self.graph.push(entry);
            id
        })
    }

    /// The file's active roots are already known. Keep their shared input graph
    /// and compile only the predicates that test actual file evidence.
    fn compile_guards(&mut self) {
        let literals = self.guards.compile(self.graph.iter().map(|entry| entry.guard));
        for entry in &mut self.graph {
            entry.guard = literals[entry.guard as usize];
        }
        self.visited.resize(self.graph.len(), false);
        self.shared = FxHashMap::default();
        self.read_ids = FxHashMap::default();
    }

    #[allow(dead_code, reason = "SDK hosts inspect the request graph before their shared row packet is indexed")]
    pub(crate) fn provider_slots(&self) -> impl Iterator<Item = (u16, &str)> {
        self.names.iter().zip(&self.host).enumerate().filter_map(|(slot, (name, host))| {
            (!*host).then_some((slot as u16, name.as_str()))
        })
    }

    /// Before prefix decisions exist, the parser may need any read below an
    /// admitted emit. Use the file request graph with its guards open.
    #[allow(dead_code, reason = "SDK hosts inspect the request graph before their shared row packet is indexed")]
    pub(crate) fn possible_provider_reads(&mut self, node: usize, mut visit: impl FnMut(&str)) {
        let Some(root) = self.nodes[node].filter(|&id| id != NONE) else { return };
        self.visited.fill(false);
        self.pending.push(root);
        while let Some(id) = self.pending.pop() {
            let i = id as usize;
            if std::mem::replace(&mut self.visited[i], true) { continue; }
            match self.graph[i].action {
                Action::Inputs(children) => self.pending.extend_from_slice(&children),
                Action::Read(read) => {
                    let slot = usize::from(self.reads[read as usize].0);
                    if !self.host[slot] { visit(&self.names[slot]); }
                }
            }
        }
    }

    fn read(&mut self, guard: u32, slot: u16, columns: Columns) -> u32 {
        let next = self.reads.len() as u32;
        let read = *self.read_ids.entry((slot, columns)).or_insert_with_key(|key| {
            self.reads.push(key.clone());
            next
        });
        self.add(guard, Action::Read(read))
    }

    // Store both children inline. A wider operator uses a balanced tree of
    // the same conjunction-free branches, with its guard only at the root.
    fn group(&mut self, guard: u32, inputs: &[u32]) -> u32 {
        if let [one] = inputs { return *one; }
        let (left, right) = inputs.split_at(inputs.len() / 2);
        let a = self.group(ALWAYS, left);
        let b = self.group(ALWAYS, right);
        self.add(guard, Action::Inputs([a, b]))
    }

    fn branch(&mut self, guard: u32, mut inputs: Vec<u32>) -> u32 {
        inputs.retain(|i| *i != NONE);
        inputs.sort_unstable();
        inputs.dedup();
        match inputs.as_slice() {
            [] => NONE,
            [one] => {
                let child = &self.graph[*one as usize];
                if guard == ALWAYS || guard == child.guard {
                    return *one;
                }
                let action = child.action;
                let both = self.guards.combine(true, vec![guard, child.guard]);
                self.add(both, action)
            }
            _ => self.group(guard, &inputs),
        }
    }
}

fn bit(c: usize) -> u64 {
    1u64.checked_shl(c as u32).unwrap_or(u64::MAX)
}

pub(crate) fn wants(mask: u64, column: usize) -> bool {
    mask & bit(column) != 0
}

fn select_columns(e: &E, source: usize, columns: &mut Columns) {
    match e {
        E::Col(s, c, _) if *s == source => columns.insert(*c),
        E::List(es) | E::Call(_, es) => { for e in es { select_columns(e, source, columns); } }
        E::Cmp(_, a, b) | E::In(a, b, _) | E::And(a, b) | E::Or(a, b) => {
            select_columns(a, source, columns);
            select_columns(b, source, columns);
        }
        E::Not(x) | E::Null(x, _) | E::Re(x, _) | E::Seed(x, _) => select_columns(x, source, columns),
        _ => {}
    }
}

impl Exec<'_> {
    /// Fix one slot for each catalog relation in this plan before the first file.
    /// A base and a view of that base address the same read-only answer.
    pub(crate) fn prepare_slots(&mut self, run: &mut Run, emits: &[usize], drivers: &[(usize, usize)]) {
        self.requests.guards.register_drivers(drivers);
        self.requests.base_slots.resize(self.plan.nodes.len(), None);
        for (i, node) in self.plan.nodes.iter().enumerate() {
            if node.op != "base" { continue; }
            let Some(rel) = node.base() else { continue };
            if native_of(run, rel).is_some() || matches!(run.supplier(rel), Supplier::RunFiles) { continue; }
            let slot = self.requests.number(run, rel);
            self.requests.base_slots[i] = Some(slot);
            if usize::from(slot) == run.fact_rels.len() {
                let empty = run.catalog(rel).map(|(_, names, tys)| Rc::new(Table::projected(names, tys, 0, 0)));
                let width = empty.as_ref().map(|table| table.names.len());
                self.requests.empty.push(empty);
                run.fact_rels.push((rel.to_owned(), width.unwrap_or(0)));
                if width.is_some() { run.fact_slots.insert(rel.to_owned(), slot); }
            }
        }
        let mut offsets = Vec::with_capacity(run.fact_rels.len() + 1);
        offsets.push(0);
        for (_, width) in &run.fact_rels {
            offsets.push(offsets.last().copied().unwrap() + width);
        }
        run.fact_offsets = offsets.into();
        run.project.resize_with(self.requests.names.len(), || None);
        self.requests.run_unused.resize(self.requests.names.len(), None);
        // A run-scoped answer serves files whose live rules can ask for
        // different columns. Include the file partition key and every column
        // the plan may read from that answer over its whole lifetime.
        self.requests.nodes.resize(self.plan.nodes.len(), None);
        self.requests.guards.begin(self.plan.nodes.len());
        for &emit in emits { self.demand(run, self.plan.emits[emit].1); }
        self.requests.run_columns.resize_with(self.requests.names.len(), Columns::default);
        for (slot, mask) in &self.requests.reads {
            self.requests.run_columns[usize::from(*slot)].extend(mask);
        }
        self.requests.compile_guards();
        self.requests.proven_empty.resize(self.requests.names.len(), false);

    }

    /// A completed part of the walk can already know a catalog relation is empty.
    pub(crate) fn walk_empty(&mut self, run: &Run, rel: &str) {
        if let Some(&slot) = run.fact_slots.get(rel) { self.walk_empty_slot(slot); }
    }

    pub(crate) fn walk_empty_slot(&mut self, slot: u16) {
        self.requests.proven_empty[usize::from(slot)] = true;
    }

    fn demand_input(&mut self, run: &mut Run, n: usize, mask: Columns) -> u32 {
        let Ok(Some(v)) = self.view(run, n) else { return self.demand(run, n) };
        let Native::Fact(i) = v.base else { return NONE };
        let slot = i;
        let mut cols = Columns::default();
        for e in &v.filter { select_columns(e, 0, &mut cols); }
        for (c, base) in v.map.iter().enumerate() {
            if mask.contains(c) {
                cols.insert(*base);
            }
        }
        let guard = self.guard(run, n);
        self.requests.read(guard, slot, cols)
    }

    fn demand(&mut self, run: &mut Run, n: usize) -> u32 {
        if let Some(d) = self.requests.nodes[n] {
            return d;
        }
        let pn = &self.plan.nodes[n];
        let guard = self.guard(run, n);
        let d = if self.view(run, n).ok().flatten().is_some() {
            self.demand_input(run, n, Columns::All)
        } else if pn.op == "base" {
            if let Some(slot) = self.requests.base_slots[n] {
                self.requests.read(guard, slot, Columns::All)
            } else {
                NONE
            }
        } else {
            let mut inputs = Vec::new();
            if pn.op == "select"
                && let Ok(sp) = self.sel(run, n)
            {
                // A keyed select passes its caller's keys to a projected source.
                // A native probe gathers every mapped view column before selecting.
                let keyed = match self.keyed_form(run, n) {
                    Ok(super::keyed::Form::Select(cols)) => Some(cols),
                    _ => None,
                };
                for (i, src) in sp.srcs.iter().enumerate() {
                    let probed = keyed.as_ref().is_some_and(|cols| {
                        cols.iter().flatten().any(|(source, _)| *source == i)
                    }) || sp.conj.iter().any(|e| matches!(e, E::Seed(x, _) if matches!(**x, E::Col(s, _, _) if s == i)));
                    let mask = if probed {
                        Columns::All
                    } else {
                        let mut mask = Columns::default();
                        for e in sp.conj.iter().chain(sp.items.iter().map(|(_, e)| e)) { select_columns(e, i, &mut mask); }
                        mask
                    };
                    inputs.push(self.demand_input(run, *src, mask));
                }
            } else {
                // The whole forms check the left before opening the right.
                // Keyed antijoins also size and probe the left first. Evidence
                // applies only to the conditional read, never to the left work.
                let driven = matches!(pn.op.as_str(), "antijoin" | "ordered-match" | "as-of");
                let first = self.frames[n].first as usize;
                for &src in &pn.inputs {
                    let input = self.demand_input(run, src, Columns::All);
                    if driven && src != first && first < self.plan.nodes.len() {
                        let guard = self.requests.guards.driver(first);
                        inputs.push(self.requests.branch(guard, vec![input]));
                    } else {
                        inputs.push(input);
                    }
                }
                if self.frames[n].counted {
                    // A counted select may emit a kept row with zero partners.
                    let sel = self.frames[n].first as usize;
                    for &src in &self.plan.nodes[sel].inputs {
                        inputs.push(self.demand_input(run, src, Columns::All));
                    }
                }
            }
            self.requests.branch(guard, inputs)
        };
        self.requests.nodes[n] = Some(d);
        d
    }

    fn request_masks(&mut self, f: &File<'_>, run: &mut Run, roots: &[usize]) -> Vec<Option<Columns>> {
        let mut masks = std::mem::take(&mut self.requests.masks);
        masks.resize(self.requests.names.len(), None);
        masks.fill(None);
        // A file with no running emit still receives a packet. Independent
        // diagnostic asks are appended later with their own column demand.
        self.requests.guards.circuit.begin_file();
        if roots.is_empty() {
            self.requests.counts.idle_files += 1;
            return masks;
        }
        let mut visited = std::mem::take(&mut self.requests.visited);
        let mut pending = std::mem::take(&mut self.requests.pending);
        let rq = &self.requests;
        visited.fill(false);
        pending.extend(roots.iter().filter_map(|&node| rq.nodes[node]).filter(|&id| id != NONE));
        while let Some(id) = pending.pop() {
            let i = id as usize;
            if std::mem::replace(&mut visited[i], true) { continue; }
            let entry = &rq.graph[i];
            match &entry.action {
                Action::Inputs(children) => {
                    if self.allows(f, run, entry.guard) {
                        pending.extend_from_slice(children);
                    }
                }
                Action::Read(read) => {
                    let (slot, columns) = &rq.reads[*read as usize];
                    let slot = usize::from(*slot);
                    if rq.proven_empty.get(slot).copied().unwrap_or(false)
                        || masks[slot].as_ref().is_some_and(|m| m.covers(columns)) { continue; }
                    if self.allows(f, run, entry.guard) {
                        masks[slot].get_or_insert_with(Columns::default).extend(columns);
                    }
                }
            }
        }
        self.requests.visited = visited;
        self.requests.pending = pending;
        masks
    }

    /// The file's one request: the union of what the steps under `roots` (the alive rules' emits) can read, asked of the
    /// provider in one call and of the host for its own; the answers kept in the file for `answered`.
    pub(crate) fn open_file(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, roots: &[usize], checks: &[&str]) -> R<Vec<R<Table>>> {
        let timed = super::fill_prof::timed();
        let start = if timed { super::super::thread_cpu() } else { 0.0 };
        let mut masks = self.request_masks(f, run, roots);
        if timed { self.requests.counts.mask_cpu += super::super::thread_cpu() - start; }
        let rq = &mut self.requests;
        rq.counts.files += 1;
        let mut asked = std::mem::take(&mut rq.asked);
        asked.clear();
        f.facts.resize_with(rq.names.len(), || None);
        f.fact_read.resize_with(rq.names.len(), Default::default);
        f.answer.clear();
        f.answer.resize_with(rq.names.len(), || None);
        let mut scopes = FxHashMap::default();
        for (n, mask) in masks.iter().enumerate() {
            if mask.is_some() && !rq.proven_empty.get(n).copied().unwrap_or(false) {
                let n = n as u16;
                if rq.host[usize::from(n)] {
                    let rel = &rq.names[usize::from(n)];
                    let t = host::host_fill(run, rel, f.file).unwrap_or_else(|| Err(format!("the host supplies no {rel}")))?;
                    rq.counts.host_relations += 1;
                    f.answer[usize::from(n)] = Some(Ok(t));
                } else {
                    if let Supplier::Project(ask) = rq.suppliers[usize::from(n)] {
                        if super::project::covers(run, n, f.file) {
                            f.answer[usize::from(n)] = Some(super::project::rows(f, run, n));
                            continue;
                        }
                        if matches!(ask, super::project::ProjectAsk::Run) {
                            let pending: Vec<u32> = (0..run.paths.len() as u32).filter(|file| !super::project::covers(run, n, *file)).collect();
                            scopes.insert(n, pending);
                        }
                    }
                    asked.push(n);
                }
            }
        }
        let checked = {
            let singleton = [f.file];
            for &n in &asked {
                let i = usize::from(n);
                if let Supplier::Project(ask) = rq.suppliers[i] {
                    let mask = masks[i].as_mut().unwrap();
                    if matches!(ask, super::project::ProjectAsk::Run) { mask.clone_from(&rq.run_columns[i]); }
                    if let Some(k) = rq.empty[i].as_ref().and_then(|table| table.tys.iter().position(|ty| *ty == super::Ty::File)) { mask.insert(k); }
                }
            }
            let mut list: Vec<Asked<'_>> = asked
                .iter()
                .map(|n| {
                    let rel = rq.names[usize::from(*n)].as_str();
                    let scope = match rq.suppliers[usize::from(*n)] {
                        Supplier::Project(super::project::ProjectAsk::Run) => Scope::Files(&scopes[n]),
                        Supplier::Project(super::project::ProjectAsk::EachFile) => Scope::Files(&singleton),
                        _ => Scope::File(f.file),
                    };
                    Asked {
                        rel,
                        scope,
                        columns: masks[usize::from(*n)].as_ref().unwrap(),
                    }
                })
                .collect();
            list.extend(checks.iter().map(|rel| Asked { rel, scope: Scope::File(f.file), columns: &Columns::All }));
            let mut answer = std::mem::take(&mut rq.answer);
            let before = if timed { super::super::thread_cpu() } else { 0.0 };
            Request { relations: &list }.read_into(run, p, &mut answer)?;
            if timed {
                rq.counts.answer_cpu += super::super::thread_cpu() - before;
                rq.counts.request_cpu += before - start;
            }
            rq.counts.provider_calls += 1;
            rq.counts.provider_relations += list.len() as u64;
            let checked = answer.split_off(asked.len());
            for t in &checked {
                match t {
                    Ok(t) => rq.counts.rows += t.n as u64,
                    Err(_) => rq.counts.refused += 1,
                }
            }
            for ((n, entry), t) in asked.iter().zip(&list).zip(answer.drain(..)) {
                match &t {
                    Ok(t) => rq.counts.rows += t.n as u64,
                    Err(_) => rq.counts.refused += 1,
                }
                if matches!(rq.suppliers[usize::from(*n)], Supplier::Project(super::project::ProjectAsk::Run))
                    && let Ok(table) = &t {
                    rq.run_unused[usize::from(*n)] = Some(table.n as u64);
                }
                let result = match entry.scope {
                    Scope::File(_) => t,
                    Scope::Files(files) => t.and_then(|t| {
                        let each = matches!(rq.suppliers[usize::from(*n)], Supplier::Project(super::project::ProjectAsk::EachFile));
                        super::project::remember(run, *n, each, files, t)?;
                        super::project::rows(f, run, *n)
                    }),
                };
                f.answer[usize::from(*n)] = Some(result);
            }
            rq.answer = answer;
            checked
        };
        rq.asked = asked;
        rq.masks = masks;
        Ok(checked)
    }

    /// A relation's table from the file's answer (moved out: the caller keeps it as the file's fill), or an error naming a
    /// relation the request did not hold.
    pub(super) fn answered(&mut self, f: &mut File<'_>, i: usize) -> R<Rc<Table>> {
        let timed = super::fill_prof::timed();
        let t0 = if timed { super::super::thread_cpu() } else { 0.0 };
        if self.requests.proven_empty.get(i).copied().unwrap_or(false) {
            return self.requests.empty[i].clone().ok_or_else(|| "unknown relation declared empty".to_owned());
        }
        let slot = f.answer.get_mut(i);
        let result = match slot {
            Some(slot @ Some(Ok(_))) => {
                if matches!(self.requests.suppliers[i], Supplier::Project(super::project::ProjectAsk::Run)) {
                    self.requests.run_unused[i] = None;
                }
                slot.take().unwrap().map(super::recycle::rc)
            }
            Some(Some(Err(err))) => Err(err.clone()),
            _ => {
                let rel = &self.requests.names[i];
                self.requests.counts.misses += 1;
                if super::fill_prof::counting() && self.requests.counts.misses <= 4 {
                    eprintln!("REQUEST-MISS file {} relation {rel}", f.file);
                }
                Err(format!("{rel}: not in this file's request"))
            }
        };
        self.requests.counts.reads += 1;
        if timed {
            self.requests.counts.read_cpu += super::super::thread_cpu() - t0;
        }
        result
    }

    /// The file's end: the answered relations no step asked, counted.
    pub(crate) fn close_file(&mut self, f: &mut File<'_>) {
        self.requests.proven_empty.fill(false);
        self.requests.guards.clear_evidence();
        for &slot in &self.requests.asked {
            let i = usize::from(slot);
            if matches!(self.requests.suppliers[i], Supplier::Project(super::project::ProjectAsk::Run)) { continue; }
            if let Some(Ok(table)) = &f.answer[i] {
                self.requests.counts.never_asked += 1;
                self.requests.counts.never_asked_rows += table.n as u64;
            }
        }
        f.answer.clear();
    }

    /// The run's request counts.
    pub(crate) fn request_line(&self) -> String {
        self.requests.line()
    }
}
