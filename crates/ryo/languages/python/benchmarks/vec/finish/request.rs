//! redo-fills pass 3 (THROWAWAY, bead ryo-fmx3.1): ONE request a file. When a file's second half starts, the rules alive
//! in it (the first half's survivors and skip: the emits the finisher runs) say, through the plan, which base relations
//! their steps can read. The executor sends the provider one request for exactly that set (relations, the columns read,
//! the scope) and keeps the one answer; every later fill is a slot read of it (`answered`): no call into the provider.
//! The host's relations of the same set are the host's answer at the same point. The form is the language's and the
//! scope's: nothing here names a relation, a language or a file beyond the request's scope.

use super::{Exec, File, Provider, R, Run, Scope, Supplier, Table, host, native_of};
use rustc_hash::FxHashMap;
use std::rc::Rc;

/// One relation of a request: its name and the columns its readers can read (bit `c`: the catalog's column `c`; every bit:
/// every column).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Asked<'r> {
    pub(crate) rel: &'r str,
    #[expect(dead_code, reason = "the request's column set is sent (every column: no reader's column use is derived yet) and no provider prunes by it yet")]
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
    /// Per plan node used as an emit's root: the relations its steps can read, one bit a number.
    reach: FxHashMap<usize, Rc<[u64]>>,
    /// This file's union and the asked list (kept for the next file).
    union: Vec<u64>,
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

impl Exec<'_> {
    /// The relations the steps under `root` can read (provider and host relations; never the node table's own, the run's
    /// files, or a relation an executor step makes): every base step reachable through the plan's inputs.
    fn reach(&mut self, run: &Run, root: usize) -> Rc<[u64]> {
        if let Some(r) = self.requests.reach.get(&root) {
            return r.clone();
        }
        let plan = self.plan;
        let mut seen = vec![false; plan.nodes.len()];
        let mut stack = vec![root];
        let mut rels: Vec<u16> = Vec::new();
        while let Some(n) = stack.pop() {
            if std::mem::replace(&mut seen[n], true) {
                continue;
            }
            let pn = &plan.nodes[n];
            if pn.op == "base"
                && let Some(rel) = pn.j["rel"].as_str()
                && native_of(rel).is_none()
                && matches!(run.supplier(rel), Supplier::Provider | Supplier::Host)
            {
                rels.push(self.requests.number(run, rel));
            }
            stack.extend(pn.inputs.iter().copied().filter(|i| !seen[*i]));
        }
        let words = (self.requests.names.len().max(1)).div_ceil(64) + 1;
        let mut bits = vec![0u64; words];
        for r in rels {
            bits[usize::from(r) / 64] |= 1 << (r % 64);
        }
        let bits: Rc<[u64]> = bits.into();
        self.requests.reach.insert(root, bits.clone());
        bits
    }

    /// Exactly the union of what the alive rules' steps can read (request shape (a); shape (b), less the relations whose
    /// every reader is known empty before any fill, was measured equal or unsound: NOTES.md, pass 3 cycle 1).
    fn union_of(&mut self, run: &Run, roots: &[usize], union: &mut Vec<u64>) {
        for r in roots {
            let bits = self.reach(run, *r);
            if union.len() < bits.len() {
                union.resize(bits.len(), 0);
            }
            for (u, b) in union.iter_mut().zip(bits.iter()) {
                *u |= *b;
            }
        }
    }

    /// The file's one request: the union of what the steps under `roots` (the alive rules' emits) can read, asked of the
    /// provider in one call and of the host for its own; the answers kept in the file for `answered`.
    pub(crate) fn open_file(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, roots: &[usize]) -> R<()> {
        let mut union = std::mem::take(&mut self.requests.union);
        union.clear();
        self.union_of(run, roots, &mut union);
        let rq = &mut self.requests;
        rq.counts.files += 1;
        let mut asked = std::mem::take(&mut rq.asked);
        asked.clear();
        f.answer.clear();
        f.answer.resize_with(rq.names.len(), || None);
        for (w, word) in union.iter().enumerate() {
            let mut b = *word;
            while b != 0 {
                let n = (w * 64 + b.trailing_zeros() as usize) as u16;
                b &= b - 1;
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
            let list: Vec<Asked<'_>> = asked.iter().map(|n| Asked { rel: rq.names[usize::from(*n)].as_str(), columns: u64::MAX }).collect();
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
        rq.union = union;
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
        for t in f.answer.drain(..).flatten().flatten() {
            self.requests.counts.never_asked += 1;
            self.requests.counts.never_asked_rows += t.n as u64;
        }
    }

    /// The run's request counts.
    pub(crate) fn request_line(&self) -> String {
        self.requests.line()
    }
}
