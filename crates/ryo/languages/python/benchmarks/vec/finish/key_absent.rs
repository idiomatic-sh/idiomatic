//! THROWAWAY (ryo-fmx3.1, redo-select2): a probe known to answer no row before it is made.
//!
//! A probe of a step on a column its input carries unchanged (`frame::ptarget`: the substitution's target) answers rows
//! only where that input has rows with those keys. Followed down to the first input already made (a table in the file's
//! memo) or a view read in place, the keys are looked up there: if none is present, every probe on the way answers
//! nothing, and none of them would make anything (a substitution over an input that answered nothing returns before its
//! body). So the probe is not made at all. Anything else on the way (a step put in place, a semi step, a general or
//! whole target, a view whose fill is not read yet) leaves the question open and the probe is made as before.

use super::{Exec, File, R, Res, Run, frame, view_holds};

/// Steps followed at most.
const DEPTH: usize = 12;

impl Exec<'_> {
    /// Whether a probe of `node` on `col` by `keys` is known to answer no row (false: not known).
    pub(super) fn keys_absent(&mut self, f: &mut File<'_>, run: &mut Run, node: usize, col: usize, keys: &[u64]) -> R<bool> {
        if !self.static_keyed || sw!("VEC_PTARGET_OFF") || sw!("VEC_OLD_EMPTYSEL") || sw!("VEC_ESEL_NO_ABSENT") {
            return Ok(false);
        }
        let (mut n, mut c) = (node, col);
        for _ in 0..DEPTH {
            if self.overridden(n).is_some() {
                return Ok(false);
            }
            match &f.memo[n] {
                Some(Res::Tab(t)) => {
                    let t = t.clone();
                    if t.n == 0 {
                        return Ok(true);
                    }
                    let col = &t.cols[c];
                    if t.n <= 32 {
                        return Ok(!keys.iter().any(|k| col.contains(k)));
                    }
                    let ix = super::table_index(f, n, &t, &[c]);
                    return Ok(!keys.iter().any(|k| ix.get(k).is_some()));
                }
                Some(Res::View(v)) => {
                    let v = v.clone();
                    return Ok(self.view_absent(f, run, &v, c, keys));
                }
                None => {}
            }
            if let Some(v) = self.view(run, n)? {
                return Ok(self.view_absent(f, run, &v, c, keys));
            }
            if self.known_empty(f, run, n) {
                return Ok(true);
            }
            // a semi step may be made whole for a probe; a counted group count runs over an input that answered nothing
            if self.frames[n].semi || self.frames[n].counted {
                return Ok(false);
            }
            let (input, icol) = self.ptarget(run, n, c)?;
            if input >= frame::GENERAL {
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
