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
pub(crate) enum Form {
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
    pub(crate) fn keyed_form(&mut self, run: &mut Run, node: usize) -> R<Form> {
        if let Some(form) = &self.keyed_forms[node] {
            return Ok(form.clone());
        }
        let fm = self.keyed_form_(run, node)?;
        self.keyed_forms[node] = Some(fm.clone());
        Ok(fm)
    }

    pub(crate) fn keyed_form_(&mut self, run: &mut Run, node: usize) -> R<Form> {
        if sw!("VEC_NO_KEYED_READ") || (self.plan.consumers(node).len() != 1 && true) || self.emitted.contains(&node) || self.view(run, node)?.is_some() || self.is_constant(run, node) {
            return Ok(Form::Not);
        }
        let pn = &self.plan.nodes[node];
        Ok(match self.plan.frame(node).op {
            super::frame::Op::Antijoin => Form::Antijoin,
            super::frame::Op::Union => Form::Union,
            super::frame::Op::Select if !pn.counted() => {
                let sp = self.sel(run, node)?;
                let closure = sp.srcs.iter().any(|s| self.plan.nodes[*s].op() == "closure");
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
    pub(crate) fn keyed_ok(&mut self, f: &File<'_>, run: &mut Run, node: usize, col: usize) -> R<bool> {
        if let Some(input)=f.population(node) {return Ok(input.can_probe(col));}
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
    pub(crate) fn keyed(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64], by_size: bool) -> R<Option<Rc<Table>>> {
        if let Some(input) = f.population(node) {
            if !input.can_probe(col) { return Ok(None); }
            let sc=self.schema(run,node)?;
            return Ok(Some(input.probe(&sc.names,&sc.tys,col,keys)));
        }
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
            self.skipped(f, node);
            return Ok(Some(self.empty_rc(run, node)?));
        }
        let out = match self.keyed_form(run, node)? {
            Form::Not => return Ok(None),
            Form::Antijoin => {
                let l = super::antijoin_kernel::keyed_left(self, f, run, p, node, col, keys)?;
                match l {
                    None => self.empty_rc(run, node)?,
                    Some((_, out)) => out,
                }
            }
            Form::Union => super::union_kernel::keyed(self, f, run, p, node, col, keys)?.0,
            Form::Select(cols) => {
                let Some((si, c)) = cols[col] else { return Ok(None) };
                match self.select_keyed(f, run, p, node, si, c, keys)? {
                    Some((out, _)) => out,
                    None => return Ok(None),
                }
            }
        };
        super::emptyprop::ran();
        self.stats.total_runs += 1;
        self.made[node] = true;        self.stats.total_rows += out.n as u64;
        Ok(Some(out))
    }
}

impl Exec<'_> {
    /// The size `node` is judged by when a join or a key read decides whether to read it by key: a made table's rows, a
    /// view's entry (its kind bucket, else its rows); a keyed step: a select the least of its sources', an antijoin its
    /// left's, a union the sum of its inputs'. A step not made (or a fill not read) is made for its size (read) only
    /// with `make` (the caller reads the step either way: whole, or by key, which makes those inputs too); else it is
    /// unknown (`usize::MAX`). 0 only when the step is empty.
    pub(crate) fn keyed_size(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, make: bool) -> R<usize> {
        if let Some(input)=f.population(node) { return Ok(input.n); }
        if let Some(Res::Tab(t)) = &f.memo[node] {
            return Ok(t.n);
        }
        if let Some(v) = self.view(run, node)? {
            // a view over a fill not read yet in this file: not read for a size
            if let super::Native::Fact(x) = v.base
                && f.fact(usize::from(x)).is_none()
                && !make
            {
                return Ok(usize::MAX);
            }
            if let Res::Tab(t) = self.res(f, run, p, node)? {
                return Ok(t.n);
            }
            // its entry, never its rows read for a size: the kind bucket, the literal rows, its rows if read already,
            // else the relation's rows (an upper bound)
            if let Some(r) = f.vrows.slot(&node) {
                return Ok(r.len());
            }
            return Ok(match super::kind_set(run, &v.filter, 0, v.base, &[]) {
                Some(ks) => super::bucket_len(f, run, &ks, v.base.extent()),
                None => match super::select_kernel::lit_file(f, super::select_kernel::LIT_FILTER | node, &v.filter, 0, v.base, &[]) {
                    Some(r) => r.len(),
                    None => match v.base {
                        super::Native::Tree(e) | super::Native::Child(e) => f.limit(e),
                        super::Native::Location => f.n,
                        super::Native::Fact(i) => f.fact(usize::from(i)).map_or(0, |t| t.len()),
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
            Form::Antijoin => self.keyed_size(f, run, p, self.plan.frame(node).first as usize, make)?,
            Form::Union => {
                let ins = &self.plan.nodes[node].inputs;
                let mut sum = 0usize;
                for &i in ins {
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
