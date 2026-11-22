//! THROWAWAY (ryo-fmx3.1, redo-select2): a probe known to answer no row before it is made.
//!
//! A probe of a step on a column its input carries unchanged (`frame::ptarget`: the substitution's target) answers rows
//! only where that input has rows with those keys. Followed down to the first input already made (a table in the file's
//! memo) or a view read in place, the keys are looked up there: if none is present, every probe on the way answers
//! nothing, and none of them would make anything (a substitution over an input that answered nothing returns before its
//! body). So the probe is not made at all. Anything else on the way (a step put in place, a semi step, a general or
//! whole target, a view whose fill is not read yet) leaves the question open and the probe is made as before.

use super::{Exec, File, R, Res, Run, frame, view_holds};

fn stop(why: &'static str, op: &str) {
    if super::empty_sel::on() {
        super::empty_sel::count(if op.is_empty() { why } else { Box::leak(format!("{why} at {op}").into_boxed_str()) }, 0.0);
    }
}

/// Steps followed at most a chain, and chains at most a question.
const DEPTH: usize = 12;
const LEAVES: usize = 16;

impl Exec<'_> {
    /// Whether a probe of `node` on `col` by `keys` is known to answer no row (false: not known).
    pub(super) fn keys_absent(&mut self, f: &mut File<'_>, run: &mut Run, node: usize, col: usize, keys: &[u64]) -> R<bool> {
        if !self.static_keyed || sw!("VEC_PTARGET_OFF") || sw!("VEC_OLD_EMPTYSEL") || sw!("VEC_ESEL_NO_ABSENT") {
            return Ok(false);
        }
        let mut budget = LEAVES;
        self.absent(f, run, node, col, keys, &mut budget)
    }

    /// The chain from (`node`, `col`); a union's inputs each (all absent), an antijoin's left where its probe runs over
    /// the probed left (`antijoin_kernel`'s `direct`), within `budget` chains.
    fn absent(&mut self, f: &mut File<'_>, run: &mut Run, node: usize, col: usize, keys: &[u64], budget: &mut usize) -> R<bool> {
        if *budget == 0 {
            return Ok(false);
        }
        *budget -= 1;
        let (mut n, mut c) = (node, col);
        for _ in 0..DEPTH {
            if self.overridden(n).is_some() {
                stop("overridden", "");
                return Ok(false);
            }
            match &f.memo[n] {
                Some(Res::Tab(t)) => {
                    let t = t.clone();
                    if t.n == 0 {
                        return Ok(true);
                    }
                    let col = &t.cols[c];
                    let a = if t.n <= 32 {
                        !keys.iter().any(|k| col.contains(k))
                    } else {
                        let ix = super::table_index(f, n, &t, &[c]);
                        !keys.iter().any(|k| ix.get(k).is_some())
                    };
                    stop(if a { "table: absent" } else { "table: present" }, "");
                    return Ok(a);
                }
                Some(Res::View(v)) => {
                    let v = v.clone();
                    let a = self.view_absent(f, run, &v, c, keys);
                    stop(if a { "view: absent" } else { "view: present or unread" }, "");
                    return Ok(a);
                }
                None => {}
            }
            if let Some(v) = self.view(run, n)? {
                let a = self.view_absent(f, run, &v, c, keys);
                stop(if a { "view (not read yet): absent" } else { "view (not read yet): present or unread" }, "");
                return Ok(a);
            }
            if self.known_empty(f, run, n) {
                return Ok(true);
            }
            // a semi step may be made whole for a probe; a counted group count runs over an input that answered nothing
            if self.frames[n].semi || self.frames[n].counted {
                stop("semi or counted", "");
                return Ok(false);
            }
            let (input, icol) = self.ptarget(run, n, c)?;
            if input == frame::GENERAL && !sw!("VEC_ESEL_ABSENT_CHAIN") {
                let op = self.frames[n].op;
                if op == frame::Op::Union && !sw!("VEC_OLD_UNION") {
                    // the union's rows for the keys are its inputs' (each by the column of that name)
                    // redo-alloc: the schema's names and the plan's inputs read in place (no copy a call)
                    let names = self.schema(run, n)?.names.clone();
                    let name = &names[c];
                    let plan = self.plan;
                    for &i in &plan.nodes[n].inputs {
                        let Some(ic) = self.schema(run, i)?.names.iter().position(|x| x == name) else { return Ok(false) };
                        if !self.absent(f, run, i, ic, keys, budget)? {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                }
                if op == frame::Op::Antijoin && !sw!("VEC_OLD_ANTIJOIN") {
                    let left = self.arg_rel(n, "left")?;
                    let ins = &self.plan.nodes[n].inputs;
                    let direct = ins.iter().filter(|i| **i == left).count() == 1 && ins.clone().iter().filter(|i| **i != left).all(|o| !self.depends(*o, left));
                    if direct {
                        let name = self.schema(run, n)?.names[c].clone();
                        let Some(lc) = self.schema(run, left)?.names.iter().position(|x| *x == name) else { return Ok(false) };
                        (n, c) = (left, lc);
                        continue;
                    }
                }
            }
            if input >= frame::GENERAL {
                stop(if input == frame::GENERAL { "general" } else { "whole" }, &self.plan.nodes[n].op);
                return Ok(false);
            }
            (n, c) = (input as usize, icol as usize);
        }
        Ok(false)
    }

    /// A view's rows on `c` for none of `keys`, read in place (false where it cannot be read in place or its fill is
    /// not read yet).
    fn view_absent(&mut self, f: &File<'_>, run: &mut Run, v: &super::ViewSpec, c: usize, keys: &[u64]) -> bool {
        let bc = v.map[c];
        if !File::probes(v.base, bc) {
            return false;
        }
        if let super::Native::Fact(i) = v.base
            && f.facts.get(usize::from(i)).is_none_or(Option::is_none)
        {
            return false;
        }
        let mut hits = Vec::new();
        for k in keys {
            hits.clear();
            f.native_probe(run, v.base, bc, *k, &mut hits);
            if hits.iter().any(|h| view_holds(f, run, v, *h)) {
                return false;
            }
        }
        true
    }
}
