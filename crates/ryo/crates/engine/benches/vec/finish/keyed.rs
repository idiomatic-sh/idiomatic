//! THROWAWAY (ryo-fmx3.1, redo-whole pass 2): a step read by key in place of made whole.
//!
//! Every step is made whole once a file, except one that (a) has exactly one consumer in the plan and is not emitted,
//! (b) is not a view, a closure or a step made once a run, (c) is a select (kernel form, no seed test, no count; a semi one runs by the kernels, its copies of a kept row unseen by its readers), an
//! antijoin or a union, and (d) is asked by its consumer through a key: `Exec::probe` (a seed, a walk by levels), the
//! probed side of a join (`select_kernel`: the driver's keys) or of a semi (`band`). Such a step answers the rows whose
//! column holds one of the keys, from its inputs read by the same keys (a view in place through its native index, a made
//! table through its index, another such step by key in turn), with no memo: its one consumer asks it once a run of its
//! own. The answer holds exactly the rows the step made whole would hold under those keys (a select's restricted source
//! drives it, an antijoin knocks out of its left's keyed rows, a union combines its leaves' keyed rows).
//!
//! Prepared once a run a step (the form and, per output column, the input column it reads); no rule, language or corpus
//! is named: the plan's shape alone decides it.

use super::{E, Exec, File, Provider, R, Res, Run, Table};
use std::rc::Rc;

/// A step's keyed form, prepared once a run.
#[derive(Clone)]
pub(super) enum Form {
    /// Not read by key: made whole.
    Not,
    /// A select whose output column `c` is source column `cols[c]` (source, column), None when computed.
    Select(Rc<[Option<(usize, usize)>]>),
    /// An antijoin: its output columns are its left's.
    Antijoin,
    /// A union: its leaves' columns by output column (`union_kernel`).
    Union,
}

impl Exec<'_> {
    /// The keyed form of `node` (prepared once a run).
    pub(super) fn keyed_form(&mut self, run: &mut Run, node: usize) -> R<Form> {
        if let Some(form) = &self.keyed_forms[node] {
            return Ok(form.clone());
        }
        let fm = self.keyed_form_(run, node)?;
        self.keyed_forms[node] = Some(fm.clone());
        Ok(fm)
    }

    fn keyed_form_(&mut self, run: &mut Run, node: usize) -> R<Form> {
        if sw!("VEC_NO_KEYED_READ") || (self.consumers[node].len() != 1 && !sw!("VEC_KEYED_MULTI")) || self.emitted.contains(&node) || self.view(run, node)?.is_some() || self.is_constant(run, node) {
            return Ok(Form::Not);
        }
        let pn = &self.plan.nodes[node];
        Ok(match self.frames[node].op {
            super::frame::Op::Antijoin => Form::Antijoin,
            super::frame::Op::Union => Form::Union,
            super::frame::Op::Select if !pn.counted() => {
                let sp = self.sel(run, node)?;
                let closure = sp.srcs.iter().any(|s| self.plan.nodes[*s].op == "closure");
                if sp.srcs.is_empty() || sp.srcs.len() > 2 || closure || (sp.srcs.len() == 2 && sp.srcs[0] == sp.srcs[1]) || !self.ksel_covers(run, node)? {
                    Form::Not
                } else {
                    Form::Select(sp.items.iter().map(|(_, e)| if let E::Col(si, c, _) = e { Some((*si, *c)) } else { None }).collect())
                }
            }
            _ => Form::Not,
        })
    }

    /// Whether `node`, not made in this file, is read by key on its column `col`.
    #[inline]
    pub(super) fn keyed_ok(&mut self, f: &File<'_>, run: &mut Run, node: usize, col: usize) -> R<bool> {
        if f.memo[node].is_some() {
            return Ok(false);
        }
        Ok(match self.keyed_form(run, node)? {
            Form::Not => false,
            Form::Select(cols) => cols.get(col).is_some_and(Option::is_some),
            Form::Antijoin | Form::Union => true,
        })
    }

    /// The rows of `node` whose column `col` holds one of `keys` (distinct), read by key; None where `node` is made whole.
    /// `by_size`: the caller judged the keys against the step's size already (a join's driver).
    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    pub(super) fn keyed(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64], by_size: bool) -> R<Option<Rc<Table>>> {
        if !self.keyed_ok(f, run, node, col)? {
            return Ok(None);
        }
        // the keys drive only when they are fewer than the rows the step would be judged by (else it is made whole)
        if !by_size {
            let n = self.keyed_size(f, run, p, node, true)?;
            if n == 0 {
                return Ok(Some(self.empty_rc(run, node)?));
            }
            if keys.len() >= n {
                return Ok(None);
            }
        }
        if !sw!("VEC_NO_KNOWN") && self.known_empty(f, run, p, node) {
            self.stats.known[node] += 1;
            self.skipped(f, node);
            return Ok(Some(self.empty_rc(run, node)?));
        }
        let (out, handed) = match self.keyed_form(run, node)? {
            Form::Not => return Ok(None),
            Form::Antijoin => {
                let l = super::antijoin_kernel::keyed_left(self, f, run, p, node, col, keys)?;
                match l {
                    None => (self.empty_rc(run, node)?, true),
                    Some((l, out)) => {
                        let h = Rc::ptr_eq(&l, &out);
                        (out, h)
                    }
                }
            }
            Form::Union => super::union_kernel::keyed(self, f, run, p, node, col, keys)?,
            Form::Select(cols) => {
                let Some((si, c)) = cols[col] else { return Ok(None) };
                match self.select_keyed(f, run, p, node, si, c, keys)? {
                    Some(x) => x,
                    None => return Ok(None),
                }
            }
        };
        super::emptyprop::ran();
        self.stats.keyed[node] += 1;
        self.stats.total_runs += 1;
        self.stats.produced(node, out.n);
        if out.n == 0 {
            self.stats.keyed_empty[node] += 1;
        } else if !handed {
            self.stats.built[node] += out.n as u64;
        }
        Ok(Some(out))
    }
}

impl Exec<'_> {
    /// The size `node` is judged by when a join or a key read decides whether to read it by key: a made table's rows, a
    /// view's entry (its kind bucket, else its rows); a keyed step: a select the least of its sources', an antijoin its
    /// left's, a union the sum of its inputs'. A step not made (or a fill not read) is made for its size (read) only
    /// with `make` (the caller reads the step either way: whole, or by key, which makes those inputs too); else it is
    /// unknown (`usize::MAX`). 0 only when the step is empty.
    pub(super) fn keyed_size(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, make: bool) -> R<usize> {
        if let Some(Res::Tab(t)) = &f.memo[node] {
            return Ok(t.n);
        }
        if let Some(v) = self.view(run, node)? {
            // The packet decision also supplies the size of a proven empty base.
            // A keyed planner can use that zero before opening any input slot.
            if let super::Native::Fact(slot) = v.base
                && self.requests.proven_empty.get(usize::from(slot)).copied().unwrap_or(false)
            {
                return Ok(0);
            }
            // a view over a fill not read yet in this file: not read for a size
            if let super::Native::Fact(x) = v.base
                && f.facts.get(usize::from(x)).is_none_or(Option::is_none)
                && !make
            {
                return Ok(usize::MAX);
            }
            let v = match self.res(f, run, p, node)? {
                Res::Tab(t) => return Ok(t.n),
                Res::View(v) if v.base == super::Native::Empty => return Ok(0),
                Res::View(v) => v,
            };
            // its entry, never its rows read for a size: the kind bucket, the literal rows, its rows if read already,
            // else the relation's rows (an upper bound)
            if let Some(r) = f.vrows.slot(&node) {
                return Ok(r.len());
            }
            return Ok(match super::kind_set(run, &v.filter, 0, v.base, &[]) {
                Some(ks) => super::bucket_len(f, run, &ks),
                None => match super::select_kernel::lit_file(f, super::select_kernel::LIT_FILTER | node, &v.filter, 0, v.base, &[]) {
                    Some(r) => r.len(),
                    None => match v.base {
                        super::Native::Tree | super::Native::Location | super::Native::Child => f.n,
                        super::Native::Token => 3 * f.n,
                        super::Native::Empty => 0,
                        super::Native::Fact(i) => f.facts[usize::from(i)].as_ref().map_or(0, |t| t.n),
                    },
                },
            });
        }
        let form = self.keyed_form(run, node)?;
        if matches!(form, Form::Not) {
            if make {
                return Ok(self.eval(f, run, p, node)?.n);
            }
            // not made yet and not to be made for a size (its consumer may end before it reads it)
            return Ok(if !sw!("VEC_NO_KNOWN") && self.known_empty(f, run, p, node) { 0 } else { usize::MAX });
        }
        if let Some(&n) = f.keyed_sizes.get(&node) {
            return Ok(n);
        }
        if !sw!("VEC_NO_KNOWN") && self.known_empty(f, run, p, node) {
            return Ok(0);
        }
        let n = match form {
            Form::Not => unreachable!(),
            Form::Select(_) => {
                let sp = self.sel(run, node)?;
                let mut least = usize::MAX;
                for s in sp.srcs.iter() {
                    least = least.min(self.keyed_size(f, run, p, *s, make)?);
                    if least == 0 {
                        break;
                    }
                }
                least
            }
            Form::Antijoin => self.keyed_size(f, run, p, self.frames[node].first as usize, make)?,
            Form::Union => {
                let ins = self.plan.nodes[node].inputs.clone();
                let mut sum = 0usize;
                for i in ins {
                    sum = sum.saturating_add(self.keyed_size(f, run, p, i, make)?);
                }
                sum
            }
        };
        if make {
            f.keyed_sizes.insert(node, n);
        }
        Ok(n)
    }
}

/// The rows a whole run of `node` built: its answer's, unless that is an input's answer handed on (or the empty one).
pub(super) fn built(ex: &mut Exec<'_>, f: &File<'_>, node: usize, out: &Rc<Table>) {
    if out.n == 0 {
        return;
    }
    let handed = ex.plan.nodes[node].inputs.iter().any(|i| matches!(&f.memo[*i], Some(Res::Tab(t)) if Rc::ptr_eq(t, out)) || f.vmat.get(i).is_some_and(|t| Rc::ptr_eq(t, out)));
    if !handed {
        ex.stats.built[node] += out.n as u64;
    }
}
