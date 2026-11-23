//! redo-fills pass 3 (THROWAWAY, bead ryo-fmx3.1): ONE request a file. When a file's second half starts, the rules alive
//! in it (the first half's survivors and skip: the emits the finisher runs) say, through the plan, which base relations
//! their steps can read. The executor sends the provider one request for exactly that set (relations, the columns read,
//! the scope) and keeps the one answer; every later fill is a slot read of it (`answered`): no call into the provider.
//! The host's relations of the same set are the host's answer at the same point. The form is the language's and the
//! scope's: nothing here names a relation, a language or a file beyond the request's scope.

use super::{Exec, File, Provider, R, Run, Scope, Supplier, Table, host, native_of, Native, E};
use rustc_hash::FxHashMap;
use std::rc::Rc;

/// One relation of a request: its name and the columns its readers can read (bit `c`: the catalog's column `c`; every bit:
/// every column).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Asked<'r> {
    pub(crate) rel: &'r str,
    pub(crate) columns: u64,
}

/// What the executor asks a provider once for a scope: every relation the scope's alive rules can read. The answer is one
/// table a relation, in the request's order.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Request<'r> {
    pub(crate) scope: Scope<'r>,
    pub(crate) relations: &'r [Asked<'r>],
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
    nodes: Vec<Option<Rc<Demand>>>,
    visit: Vec<bool>,
    work: Vec<usize>,
    masks: Vec<Option<u64>>,
    asked: Vec<u16>,
    answer: Vec<Result<Table, String>>,
    pub(super) counts: Counts,
}

/// Deterministic counts of the run's requests.
#[derive(Default, Debug)]
pub(crate) struct Counts {
    pub(super) files: u64,
    pub(super) provider_calls: u64,
    pub(super) provider_relations: u64,
    pub(super) host_relations: u64,
    pub(super) rows: u64,
    /// Relations the provider refused for the file (a step reading one faults, as the language says).
    pub(super) refused: u64,
    /// Relations answered that no step of the file asked (rows and size never consulted).
    pub(super) never_asked: u64,
    pub(super) never_asked_rows: u64,
}

impl Requests {
    fn number(&mut self, run: &Run, rel: &str) -> u16 {
        if let Some(n) = self.by_name.get(rel) {
            return *n;
        }
        let n = self.names.len() as u16;
        self.names.push(rel.to_owned());
        self.by_name.insert(rel.to_owned(), n);
        self.host.push(matches!(run.supplier(rel), Supplier::Host));
        n
    }

    /// The counts' line (printed at the run's end).
    pub(super) fn line(&self) -> String {
        let c = &self.counts;
        format!(
            "REQUEST files {} provider-calls {} provider-relations {} host-relations {} rows {} refused {} never-asked {} ({} rows); relations nameable {}",
            c.files, c.provider_calls, c.provider_relations, c.host_relations, c.rows, c.refused, c.never_asked, c.never_asked_rows, self.names.len()
        )
    }
}

/// A materialized operator reads its inputs whole. A select reads its view sources
/// in place, through their predicates and the columns of its expressions.
#[derive(Default)]
struct Demand {
    inputs: Vec<usize>,
    facts: Vec<(usize, u16, u64)>,
}

fn bit(c: usize) -> u64 {
    1u64.checked_shl(c as u32).unwrap_or(u64::MAX)
}

pub(crate) fn wants(mask: u64, column: usize) -> bool {
    mask & bit(column) != 0
}

fn columns(e: &E, source: usize) -> u64 {
    match e {
        E::Col(s, c, _) if *s == source => bit(*c),
        E::List(es) | E::Call(_, es) => es.iter().fold(0, |m, e| m | columns(e, source)),
        E::Cmp(_, a, b) | E::In(a, b, _) | E::And(a, b) | E::Or(a, b) => columns(a, source) | columns(b, source),
        E::Not(x) | E::Null(x, _) | E::Re(x, _) | E::Seed(x, _) => columns(x, source),
        _ => 0,
    }
}

impl Exec<'_> {
    fn demand_input(&mut self, run: &mut Run, d: &mut Demand, n: usize, mask: u64) {
        match self.view(run, n) {
            Ok(Some(v)) => {
                if let Native::Fact(i) = v.base {
                    let rel = &run.fact_rels[usize::from(i)].0;
                    let slot = self.requests.number(run, rel);
                    let mut cols = v.filter.iter().fold(0, |m, e| m | columns(e, 0));
                    for (c, base) in v.map.iter().enumerate() {
                        if mask & bit(c) != 0 {
                            cols |= bit(*base);
                        }
                    }
                    d.facts.push((n, slot, cols));
                }
            }
            _ => d.inputs.push(n),
        }
    }

    fn demand(&mut self, run: &mut Run, n: usize) -> Rc<Demand> {
        if let Some(d) = &self.requests.nodes[n] {
            return d.clone();
        }
        let pn = &self.plan.nodes[n];
        let mut d = Demand::default();
        if self.view(run, n).ok().flatten().is_some() {
            self.demand_input(run, &mut d, n, u64::MAX);
        } else if pn.op == "base" {
            if let Some(rel) = pn.j["rel"].as_str()
                && native_of(rel).is_none()
                && matches!(run.supplier(rel), Supplier::Provider | Supplier::Host)
            {
                d.facts.push((n, self.requests.number(run, rel), u64::MAX));
            }
        } else if pn.op == "select" && let Ok(sp) = self.sel(run, n) {
            for (i, src) in sp.srcs.iter().enumerate() {
                let probed = sp.conj.iter().any(|e| matches!(e, E::Seed(x, _) if matches!(**x, E::Col(s, _, _) if s == i)));
                let mask = if probed { u64::MAX } else {
                    sp.conj.iter().chain(sp.items.iter().map(|(_, e)| e)).fold(0, |m, e| m | columns(e, i))
                };
                self.demand_input(run, &mut d, *src, mask);
            }
        } else {
            for &src in &pn.inputs {
                self.demand_input(run, &mut d, src, u64::MAX);
            }
            // A counted select can emit a kept row with zero partners. Its
            // group reads both sources directly even when the select is empty.
            if self.frames[n].counted {
                let sel = self.frames[n].first as usize;
                for &src in &self.plan.nodes[sel].inputs {
                    self.demand_input(run, &mut d, src, u64::MAX);
                }
            }
        }
        let d = Rc::new(d);
        self.requests.nodes[n] = Some(d.clone());
        d
    }

    fn request_masks(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, roots: &[usize]) -> Vec<Option<u64>> {
        self.requests.nodes.resize_with(self.plan.nodes.len(), || None);
        let mut masks = std::mem::take(&mut self.requests.masks);
        masks.fill(None);
        let mut work = std::mem::take(&mut self.requests.work);
        let mut seen = std::mem::take(&mut self.requests.visit);
        seen.resize(self.plan.nodes.len(), false);
        seen.fill(false);
        work.extend_from_slice(roots);
        f.before_answer = true;
        while let Some(n) = work.pop() {
            if std::mem::replace(&mut seen[n], true) || (!sw!("VEC_NO_KNOWN") && self.static_empty(f, run, p, n)) {
                continue;
            }
            let d = self.demand(run, n);
            work.extend_from_slice(&d.inputs);
            for &(src, slot, cols) in &d.facts {
                if src != n && !sw!("VEC_NO_KNOWN") && self.static_empty(f, run, p, src) {
                    continue;
                }
                masks.resize(self.requests.names.len(), None);
                *masks[usize::from(slot)].get_or_insert(0) |= cols;
            }
        }
        f.before_answer = false;
        self.requests.work = work;
        self.requests.visit = seen;
        masks
    }

    /// The file's one request: the union of what the steps under `roots` (the alive rules' emits) can read, asked of the
    /// provider in one call and of the host for its own; the answers kept in the file for `answered`.
    pub(crate) fn open_file(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, roots: &[usize]) -> R<()> {
        let masks = self.request_masks(f, run, p, roots);
        let rq = &mut self.requests;
        rq.counts.files += 1;
        let mut asked = std::mem::take(&mut rq.asked);
        asked.clear();
        f.answer.clear();
        f.answer.resize_with(rq.names.len(), || None);
        for (n, mask) in masks.iter().enumerate() {
            if mask.is_some() {
                let n = n as u16;
                if rq.host[usize::from(n)] {
                    let rel = &rq.names[usize::from(n)];
                    let t = host::host_fill(run, rel, f.file).unwrap_or_else(|| Err(format!("the host supplies no {rel}")))?;
                    rq.counts.host_relations += 1;
                    f.answer[usize::from(n)] = Some(Ok(t));
                } else {
                    asked.push(n);
                }
            }
        }
        if !asked.is_empty() {
            let list: Vec<Asked<'_>> = asked.iter().map(|n| Asked { rel: rq.names[usize::from(*n)].as_str(), columns: masks[usize::from(*n)].unwrap() }).collect();
            let mut answer = std::mem::take(&mut rq.answer);
            answer.clear();
            p.answer(run, &Request { scope: Scope::File(f.file), relations: &list }, &mut answer)?;
            if answer.len() != list.len() {
                return Err(format!("the provider answered {} relations of {}", answer.len(), list.len()));
            }
            rq.counts.provider_calls += 1;
            rq.counts.provider_relations += list.len() as u64;
            for (n, t) in asked.iter().zip(answer.drain(..)) {
                match &t {
                    Ok(t) => rq.counts.rows += t.n as u64,
                    Err(_) => rq.counts.refused += 1,
                }
                f.answer[usize::from(*n)] = Some(t);
            }
            rq.answer = answer;
        }
        rq.asked = asked;
        rq.masks = masks;
        Ok(())
    }

    /// A relation's table from the file's answer (moved out: the caller keeps it as the file's fill), or an error naming a
    /// relation the request did not hold.
    pub(super) fn answered(&mut self, f: &mut File<'_>, rel: &str) -> R<Table> {
        let n = self.requests.by_name.get(rel).copied().ok_or_else(|| format!("{rel}: in no request of this run"))?;
        f.answer.get_mut(usize::from(n)).and_then(Option::take).ok_or_else(|| format!("{rel}: not in this file's request"))?
    }

    /// The file's end: the answered relations no step asked, counted.
    pub(crate) fn close_file(&mut self, f: &mut File<'_>) {
        for (i, answer) in f.answer.drain(..).enumerate() {
            if self.requests.host[i] { continue; }
            let Some(Ok(t)) = answer else { continue };
            self.requests.counts.never_asked += 1;
            self.requests.counts.never_asked_rows += t.n as u64;
        }
    }

    /// The run's request counts.
    pub(crate) fn request_line(&self) -> String {
        self.requests.line()
    }
}
