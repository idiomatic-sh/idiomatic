//! Locate the outputs of file-local steps consumed after admission closes.
//! Every node number addresses the same prepared executor plan in both phases.
use super::{E, Exec, Op, R, Run, Ty};
use std::collections::BTreeSet;

#[derive(Clone, Default)]
pub(in super::super) struct Partition {
    pub inputs: Vec<usize>,
    pub local_emit: bool,
    pub file_steps: Vec<bool>,
}

/// A multidirectional join stays local only if its equality edges connect every
/// source with file-local keys. Pure project relations and caller snapshots are
/// always complete at the later phase, even if they happen to hold one row.
pub(in super::super) fn partition(
    exec: &mut Exec<'_>,
    run: &mut Run,
    wide: impl Fn(&str) -> bool,
) -> R<Partition> {
    // All operands here are already local. A span names a place in that source.
    fn anchor(ty: Ty) -> bool {
        matches!(ty, Ty::Node | Ty::File | Ty::Span)
    }
    let plan = exec.plan;
    let mut local = Vec::with_capacity(plan.nodes.len());
    for (i, step) in plan.nodes.iter().enumerate() {
        let args = step.args();
        let keys_hold = |ex: &mut Exec<'_>, run: &mut Run, from: &str, keys: &str| -> R<bool> {
            let source = ex.arg_rel(i, from)?;
            let s = ex.schema(run, source)?;
            for name in super::strs(&args[keys])? {
                if anchor(s.tys[super::pos(&s.names, name)?]) {
                    return Ok(true);
                }
            }
            Ok(false)
        };
        let here = if !step.inputs.iter().all(|&source| local[source]) {
            false
        } else {
            match step.op.as_str() {
                "base" => !wide(step.base().ok_or("base relation")?),
                "rows" => false,
                "select" => {
                    if step.inputs.len() < 2 {
                        true
                    } else {
                        let names = step
                            .inputs
                            .iter()
                            .enumerate()
                            .map(|(i, &n)| {
                                Ok((step.alias(i).map(str::to_owned), exec.schema(run, n)?))
                            })
                            .collect::<R<Vec<_>>>()?;
                        let mut components: Vec<_> = (0..names.len()).collect();
                        let mut terms = Vec::new();
                        if let Some(cond) = exec.select_condition(run, step, &names)? {
                            super::flatten(cond, &mut terms);
                        }
                        for term in terms {
                            let E::Cmp(Op::Eq, a, b) = term else { continue };
                            let (E::Col(left, _, lt), E::Col(right, _, rt)) = (*a, *b) else {
                                continue;
                            };
                            if lt != rt || !anchor(lt) {
                                continue;
                            }
                            let (a, b) = (components[left], components[right]);
                            for c in &mut components {
                                if *c == b {
                                    *c = a;
                                }
                            }
                        }
                        components.iter().all(|&c| c == components[0])
                    }
                }
                "project" => {
                    let src = exec.arg_rel(i, "from")?;
                    exec.schema(run, i)?.tys.iter().any(|&t| anchor(t))
                        || !exec.schema(run, src)?.tys.iter().any(|&t| anchor(t))
                }
                "union" | "intersection" | "difference" => {
                    let mut held = true;
                    for &source in &step.inputs {
                        held &= exec.schema(run, source)?.tys.iter().any(|&t| anchor(t));
                    }
                    held
                }
                "antijoin" | "as-of" | "ordered-match" => {
                    keys_hold(exec, run, "left", "left-on")?
                        && keys_hold(exec, run, "right", "right-on")?
                }
                "group-count" | "group-digest" | "group-min" | "group-max" | "group-ordinal" => {
                    keys_hold(exec, run, "from", "keys")?
                }
                "rank" | "extend" | "capture" | "span-bounds" | "span-between" | "source-matches"
                | "render-syntax" | "fits-rendered" | "words" | "replace" => true,
                "closure" | "cycles" | "follow" => {
                    let source = exec.arg_rel(i, "edges")?;
                    let edges = exec.schema(run, source)?;
                    let source = super::pos(&edges.names, super::s(&args["source"])?)?;
                    anchor(edges.tys[source])
                        || (step.op.as_str() != "cycles" && keys_hold(exec, run, "edges", "keys")?)
                }
                "axis" => true,
                _ => false,
            }
        };
        local.push(here);
    }
    let input_set: BTreeSet<_> = plan
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| !local[*i])
        .flat_map(|(_, s)| s.inputs.iter().copied().filter(|i| local[*i]))
        .chain(plan.emits.iter().map(|e| e.1).filter(|i| local[*i]))
        .collect();
    let local_emit = plan.emits.iter().all(|(_, n, _, _)| local[*n]);
    Ok(Partition {
        inputs: input_set.into_iter().collect(),
        file_steps: local,
        local_emit,
    })
}
