//! Bind provider reads to the typed plan and admit one ordered packet per file.
use super::{E, Exec, File, Native, Provider, R, Run, Scope, Supplier, Table, native_of, project};
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
    fn count(&self) -> usize {
        match self {
            Self::Columns(table) => table.n,
            Self::Indexed { rows, .. } => *rows,
        }
    }

    pub(crate) struct Edge {
    input: usize,
    projected: Option<(u16, Columns)>,
}

fn columns(self) -> R<Rc<Table>> {
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
    steps: Vec<Option<Demand>>,
    seen: Vec<bool>,
    masks: Vec<Option<Columns>>,
    replies: Vec<Reply>,
    pub(super) files: u64,
    pub(super) calls: u64,
    pub(super) answered: u64,
    pub(super) refused: u64,
    pub(super) rows: u64,
    pub(super) unread: u64,
    pub(super) missing: u64,
}

enum Demand {
    Native,
    Base(u16),
    Inputs(Box<[Edge]>),
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
    run.fact_rels.push((relation.to_owned(), width));
    run.fact_slots.insert(relation.to_owned(), id);
    id
}

impl Exec<'_> {
    fn bind_request(&mut self, run: &mut Run, node: usize) {
        if self.requests.steps[node].is_some() { return; }
        let plan = &self.plan.nodes[node];
        // Mark before descending. Shared subtrees are bound once.
        self.requests.steps[node] = Some(Demand::Native);
        for &input in &plan.inputs { self.bind_request(run, input); }
        let demand = if let Some(rel) = plan.base() {
            if native_of(run, rel).is_some() || matches!(run.supplier(rel), Supplier::RunFiles) {
                Demand::Native
            } else {
                Demand::Base(slot(run, rel))
            }
        } else {
            let mut edges: Vec<_> = plan.inputs.iter().copied()
                .filter(|&input| !matches!(self.requests.steps[input], Some(Demand::Native)))
                .map(|input| Edge { input, projected: None }).collect();
            if edges.is_empty() {
                return;
            }
            if plan.op == "select" && !plan.counted()
                && let Ok(select) = self.sel(run, node)
                && select.srcs.len() == 1
                && matches!(self.keyed_form(run, node), Ok(super::keyed::Form::Not))
                && let Ok(Some(view)) = self.view(run, select.srcs[0])
                && let Native::Fact(slot) = view.base
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
                if let Some(edge) = edges.iter_mut().find(|edge| edge.input == select.srcs[0]) {
                    edge.projected = Some((slot, base_columns));
                }
            }
            Demand::Inputs(edges.into_boxed_slice())
        };
        self.requests.steps[node] = Some(demand);
    }

    fn collect_request(&mut self, run: &mut Run, file: &mut File<'_>, provider: &mut dyn Provider,
        steps: &[Option<Demand>], node: usize, prune: bool) {
        let demand = steps[node].as_ref().expect("bound request");
        if matches!(demand, Demand::Native) || std::mem::replace(&mut self.requests.seen[node], true) { return; }
        if prune && !self.plan.nodes[node].counted() && self.static_empty(file, run, provider, node) { return; }
        match demand {
            Demand::Native => {}
            Demand::Base(slot) => { self.requests.masks[usize::from(*slot)] = Some(Columns::All); }
            Demand::Inputs(edges) => {
                for edge in edges {
                    if let Some((slot, columns)) = &edge.projected {
                        if !prune || !self.static_empty(file, run, provider, edge.input) {
                            let mask = self.requests.masks[usize::from(*slot)].get_or_insert_with(Columns::default);
                            mask.extend(columns);
                        }
                    } else {
                        self.collect_request(run, file, provider, steps, edge.input, prune);
                    }
                }
            }
        }
    }

    /// The request owns scopes and projection masks only until the provider returns.
    /// Results belong to the shared file version, including refusals for that file.
    pub(crate) fn open_file(&mut self, file: &mut File<'_>, run: &mut Run, provider: &mut dyn Provider,
        roots: impl IntoIterator<Item = usize> + Clone, checks: &[&str], empty: Option<&[&str]>) -> R<Vec<Reply>> {
        self.requests.steps.resize_with(self.plan.nodes.len(), || None);
        self.requests.seen.resize(self.plan.nodes.len(), false);
        self.requests.seen.fill(false);
        for node in roots.clone() { self.bind_request(run, node); }
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
        let prune = empty.is_some() && !sw!("VEC_NO_KNOWN") && !sw!("VEC_NO_SEMI");
        let steps = std::mem::take(&mut self.requests.steps);
        for node in roots { self.collect_request(run, file, provider, &steps, node, prune); }
        self.requests.steps = steps;
        file.admitting = false;
        // A negative decision can change once the admitted fact columns are present.
        if prune { file.known.iter_mut().filter(|value| **value == super::known::NO).for_each(|value| *value = 0); }
        self.admit_packet(file.version, run, provider, checks)
    }

    fn admit_packet(&mut self, version: &super::FileVersion<'_>, run: &mut Run, provider: &mut dyn Provider, checks: &[&str]) -> R<Vec<Reply>> {
        self.requests.files += 1;
        let mut packet = version.facts.borrow_mut();
        packet.resize_with(run.fact_rels.len(), || None);
        let proven = version.proven.borrow();
        let mut pending = Vec::new();
        let mut scopes = Vec::new();
        for (id, mask) in self.requests.masks.iter().enumerate() {
            let Some(mask) = mask else { continue };
            let relation = &run.fact_rels[id].0;
            if packet[id].is_some() || version.supplied.contains_key(relation) { continue; }
            if proven.get(id).copied().unwrap_or(false) {
                let relation = relation.clone();
                let reply = run.catalog(&relation).map(|(_, names, types)|
                    Answer::Columns(Rc::new(Table::with_rows(names, types, 0)))).ok_or_else(|| format!("undeclared {relation}"));
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
                        pending.push((id, &Columns::All, Some(ask)));
                        scopes.push(Some(match ask {
                            project::ProjectAsk::Run => (0..run.paths.len() as u32).collect(),
                            project::ProjectAsk::EachFile => vec![version.file],
                        }));
                    }
                }
                _ => { pending.push((id, mask, None)); scopes.push(None); }
            }
        }
        let names: Vec<_> = pending.iter().map(|&(id, _, _)| run.fact_rels[id].0.clone()).collect();
        let list: Vec<_> = pending.iter().zip(&scopes).zip(&names).map(|(((_, columns, _), files), relation)| Asked {
            relation, columns, scope: files.as_deref().map_or(Scope::File(version.file), Scope::Files),
        }).chain(checks.iter().map(|relation| Asked {
            relation, columns: &Columns::All, scope: Scope::File(version.file),
        })).collect();
        self.requests.calls += 1;
        read(run, provider, &list, &mut self.requests.replies)?;
        let returned = pending.len();
        drop(list);
        for (((id, _, project), relation), reply) in pending.into_iter().zip(names).zip(self.requests.replies.drain(..returned)) {
            self.requests.answered += u64::from(reply.is_ok());
            self.requests.refused += u64::from(reply.is_err());
            self.requests.rows += reply.as_ref().map_or(0, |t| t.count() as u64);
            let reply = if let Some(ask) = project {
                project::for_file(version.file, run, &relation, ask, |_, _| reply.and_then(Answer::columns).map(|t|
                    Rc::try_unwrap(t).unwrap_or_else(|t| t.copy()))).map(super::recycle::rc).map(Answer::Columns)
            } else {
                reply
            };
            packet[id] = Some(Delivered::new(reply, true));
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
    count: bool,
    pub(super) widened: bool,
}
impl Delivered {
    fn new(reply: Reply, count: bool) -> Self { Self { reply, read: false, count, widened: false } }
}
