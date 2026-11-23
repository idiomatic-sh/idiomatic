//! Whole-table select kernels with sources, output schemas and column positions bound at plan load.
use super::{E, Exec, File, Provider, R, Run, Table, recycle};
use std::rc::Rc;

mod join;
mod join_audit;
mod project;
use project::{Projection, Work};

#[derive(Clone, Copy, Default)]
pub(super) enum Operator {
    #[default]
    Ordinary,
    Projection(u32),
    Join(u32),
}

#[derive(Default)]
pub(super) struct Kernels {
    // One word per plan node: ordinary (zero), then projections, then joins.
    by_node: Vec<u32>,
    joins: Vec<join::Join>,
    join_work: join::Work,
    join_audit: Option<join_audit::Audit>,
    projections: Vec<Projection>,
    work: Work,
    audit: Option<Audit>,
}

impl Kernels {
    #[inline]
    pub(super) fn operator(&self, node: usize) -> Operator {
        let slot = self.by_node.get(node).copied().unwrap_or(0);
        if slot == 0 {
            Operator::Ordinary
        } else if slot <= self.projections.len() as u32 {
            Operator::Projection(slot - 1)
        } else {
            Operator::Join(slot - self.projections.len() as u32 - 1)
        }
    }
}

impl Exec<'_> {
    pub(crate) fn col_report(&mut self) {
        if let Some(audit) = &mut self.col.join_audit {
            audit.report();
        }
        if let Some(audit) = &self.col.audit {
            audit.print();
        }
    }

    pub(crate) fn col_load(&mut self, run: &mut Run) {
        if sw!("VEC_NO_COL")
            || sw!("VEC_OLD_SELECT")
            || sw!("VEC_OLD_EMPTYSEL")
            || sw!("VEC_ESEL_WHY")
            || sw!("VEC_KSEL_PROF")
            || sw!("VEC_KSEL_DISTINCT")
        {
            return;
        }
        let mut kernels = Kernels {
            by_node: vec![0; self.plan.nodes.len()],
            ..Kernels::default()
        };
        let check = sw!("VEC_COL_CHECK");
        if check || sw!("VEC_COL_PROFILE") {
            kernels.audit = Some(Audit {
                check,
                rows: Vec::new(),
                profile: sw!("VEC_COL_PROFILE"),
                phases: [0.0; 3],
            });
        }
        let plan = self.plan;
        for (node, pn) in plan.nodes.iter().enumerate() {
            if pn.op != "select"
                || !pn.j["cond"].is_null()
                || !pn.j["count"].is_null()
                || !pn.j["semi"].is_null()
            {
                continue;
            }
            let Some([src]) = pn.j["src"].as_array().map(Vec::as_slice) else {
                continue;
            };
            let Some(source) = src[0].as_u64().and_then(|n| usize::try_from(n).ok()) else {
                continue;
            };
            let Some(input) = plan.nodes.get(source) else {
                continue;
            };
            if matches!(input.op.as_str(), "closure" | "base" | "rows")
                || !matches!(self.view(run, source), Ok(None))
            {
                continue;
            }
            // Preserve the executor's on-demand errors for unsupported selects on unreachable paths.
            let Ok(sel) = self.sel(run, node) else {
                continue;
            };
            let Some(picks) = sel
                .items
                .iter()
                .map(|(_, e)| match e {
                    E::Col(0, c, _) => Some(*c),
                    _ => None,
                })
                .collect::<Option<Box<[_]>>>()
            else {
                continue;
            };
            let Ok(schema) = self.schema(run, node) else {
                continue;
            };
            let Ok(input) = self.schema(run, source) else {
                continue;
            };
            let ident =
                schema.names == input.names && picks.iter().copied().eq(0..input.names.len());
            let distinct =
                !sw!("VEC_SELECT_BAG") && !sw!("VEC_UNION_ALL") && picks.len() < input.names.len();
            let projection = Projection {
                source,
                schema,
                picks,
                ident,
                distinct,
            };
            if let Some(a) = &mut kernels.audit {
                a.rows.push(Counts {
                    node,
                    kind: projection.kind(),
                    ..Counts::default()
                });
            }
            kernels.projections.push(projection);
            kernels.by_node[node] = kernels.projections.len() as u32;
        }
        join::load(self, run, &mut kernels);
        self.col = kernels;
    }

    pub(super) fn col_select(
        &mut self,
        f: &mut File<'_>,
        run: &mut Run,
        p: &mut dyn Provider,
        node: usize,
        index: usize,
    ) -> R<Rc<Table>> {
        let profile = self.col.audit.as_ref().is_some_and(|a| a.profile);
        let mark = || {
            if profile {
                super::select_kernel::ticks()
            } else {
                0.0
            }
        };
        let start = mark();
        let nested = self.inner;
        let input = self.eval(f, run, p, self.col.projections[index].source)?;
        let read = mark();
        let projection = &self.col.projections[index];
        let count = input.n;
        let work = &mut self.col.work;
        let selected = projection.distinct && count > 1 && work.distinct(&input, &projection.picks);
        let keyed = mark();
        let output = if count == 0 {
            self.empty_rc(run, node)?
        } else if projection.ident {
            input
        } else {
            self.stats.touched[node] += count as u64;
            recycle::rc(projection.gather(&input, selected.then_some(work.kept.as_slice())))
        };
        if let Some(a) = &mut self.col.audit {
            let row = &mut a.rows[index];
            row.calls += 1;
            row.input += count as u64;
            row.output += output.n as u64;
            if profile {
                let phases = [
                    read - start - (self.inner - nested),
                    keyed - read,
                    mark() - keyed,
                ];
                for (i, seconds) in phases.into_iter().enumerate() {
                    row.phases[i] += seconds;
                    a.phases[i] += seconds;
                }
            }
            if a.check {
                // The same immutable source is already memoized. Compare every cell in physical order.
                let touched = self.stats.touched[node];
                let old = self.select_entry(f, run, p, node)?;
                self.stats.touched[node] = touched;
                assert!(
                    old.n == output.n
                        && old.names == output.names
                        && old.tys == output.tys
                        && old.cols == output.cols,
                    "projection differs at step {node}"
                );
            }
        }
        Ok(output)
    }
}

#[derive(Default)]
struct Counts {
    node: usize,
    kind: &'static str,
    calls: u64,
    input: u64,
    output: u64,
    phases: [f64; 3],
}

struct Audit {
    check: bool,
    profile: bool,
    rows: Vec<Counts>,
    phases: [f64; 3],
}

impl Audit {
    fn print(&self) {
        for c in &self.rows {
            println!(
                "COL_NODE\t{}\t{}\t{}\t{}\t{}",
                c.node, c.kind, c.calls, c.input, c.output
            );
            if self.profile && c.calls != 0 {
                println!(
                    "COL_PHASE_NODE\t{}\t{:.9}\t{:.9}\t{:.9}",
                    c.node, c.phases[0], c.phases[1], c.phases[2]
                );
            }
        }
        if self.profile {
            println!(
                "COL_PHASE source {:.6} distinct {:.6} gather {:.6}",
                self.phases[0], self.phases[1], self.phases[2]
            );
        }
    }
}
