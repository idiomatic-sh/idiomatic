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
    /// The input whose empty answer empties the step, tested before its body (`Exec::make_`): `first`, but `NONE` for a
    /// union and a counted group count.
    pub(super) gate: u32,
}

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
                "union" | "antijoin" | "intersection" | "difference" | "ordered-match" | "as-of" => rel("left"),
                "closure" | "follow" => rel("edges"),
                // A worklist validates even the unused lattice and edges of an empty root set.
                "base" | "select" | "rows" | "worklist" => NONE,
                _ => rel("from"),
            };
            let counted = op == Op::GroupCount && !pn.distinct_count() && !no_semi && first != MISSING && plan.nodes[first as usize].counted();
            // a union is empty only when every input is; a counted group count reads its select's sources, not the input
            let gate = if op == Op::Union || counted { NONE } else { first };
            Frame { op, first, counted, gate }
        })
        .collect()
}

