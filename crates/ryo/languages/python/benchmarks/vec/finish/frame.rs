//! THROWAWAY (ryo-fmx3.1, redo-union pass 2): what the step bracket reads on every run, prepared once at plan load in
//! flat arrays by node id: the operator as a code, the input whose emptiness ends the step, whether a group count reads
//! its select's counted rows. Nothing is read from the plan's JSON, no name compared, no map looked up per run.

use super::{NONE, Plan};

/// A step's operator, as a code.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Op {
    Select,
    Union,
    Antijoin,
    Extend,
    Base,
    Rows,
    Closure,
    Follow,
    GroupCount,
    /// Every other operator (its body matches the name).
    Other,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Frame {
    pub(super) op: Op,
    /// The input whose emptiness ends the step before anything else is asked (`NONE`: none; `MISSING`: the plan names
    /// none where the operator needs one, an error at the run).
    pub(super) first: u32,
    /// A group count over a counted select (`band_rows` with counts) is tried before the first input.
    pub(super) counted: bool,
    /// A select marked `semi` without `count` (a probe may make it whole: `semi_drives_whole`).
    pub(super) semi: bool,
}

/// A probe's target by (step, column), resolved once a run (static keyed read): the input put in place and its column,
/// or the step made whole (and why), or the general path.
pub(super) const WHOLE_CARRIED: u32 = u32::MAX;
pub(super) const WHOLE_UNCARRIED: u32 = u32::MAX - 1;
pub(super) const GENERAL: u32 = u32::MAX - 2;

pub(super) const MISSING: u32 = u32::MAX - 1;

pub(super) fn frames(plan: &Plan) -> Vec<Frame> {
    let no_semi = sw!("VEC_NO_SEMI");
    plan.nodes
        .iter()
        .map(|pn| {
            let op = match pn.op.as_str() {
                "select" => Op::Select,
                "union" => Op::Union,
                "antijoin" => Op::Antijoin,
                "extend" => Op::Extend,
                "base" => Op::Base,
                "rows" => Op::Rows,
                "closure" => Op::Closure,
                "follow" => Op::Follow,
                "group-count" => Op::GroupCount,
                _ => Op::Other,
            };
            let rel = |name: &str| pn.rels.iter().find(|(n, _)| n == name).map_or(MISSING, |x| x.1 as u32);
            let first = match pn.op.as_str() {
                "union" | "antijoin" | "ordered-match" | "as-of" => rel("left"),
                "closure" | "follow" => rel("edges"),
                "base" | "select" | "rows" => NONE,
                _ => rel("from"),
            };
            let counted = op == Op::GroupCount && pn.j["args"]["distinct"].is_null() && !no_semi && first != MISSING && plan.nodes[first as usize].j.get("count").is_some();
            let semi = op == Op::Select && pn.j.get("semi").and_then(serde_json::Value::as_u64).is_some() && pn.j.get("count").is_none() && !no_semi;
            Frame { op, first, counted, semi }
        })
        .collect()
}

impl super::Exec<'_> {
    /// The probe target of (node, column): (input, its column), or `WHOLE_*` / `GENERAL` in the first place.
    #[inline]
    pub(super) fn ptarget(&mut self, run: &mut super::Run, node: usize, col: usize) -> super::R<(u32, u32)> {
        if let Some(t) = &self.ptargets[node] {
            return Ok(t[col]);
        }
        let sc = self.schema(run, node)?;
        let mut v = Vec::with_capacity(sc.names.len());
        for c in 0..sc.names.len() {
            // a column whose target cannot be resolved takes the general path (which reports the same error)
            v.push(self.ptarget_of(run, node, c).unwrap_or((GENERAL, 0)));
        }
        let b: Box<[(u32, u32)]> = v.into();
        let t = b[col];
        self.ptargets[node] = Some(b);
        Ok(t)
    }

    fn ptarget_of(&mut self, run: &mut super::Run, node: usize, col: usize) -> super::R<(u32, u32)> {
        let plan = self.plan;
        let pn = &plan.nodes[node];
        let sc = self.schema(run, node)?;
        let name = sc.names[col].clone();
        let unary = |me: &mut Self, run: &mut super::Run, arg: &str| -> super::R<Option<(usize, usize)>> {
            let n = me.arg_rel(node, arg)?;
            let isc = me.schema(run, n)?;
            Ok(isc.names.iter().position(|c| *c == name).map(|c| (n, c)))
        };
        let target = match pn.op.as_str() {
            "select" => {
                let sp = self.sel(run, node)?;
                match &sp.items[col].1 {
                    super::E::Col(si, c, _) if sp.srcs.iter().filter(|x| **x == sp.srcs[*si]).count() == 1 => Some((sp.srcs[*si], *c)),
                    _ => None,
                }
            }
            "extend" | "project" | "span-between" | "span-bounds" | "source-matches" | "replace" | "capture" => unary(self, run, "from")?,
            "group-count" | "group-min" | "group-max" => {
                let ks = super::strs(&pn.j["args"]["keys"])?;
                if ks.contains(&name) { unary(self, run, "from")? } else { None }
            }
            "union" | "antijoin" | "axis" | "closure" => return Ok((GENERAL, 0)),
            _ => None,
        };
        let carried = target.is_some();
        let target = target.filter(|(input, _)| {
            let ins = &plan.nodes[node].inputs;
            ins.iter().filter(|i| **i == *input).count() == 1 && ins.iter().filter(|i| **i != *input).all(|o| !self.depends(*o, *input))
        });
        Ok(match target {
            Some((i, c)) => (i as u32, c as u32),
            None if carried => (WHOLE_CARRIED, 0),
            None => (WHOLE_UNCARRIED, 0),
        })
    }
}
