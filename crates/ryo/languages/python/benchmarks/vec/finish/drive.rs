//! THROWAWAY (ryo-fmx3.1, step 6e item 1): the bound side drives. Demand-driven evaluation with tabling over batches of
//! keys: a step not made yet in a file is never made whole because a join reads it; the join probes it with the keys its
//! bound side gives (the per-key memo is the table). A step is made whole only where no key reaches it, and then the
//! reason is recorded (`Stats::why`). "Bound" propagates statically from the survivor seeds through row-wise positions;
//! made tables and kind buckets are bound by their exact counts.

use super::{Exec, File, Provider, R, Run, Table, Ty};
use std::rc::Rc;

impl Exec<'_> {
    /// Whether a step not made yet is left pending, to be read by the keys of whatever reads it.
    pub(super) fn deferred(&mut self, f: &File<'_>, run: &mut Run, node: usize) -> R<bool> {
        if f.memo[node].is_some() || self.view(run, node)?.is_some() || sw!("VEC_NO_KEYED") {
            return Ok(false);
        }
        if self.static_keyed {
            return Ok(self.keyed(run, node));
        }
        let op = self.plan.nodes[node].op.as_str();
        Ok(!op.starts_with("closure") && !self.emitted.contains(&node) && !self.is_constant(node))
    }

    /// Whether a step's rows are restricted by a survivor seed: a select with a seed test or a bound source, and every
    /// step whose rows come row-wise from a bound input.
    pub(super) fn bound(&mut self, run: &mut Run, node: usize) -> bool {
        if let Some(b) = self.bound_memo[node] {
            return b;
        }
        self.bound_memo[node] = Some(false);
        let pn = &self.plan.nodes[node];
        let op = pn.op.clone();
        let arg = |me: &Self, name: &str| me.arg_rel(node, name).ok();
        let b = match op.as_str() {
            "select" => match self.sel(run, node) {
                Ok(sp) => sp.conj.iter().any(|e| matches!(e, super::E::Seed(..))) || sp.srcs.clone().iter().any(|x| self.bound(run, *x)),
                Err(_) => false,
            },
            "union" => match (arg(self, "left"), arg(self, "right")) {
                (Some(l), Some(r)) => self.bound(run, l) && self.bound(run, r),
                _ => false,
            },
            "antijoin" | "ordered-match" | "as-of" => arg(self, "left").is_some_and(|l| self.bound(run, l)),
            "base" | "rows" | "closure" | "follow" => false,
            _ => arg(self, "from").is_some_and(|x| self.bound(run, x)),
        };
        self.bound_memo[node] = Some(b);
        b
    }

    /// The right side of a keyed step (antijoin, as-of, ordered-match) read by the left's keys on its first key column
    /// that is not the file, where the right side is still pending; else made.
    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    pub(super) fn right_by_keys(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, rn: usize, l: &Table, lk: &[usize], rk: &[usize]) -> R<Rc<Table>> {
        // the right side is never the probed left put in place, even where both read the same step
        let saved = self.over.take();
        let out = self.right_by_keys_(f, run, p, rn, l, lk, rk);
        self.over = saved;
        out
    }

    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    fn right_by_keys_(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, rn: usize, l: &Table, lk: &[usize], rk: &[usize]) -> R<Rc<Table>> {
        if self.deferred(f, run, rn)? {
            let sc = self.schema(run, rn)?;
            let at = (0..rk.len()).find(|i| sc.tys[rk[*i]] != Ty::File).or(if self.static_keyed { Some(0) } else { None });
            if let Some(i) = at {
                let ks: Vec<u64> = l.cols[lk[i]].iter().copied().collect::<rustc_hash::FxHashSet<u64>>().into_iter().collect();
                return self.probe(f, run, p, rn, rk[i], &ks);
            }
            self.reason = "keyed right side with only file keys";
        }
        self.eval(f, run, p, rn)
    }

    /// A select's positional override: taken out (the select's other positions, and everything it asks, read the real
    /// step), with the position it holds.
    pub(super) fn take_positional(&mut self, srcs: &[usize]) -> Option<(usize, Rc<Table>)> {
        match &self.over {
            Some((step, input, Some(at), _)) if self.stack.last() == Some(step) && srcs.get(*at) == Some(input) => {
                let (_, _, at, t) = self.over.take()?;
                Some((at?, t))
            }
            _ => None,
        }
    }

    /// The rows put in place of `node` for the step now running, if any.
    pub(super) fn overridden(&self, node: usize) -> Option<Rc<Table>> {
        match &self.over {
            Some((step, input, _, t)) if *input == node && self.stack.last() == Some(step) => Some(t.clone()),
            _ => None,
        }
    }

    /// One run of `node` over `sub` in place of its input `input` (the step's own run only: nothing in the file's memo
    /// changes, so what other inputs make is exact and stays).
    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    pub(super) fn run_over(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, input: usize, at: Option<usize>, sub: Rc<Table>) -> R<Rc<Table>> {
        let saved = self.over.replace((node, input, at, sub));
        let out = self.body(f, run, p, node);
        self.over = saved;
        out
    }

    /// A grouping step (group-min, group-max, group-ordinal, ordered-match) probed on a column that is not one of its
    /// keys: the keys reach it through its groups. The probed rows of the input give their group keys; the input is
    /// probed again by those (whole groups), the step runs over them alone, and its rows are kept where the column holds
    /// one of the keys. Exact: each group's answer depends only on that group's rows.
    pub(super) fn probe_by_group(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64]) -> R<Option<Table>> {
        let plan = self.plan;
        let pn = &plan.nodes[node];
        let a = &pn.j["args"];
        let (arg, gkeys) = match pn.op.as_str() {
            "group-min" | "group-max" | "group-ordinal" => ("from", super::strs(&a["keys"])?),
            "ordered-match" => ("left", super::strs(&a["left-on"])?),
            _ => return Ok(None),
        };
        let sc = self.schema(run, node)?;
        let name = sc.names[col].clone();
        // the probed column in the input
        let in_name = if pn.op == "ordered-match" {
            match (a["output"][name.as_str()][0].as_str(), a["output"][name.as_str()][1].as_str()) {
                (Some("left"), Some(c)) => c.to_owned(),
                _ => return Ok(None),
            }
        } else {
            name.clone()
        };
        let input = self.arg_rel(node, arg)?;
        if pn.op != "ordered-match" && pn.inputs.iter().filter(|i| **i == input).count() != 1 {
            return Ok(None);
        }
        let isc = self.schema(run, input)?;
        let Some(ic) = isc.names.iter().position(|c| *c == in_name) else { return Ok(None) };
        let Some(gc) = gkeys.iter().filter_map(|k| isc.names.iter().position(|c| c == k)).find(|c| isc.tys[*c] != Ty::File) else { return Ok(None) };
        let first = self.probe(f, run, p, input, ic, keys)?;
        let gk: Vec<u64> = first.cols[gc].iter().copied().collect::<rustc_hash::FxHashSet<u64>>().into_iter().collect();
        let groups = self.probe(f, run, p, input, gc, &gk)?;
        let out = self.run_over(f, run, p, node, input, None, groups)?;
        let want: rustc_hash::FxHashSet<u64> = keys.iter().copied().collect();
        let mut kept = Table::with(out.names.clone(), out.tys.clone());
        let mut row = Vec::new();
        for r in 0..out.n {
            if want.contains(&out.cols[col][r]) {
                out.row(r, &mut row);
                kept.push_row(&row);
            }
        }
        Ok(Some(kept))
    }
}

/// The rows of `t` whose column `col` holds one of `keys` (an index made for this ask only).
pub(super) fn subset_of(t: &Table, col: usize, keys: &[u64]) -> Table {
    let ix = super::index_of(t, &[col]);
    let mut out = Table::with(t.names.clone(), t.tys.clone());
    let mut row = Vec::new();
    for k in keys {
        if let Some(rows) = ix.get(k) {
            for r in rows {
                t.row(*r as usize, &mut row);
                out.push_row(&row);
            }
        }
    }
    out
}
