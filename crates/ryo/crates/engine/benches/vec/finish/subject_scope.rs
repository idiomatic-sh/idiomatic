//! Propagate final file-subject admission only when every reachable use of a
//! population preserves the same column. Other operands may explain a finding
//! about a different file and retain their complete population.
use super::{E, Exec, R, Rc, Run, Table, Ty};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Use {
    #[default]
    Unseen,
    Whole,
    File(usize),
}

impl Use {
    fn observe(&mut self, column: Option<usize>) {
        let seen = column.map_or(Self::Whole, Self::File);
        *self = match *self {
            Self::Unseen => seen,
            same if same == seen => same,
            _ => Self::Whole,
        };
    }
}

#[derive(Default)]
pub(super) struct Scope {
    cut: Vec<usize>,
    uses: Vec<Use>,
}

fn input_columns(
    exec: &mut Exec<'_>,
    run: &mut Run,
    node: usize,
    col: usize,
) -> R<Vec<Option<usize>>> {
    let step = &exec.plan.nodes[node];
    let mut out = vec![None; step.inputs.len()];
    if step.inputs.is_empty() {
        return Ok(out);
    }
    if step.op == "select" {
        let planned = exec.sel(run, node)?;
        if let E::Col(slot, c, Ty::File) = planned.items[col].1 {
            out[slot] = Some(c);
        }
        return Ok(out);
    }
    let name = exec.schema(run, node)?.names[col].clone();
    for (slot, &source) in step.inputs.iter().enumerate() {
        let schema = exec.schema(run, source)?;
        let inherited = || {
            schema
                .names
                .iter()
                .position(|n| n == &name)
                .filter(|&i| schema.tys[i] == Ty::File)
        };
        out[slot] = match step.op.as_str() {
            "union" | "project" => inherited(),
            "capture" | "extend" | "span-bounds" | "words" => {
                (schema.tys.get(col) == Some(&Ty::File)).then_some(col)
            }
            "axis"
                if exec.arg_rel(node, "from") == Ok(source)
                    && exec.arg_rel(node, "domain") != Ok(source)
                    && exec.arg_rel(node, "relation") != Ok(source) =>
            {
                inherited()
            }
            "group-count" | "group-digest" => {
                let keys = super::strs(&step.args()["keys"])?;
                if keys.into_iter().any(|k| k == name) {
                    inherited()
                } else {
                    None
                }
            }
            "antijoin" if exec.arg_rel(node, "left") != exec.arg_rel(node, "right") => {
                if exec.arg_rel(node, "left") == Ok(source) {
                    inherited()
                } else {
                    let left = super::strs(&step.args()["left-on"])?;
                    let right = super::strs(&step.args()["right-on"])?;
                    left.zip(right)
                        .find(|(key, _)| *key == name)
                        .and_then(|(_, key)| {
                            schema
                                .names
                                .iter()
                                .position(|n| n == key)
                                .filter(|&i| schema.tys[i] == Ty::File)
                        })
                }
            }
            _ => None,
        };
    }
    Ok(out)
}

impl Scope {
    fn compile(exec: &mut Exec<'_>, run: &mut Run, boundaries: Vec<usize>) -> R<Self> {
        let plan = exec.plan;
        let cut: BTreeSet<_> = boundaries.iter().copied().collect();
        let mut uses = vec![Use::Unseen; plan.nodes.len()];
        for (_, node, subject, _) in &plan.emits {
            let schema = exec.schema(run, *node)?;
            let c = super::pos(&schema.names, subject)?;
            uses[*node].observe((schema.tys[c] == Ty::File).then_some(c));
        }
        for (i, step) in plan.nodes.iter().enumerate().rev() {
            if cut.contains(&i) || step.inputs.is_empty() {
                continue;
            }
            match uses[i] {
                Use::Unseen => {}
                Use::Whole => {
                    for &source in &step.inputs {
                        uses[source].observe(None);
                    }
                }
                Use::File(col) => {
                    for (&source, key) in step.inputs.iter().zip(input_columns(exec, run, i, col)?)
                    {
                        uses[source].observe(key);
                    }
                }
            }
        }
        Ok(Self {
            cut: boundaries,
            uses,
        })
    }
}

/// All bases have been obtained and validated before an admitted subset is used.
/// Absence, public unknowns and refusal still come from the complete providers.
pub(super) fn restrict(
    exec: &mut Exec<'_>,
    run: &mut Run,
    cut: &super::population::Inputs,
    admits: impl Fn(u32) -> bool,
) -> R<()> {
    if exec.subject_scope.uses.is_empty()
        || !exec
            .subject_scope
            .cut
            .iter()
            .copied()
            .eq(cut.keys().copied())
    {
        exec.subject_scope = Scope::compile(exec, run, cut.keys().copied().collect())?;
    }
    for (i, value) in exec.run_memo.iter_mut().enumerate() {
        let Use::File(col) = exec.subject_scope.uses[i] else {
            continue;
        };
        let Some(table) = value else { continue };
        let keys = &table.cols[col];
        let keep = |word: u64| u32::try_from(word).is_ok_and(&admits);
        let Some(first_missing) = keys.iter().position(|&k| !keep(k)) else {
            continue;
        };
        let retained: Vec<_> = (0..first_missing)
            .chain((first_missing + 1..keys.len()).filter(|&r| keep(keys[r])))
            .collect();
        let mut out = Table::retained(table.names.clone(), table.tys.clone(), retained.len());
        for (dest, src) in out.cols.iter_mut().zip(&table.cols) {
            dest.extend(retained.iter().map(|&r| src[r]));
        }
        out.n = retained.len();
        *table = Rc::new(out);
    }
    Ok(())
}
