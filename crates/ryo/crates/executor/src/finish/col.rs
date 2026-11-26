//! Whole-table select kernels. Bind the single source, output schema and column positions on the first demand in the run.
use super::{E, Exec, File, Provider, R, Run, Table, recycle};
use std::rc::Rc;

mod project;
use project::{Projection, Work};

#[derive(Default)]
pub(super) struct Kernels {
    // Zero is the ordinary select entry. Dense indices keep the dispatch table small.
    by_node: Vec<u32>,
    projections: Vec<Projection>,
    work: Work,
    audit: Option<Audit>,
}

const UNBOUND: u32 = u32::MAX;

impl Exec<'_> {
    pub(crate) fn col_report(&self) {
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
            by_node: vec![UNBOUND; self.plan.nodes.len()],
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
        self.col = kernels;
        if self.col.audit.is_some() {
            // An inventory also reports nodes which no file reaches.
            for node in 0..self.plan.nodes.len() {
                self.bind_projection(run, node);
            }
        }
    }

    #[inline]
    pub(super) fn projection(&mut self, run: &mut Run, node: usize) -> Option<usize> {
        let slot = *self.col.by_node.get(node)?;
        if slot == UNBOUND {
            self.bind_projection(run, node);
            self.col.by_node[node].checked_sub(1).map(|i| i as usize)
        } else {
            slot.checked_sub(1).map(|i| i as usize)
        }
    }

    #[inline(never)]
    fn bind_projection(&mut self, run: &mut Run, node: usize) {
        self.col.by_node[node] = 0;
        let plan = self.plan;
        let pn = &plan.nodes[node];
        if pn.op() != "select" || pn.tested() || pn.counted() || pn.semi().is_some() {
            return;
        }
        let &[source] = &*pn.inputs else {
            return;
        };
        let Some(input) = plan.nodes.get(source) else {
            return;
        };
        if matches!(input.op(), "closure" | "base" | "rows")
            || !matches!(self.view(run, source), Ok(None))
        {
            return;
        }
        // Preserve the executor's on-demand errors for unsupported selects on unreachable paths.
        let Ok(sel) = self.sel(run, node) else {
            return;
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
            return;
        };
        let Ok(schema) = self.schema(run, node) else {
            return;
        };
        let Ok(input) = self.schema(run, source) else {
            return;
        };
        let ident = schema.names == input.names && picks.iter().copied().eq(0..input.names.len());
        let distinct =
            !sw!("VEC_SELECT_BAG") && !sw!("VEC_UNION_ALL") && picks.len() < input.names.len();
        let projection = Projection {
            source,
            schema,
            picks,
            ident,
            distinct,
        };
        let kernels = &mut self.col;
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
                let old = self.select_entry(f, run, p, node)?;
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
