//! Bind provider reads to the typed plan and admit one ordered packet per file.
use super::{E, Exec, File, Native, Provider, R, Run, Scope, Supplier, Table, native_of, project};
use rustc_hash::FxHashMap;
use ryo::languages::ColumnProjection as Columns;
use std::rc::Rc;

pub(crate) struct Asked<'a> {
    pub(crate) relation: &'a str,
    pub(crate) columns: &'a Columns,
    pub(crate) scope: Scope<'a>,
}

pub(crate) type Reply = R<Answer>;

/// An indexed answer is already complete in the reader's file packet. Its
/// selected words are widened into executor columns when the frame reads it.
pub(crate) enum Answer {
    Columns(Rc<Table>),
    #[allow(dead_code, reason = "only the indexed SDK hosts construct this reply")]
    Indexed { relation: ryo::languages::Relation, columns: Columns, rows: usize },
}

impl Answer {
    pub(super) fn count(&self) -> usize {
        match self {
            Self::Columns(table) => table.n,
            Self::Indexed { rows, .. } => *rows,
        }
    }

    pub(crate) fn columns(self) -> R<Rc<Table>> {
        match self {
            Self::Columns(table) => Ok(table),
            Self::Indexed { .. } => Err("closed scope requires owned column tables".into()),
        }
    }
}

pub(crate) fn read(run: &mut Run, provider: &mut dyn Provider, asks: &[Asked<'_>], replies: &mut Vec<Reply>) -> R<()> {
    replies.clear();
    provider.answer(run, asks, replies);
    if replies.len() == asks.len() { Ok(()) }
    else { Err(format!("provider returned {} answers for {} requests", replies.len(), asks.len())) }
}

/// Decode columns from immutable, already supplied file data.
pub(crate) trait Indexed {
    fn widen(&self, run: &mut Run, relation: ryo::languages::Relation, columns: &Columns) -> R<Table>;
}

#[derive(Default)]
pub(super) struct Requests {
    by_step: Vec<u32>,
    graph: Vec<Demand>,
    shared: FxHashMap<Demand, u32>,
    seen: Vec<bool>,
    pending: Vec<u32>,
    masks: Vec<Option<Columns>>,
    empty: Vec<Option<Rc<Table>>>,
    packet_pending: Vec<Pending>,
    #[allow(dead_code, reason = "used by the check-session host")]
    indexed_slots: Vec<Option<u16>>,
    #[allow(dead_code, reason = "used by the check-session host")]
    host_slots: Vec<u16>,
    replies: Vec<Reply>,
    pub(super) files: u64,
    pub(super) calls: u64,
    pub(super) answered: u64,
    pub(super) refused: u64,
    pub(super) rows: u64,
    pub(super) unread: u64,
    pub(super) missing: u64,
}

const NO_READ: u32 = u32::MAX;
const UNBOUND: u32 = u32::MAX - 1;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Demand {
    /// An existing executor empty decision, indexed by its typed plan step.
    guard: Option<usize>,
    reads: Reads,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum Reads {
    Base(u16, Columns),
    Inputs(Box<[u32]>),
}

struct Pending {
    slot: usize,
    relation: Rc<str>,
    project: Option<project::ProjectAsk>,
    files: Option<Vec<u32>>,
}

fn columns(expr: &E, source: usize, out: &mut Columns) {
    match expr {
        E::Col(s, c, _) if *s == source => out.insert(*c),
        E::List(args) | E::Call(_, args) => {
            for arg in args { columns(arg, source, out); }
        }
        E::Cmp(_, a, b) | E::In(a, b, _) | E::And(a, b) | E::Or(a, b) => {
            columns(a, source, out);
            columns(b, source, out);
        }
        E::Not(a) | E::TextTest(a, _) => columns(a, source, out),
        _ => {}
    }
}

pub(super) fn slot(run: &mut Run, relation: &str) -> u16 {
    if let Some(&id) = run.fact_slots.get(relation) { return id; }
    let id = u16::try_from(run.fact_rels.len()).expect("provider relation slots");
    let width = run.catalog(relation).map_or(0, |(_, names, _)| names.len());
    run.fact_rels.push((Rc::from(relation), width));
    run.fact_slots.insert(relation.to_owned(), id);
    id
}

impl Exec<'_> {
    /// Compile provider-bearing branches the first time a rule survives screening.
    fn bind_provider_reads(&mut self, run: &mut Run, roots: impl IntoIterator<Item = usize>, prune: bool) {
        self.requests.by_step.resize(self.plan.nodes.len(), UNBOUND);
        let mut shared = std::mem::take(&mut self.requests.shared);
        for node in roots {
            self.bind_request(run, node, &mut shared, prune);
        }
        self.requests.shared = shared;
        self.requests.seen.resize(self.requests.graph.len(), false);
    }

    fn demand(&mut self, guard: Option<usize>, reads: Reads, shared: &mut FxHashMap<Demand, u32>) -> u32 {
        let demand = Demand { guard, reads };
        *shared.entry(demand).or_insert_with_key(|demand| {
            let id = self.requests.graph.len() as u32;
            self.requests.graph.push(demand.clone());
            id
        })
    }

    fn guard(&mut self, run: &mut Run, node: usize, prune: bool) -> Option<usize> {
        if !prune || self.plan.nodes[node].counted() { return None; }
        self.empty_guard(run, node)
    }

    fn bind_request(&mut self, run: &mut Run, node: usize, shared: &mut FxHashMap<Demand, u32>, prune: bool) -> u32 {
        let known = self.requests.by_step[node];
        if known != UNBOUND {
            return known;
        }
        self.requests.by_step[node] = NO_READ;
        let plan = self.plan;
        let plan = &plan.nodes[node];
        let demand = if let Some(rel) = plan.base() {
            if native_of(run, rel).is_some() || matches!(run.supplier(rel), Supplier::RunFiles) {
                NO_READ
            } else {
                let slot = slot(run, rel);
                let guard = self.guard(run, node, prune);
                self.demand(guard, Reads::Base(slot, Columns::All), shared)
            }
        } else {
            let mut inputs: Vec<_> = plan.inputs.iter().copied()
                .map(|input| (input, self.bind_request(run, input, shared, prune)))
                .filter(|&(_, read)| read != NO_READ)
                .collect();
            if inputs.is_empty() {
                return NO_READ;
            }
            if plan.op == "select" && plan.inputs.len() == 1 && !plan.counted()
                && let Ok(select) = self.sel(run, node)
                && select.srcs.len() == 1
                && matches!(self.keyed_form(run, node), Ok(super::keyed::Form::Not))
                && let Ok(Some(view)) = self.view(run, select.srcs[0])
                && let Native::Fact(slot) = view.base
                && let Some((input, entry)) = inputs.iter_mut().find(|(input, _)| *input == select.srcs[0])
            {
                let mut mask = Columns::default();
                for expr in select.conj.iter().chain(select.items.iter().map(|(_, e)| e)) {
                    columns(expr, 0, &mut mask);
                }
                let mut base_columns = Columns::default();
                for expr in &view.filter { columns(expr, 0, &mut base_columns); }
                for (column, &base) in view.map.iter().enumerate() {
                    if mask.contains(column) { base_columns.insert(base); }
                }
                let guard = self.guard(run, *input, prune);
                *entry = self.demand(guard, Reads::Base(slot, base_columns), shared);
            }
            let guard = self.guard(run, node, prune);
            if let [(_, only)] = inputs.as_slice()
                && (guard.is_none() || self.requests.graph[*only as usize].guard == guard)
            {
                *only
            } else {
                self.demand(guard, Reads::Inputs(inputs.into_iter().map(|(_, demand)| demand).collect()), shared)
            }
        };
        self.requests.by_step[node] = demand;
        demand
    }

    fn collect_request(&mut self, run: &mut Run, file: &mut File<'_>, provider: &mut dyn Provider,
        roots: impl IntoIterator<Item = usize>, prune: bool) {
        self.requests.seen.fill(false);
        self.requests.pending.extend(roots.into_iter().filter_map(|node| {
            let id = self.requests.by_step[node];
            debug_assert_ne!(id, UNBOUND);
            (id != NO_READ).then_some(id)
        }));
        self.requests.pending.reverse();
        let graph = std::mem::take(&mut self.requests.graph);
        while let Some(id) = self.requests.pending.pop() {
            if std::mem::replace(&mut self.requests.seen[id as usize], true) {
                continue;
            }
            let demand = &graph[id as usize];
            if prune && demand.guard.is_some_and(|node| self.static_empty(file, run, provider, node)) {
                continue;
            }
            match &demand.reads {
                Reads::Base(slot, columns) => self.requests.masks[usize::from(*slot)]
                    .get_or_insert_with(Columns::default).extend(columns),
                Reads::Inputs(inputs) => self.requests.pending.extend(inputs.iter().rev().copied()),
            }
        }
        self.requests.graph = graph;
    }

    /// The request owns scopes and projection masks only until the provider returns.
    /// Results belong to the shared file version, including refusals for that file.
    pub(crate) fn open_file(&mut self, file: &mut File<'_>, run: &mut Run, provider: &mut dyn Provider,
        roots: impl IntoIterator<Item = usize> + Clone, checks: &[&str], empty: Option<&[&str]>) -> R<Vec<Reply>> {
        let prune = empty.is_some() && !sw!("VEC_NO_KNOWN") && !sw!("VEC_NO_SEMI");
        self.bind_provider_reads(run, roots.clone(), prune);
        self.requests.masks.resize_with(run.fact_rels.len(), || None);
        self.requests.masks.fill(None);
        if let Some(empty) = empty {
            let mut proven = file.version.proven.borrow_mut();
            proven.resize(run.fact_rels.len(), false);
            for name in empty {
                if let Some(&slot) = run.fact_slots.get(*name) { proven[usize::from(slot)] = true; }
            }
        }
        file.admitting = true;
        self.collect_request(run, file, provider, roots, prune);
        file.admitting = false;
        if prune { file.known.iter_mut().filter(|value| **value == super::known::NO_LATE).for_each(|value| *value = 0); }
        self.admit_packet(file.version, run, provider, checks)
    }

    /// The indexed reader has already bound these typed relation demands
    /// for the active bindings. Reuse its admission rather than binding a
    /// second execution plan merely to describe the existing packet.
    #[allow(dead_code, reason = "the session host already binds its admission relations")]
    pub(crate) fn open_indexed(&mut self, version: &super::FileVersion<'_>, run: &mut Run,
        provider: &mut dyn Provider, relations: &[ryo::languages::Relation], checks: &[&str]) -> R<Vec<Reply>> {
        use ryo_relations::domain::relations::service::{ALL, name, number_of};
        if self.requests.indexed_slots.is_empty() {
            self.requests.indexed_slots.resize(ALL.len(), None);
            let hosts: Vec<_> = run.declared.iter()
                .filter(|(_, declaration)| matches!(declaration.supplier, Supplier::Host))
                .map(|(relation, _)| relation.clone()).collect();
            self.requests.host_slots.extend(hosts.iter().map(|relation| slot(run, relation)));
        }
        for &relation in relations {
            self.requests.indexed_slots[number_of(relation).0].get_or_insert_with(|| slot(run, name(relation).0));
        }
        self.requests.masks.resize_with(run.fact_rels.len(), || None);
        self.requests.masks.fill(None);
        let slots = relations.iter().map(|&r| self.requests.indexed_slots[number_of(r).0].expect("bound relation"));
        for id in slots.chain(self.requests.host_slots.iter().copied()) {
            self.requests.masks[usize::from(id)] = Some(Columns::All);
        }
        self.admit_packet(version, run, provider, checks)
    }

    fn admit_packet(&mut self, version: &super::FileVersion<'_>, run: &mut Run, provider: &mut dyn Provider, checks: &[&str]) -> R<Vec<Reply>> {
        let counted = super::fill_prof::counting();
        if counted { self.requests.files += 1; }
        let mut packet = version.facts.borrow_mut();
        packet.resize_with(run.fact_rels.len(), || None);
        self.requests.empty.resize_with(run.fact_rels.len(), || None);
        let proven = version.proven.borrow();
        let pending = &mut self.requests.packet_pending;
        pending.clear();
        for (id, mask) in self.requests.masks.iter().enumerate() {
            if mask.is_none() { continue; }
            let relation = &run.fact_rels[id].0;
            if packet[id].is_some() || version.supplied.contains_key(relation.as_ref()) { continue; }
            if proven.get(id).copied().unwrap_or(false) {
                let empty = &mut self.requests.empty[id];
                if empty.is_none() {
                    let relation = relation.clone();
                    *empty = run.catalog(&relation).map(|(_, names, types)|
                        Rc::new(Table::with_projection(names, types, 0, &Columns::default())));
                }
                let reply = empty.clone().map(Answer::Columns).ok_or_else(|| format!("undeclared {}", run.fact_rels[id].0));
                packet[id] = Some(Delivered::new(reply, false));
                continue;
            }
            match run.supplier(relation) {
                Supplier::Host => {
                    let relation = relation.clone();
                    let result = super::host::host_fill(run, &relation, version.file)
                        .unwrap_or_else(|| Err(format!("the host supplies no {relation}")));
                    packet[id] = Some(Delivered::new(result.map(super::recycle::rc).map(Answer::Columns), false));
                }
                Supplier::Project(ask) => {
                    if project::covered(run, relation, version.file) {
                        let relation = relation.clone();
                        let table = project::existing(run, &relation, version.file, ask);
                        packet[id] = Some(Delivered::new(table.map(super::recycle::rc).map(Answer::Columns), false));
                    } else {
                        let files = match ask {
                            project::ProjectAsk::Run => (0..run.paths.len() as u32).collect(),
                            project::ProjectAsk::EachFile => vec![version.file],
                        };
                        pending.push(Pending { slot: id, relation: relation.clone(), project: Some(ask), files: Some(files) });
                    }
                }
                _ => pending.push(Pending { slot: id, relation: relation.clone(), project: None, files: None }),
            }
        }
        let list: Vec<_> = pending.iter().map(|p| Asked {
            relation: &p.relation,
            columns: if p.project.is_some() { &Columns::All } else { self.requests.masks[p.slot].as_ref().expect("selected relation") },
            scope: p.files.as_deref().map_or(Scope::File(version.file), Scope::Files),
        }).chain(checks.iter().map(|relation| Asked {
            relation, columns: &Columns::All, scope: Scope::File(version.file),
        })).collect();
        if counted { self.requests.calls += 1; }
        read(run, provider, &list, &mut self.requests.replies)?;
        let returned = pending.len();
        drop(list);
        for (Pending { slot, project, relation, .. }, reply) in pending.drain(..).zip(self.requests.replies.drain(..returned)) {
            if counted {
                self.requests.answered += u64::from(reply.is_ok());
                self.requests.refused += u64::from(reply.is_err());
                self.requests.rows += reply.as_ref().map_or(0, |t| t.count() as u64);
            }
            let reply = if let Some(ask) = project {
                project::for_file(version.file, run, &relation, ask, |_, _| reply.and_then(Answer::columns).map(|t|
                    Rc::try_unwrap(t).unwrap_or_else(|t| t.copy()))).map(super::recycle::rc).map(Answer::Columns)
            } else {
                reply
            };
            packet[slot] = Some(Delivered::new(reply, counted));
        }
        Ok(self.requests.replies.drain(..).collect())
    }

    pub(crate) fn request_line(&self) -> String {
        let a = &self.requests;
        format!("REQUEST files {} provider-calls {} answered {} refused {} rows {} unread {} missing {}",
            a.files, a.calls, a.answered, a.refused, a.rows, a.unread, a.missing)
    }

    pub(crate) fn close_file(&mut self, version: &super::FileVersion<'_>) -> u64 {
        let packet = version.facts.borrow();
        let mut widened = 0;
        for answer in packet.iter().flatten() {
            if answer.count && answer.reply.is_ok() && !answer.read { self.requests.unread += 1; }
            widened += u64::from(answer.widened);
        }
        widened
    }

}

pub(super) struct Delivered {
    pub(super) reply: Reply,
    pub(super) read: bool,
    pub(super) columns_read: bool,
    count: bool,
    pub(super) widened: bool,
}
impl Delivered {
    fn new(reply: Reply, count: bool) -> Self { Self { reply, read: false, columns_read: false, count, widened: false } }
}
