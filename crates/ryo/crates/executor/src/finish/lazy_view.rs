//! redo-fills pass 2 (THROWAWAY, bead ryo-fmx3.1): a select source that is a fact view whose size the front never
//! measures (no kind set, no literal test in its filter or in the select's conjuncts on it) is taken unfilled; its fill
//! is asked only once the front has passed every early end (an empty source, an empty seed, an empty driver), so a
//! select that ends there asks no fill whose rows nobody reads. What the front decides is the same: a view with no
//! literal test has no size (`lit_rows` answers none whatever its fill holds).

use super::{E, Exec, File, Native, R, Run, ViewSpec, kinds_required};
use std::rc::Rc;

fn literal_on(e: &E, si: usize, width: usize) -> bool {
    (0..width).any(|c| kinds_required(e, si, c).is_some())
}

impl Exec<'_> {
    /// The view of select source `i` (step `node`), unfilled, when the front reads no size of it (see the module); None:
    /// the source is asked as before. `VEC_EAGER_VIEW` turns it off.
    pub(super) fn lazy_view(&mut self, f: &mut File<'_>, run: &mut Run, node: usize, conj: &[E], i: usize, kinds: bool) -> R<Option<Rc<ViewSpec>>> {
        if kinds || sw!("VEC_EAGER_VIEW") || matches!(f.memo[node], Some(super::Res::Tab(_))) || self.plan.nodes[node].op() == "closure" {
            return Ok(None);
        }
        let Some(v) = self.view(run, node)? else { return Ok(None) };
        let Native::Fact(x) = v.base else { return Ok(None) };
        let x = usize::from(x);
        if f.fact(x).is_some() {
            return Ok(None);
        }
        let width = v.map.len().max(v.map.iter().map(|c| c + 1).max().unwrap_or(0));
        if v.filter.iter().any(|e| literal_on(e, 0, width + 64)) || conj.iter().any(|e| literal_on(e, i, v.map.len())) {
            return Ok(None);
        }
        if f.facts.len() <= x {
            f.facts.resize(x + 1, None);
        }
        if f.fact_read.len() <= x {
            f.fact_read.resize_with(x + 1, Default::default);
        }
        if f.owed_fills.len() <= x {
            f.owed_fills.resize(x + 1, false);
        }
        f.owed_fills[x] = true;
        // the step's answer is the view, as `res` keeps it (and lists it to be cleared at the file's end)
        if f.memo[node].is_none() {
            f.remember(node, super::Res::View(v.clone()));
        }
        Ok(Some(v))
    }
}
