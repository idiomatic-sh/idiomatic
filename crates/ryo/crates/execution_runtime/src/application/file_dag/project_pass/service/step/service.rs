//! One occurrence's node step after the files: a node run through the file pass's one node step, its rows' coverage, and the
//! rows each of its rule's steps was fed.

use crate::application::file_dag::file_pass::service::nodes::service::{run_node, step_coverage_with};
use crate::application::file_dag::file_pass::types::NodeReads;
use crate::application::file_dag::frontiers::service::{handed_coverage, held_frontier, reason};
use crate::application::file_dag::frontiers::types::{HandedOver, Withholdings};
use crate::application::file_dag::project_pass::types::PhaseBatch;
use ryo_operators::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::application::file_dag::project_pass::types::{PhaseInputs, PhaseResources};
use crate::application::file_dag::publications::service::{bound_coverage, held_observation};
use ryo_execution_domain::domain::completion::service::{lacks_of, partial_of_lacks};
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::completion::types::{CoverageState, RegionAdmission};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::service::{rule as rule_nodes, schema};
use ryo_execution_domain::domain::executable_program::types::RuleNodes;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::service::portable_fault;
use ryo_execution_domain::domain::operator::types::{LogicalCounts, RowsFed};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_operators::domain::physical_plan::types::{NodeRows, PlanNodeId};
use ryo_relations::domain::relations::service::kinds as relation_kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::lineage::service::one_file;
use ryo_values::domain::lineage::types::Lineage;
use ryo_values::domain::plan_parts::types::{Input, InputNumber, StepId, StepScope};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::rows::service::reset;
use ryo_values::domain::rows::types::Table;

/// One occurrence's node step: the phase's buffers and inputs, and the rule whose steps it runs.
pub(crate) struct StepRun<'r, 'b, 's> {
    pub(crate) batch: &'r PhaseBatch<'b, 's>,
    pub(crate) buffers: &'r mut FileBuffers,
    pub(crate) withholdings: &'r Withholdings,
    pub(crate) inputs: &'r PhaseInputs<'r>,
    pub(crate) elements: &'r Elements,
    pub(crate) rule: RuleIndex,
    pub(crate) completion: &'r CompletionPolicy,
}

impl StepRun<'_, '_, '_> {
    /// The rows of `node`, the node of the rule's step `step`, and their coverage; empty and partial when a read it trusts the
    /// absence of is partial.
    pub(crate) fn answer(
        &mut self,
        symbols: &mut Symbols,
        node: PlanNodeId,
        step: StepId,
    ) -> Result<(Table, CoverageState), OperatingError> {
        let empty = NodeReads::default();
        let reads = self
            .batch
            .pass
            .reads
            .get(self.rule.0)
            .and_then(|reads| reads.get(usize::try_from(step.0).unwrap_or(usize::MAX)))
            .unwrap_or(&empty);
        if self.completion.region_admission(&reads.needs) == RegionAdmission::Blocked {
            let mut table = Table::default();
            reset(&mut table, schema(&self.batch.pass.program, node)?);
            return Ok((table, self.completion.blocked_coverage()));
        }
        self.ran(symbols, node)?;
        let table = match self.buffers.rows(node) {
            NodeRows::Materialized(table) => table.clone(),
            NodeRows::Absent => return Err(OperatingError::NotRun),
        };
        let coverage = self.coverage(step)?;
        Ok((table, coverage))
    }

    /// Runs `node` through the file pass's node step, a withheld input answered with the reason it was withheld.
    pub(crate) fn ran(&mut self, symbols: &mut Symbols, node: PlanNodeId) -> Result<(), OperatingError> {
        let mut resources = PhaseResources {
            symbols,
            paths: self.batch.request.paths(),
            arenas: self.batch.arenas,
            elements: self.elements,
        };
        let ran = run_node(self.batch.pass, node, self.buffers, self.inputs, &mut resources);
        if self.inputs.unhanded.replace(false) {
            return Err(OperatingError::NotRead);
        }
        ran.map_err(|fault| match fault {
            PortableOperatorFault::Withheld(token) => reason(self.withholdings, token),
            other => portable_fault(other),
        })
    }

    /// The coverage of the rule's step `step`: each input the files or the project handed over covered as its handover proved,
    /// each bound observation as its producers published it, and each step on the way passing on what its inputs lack as its
    /// operator does.
    pub(crate) fn coverage(&self, step: StepId) -> Result<CoverageState, OperatingError> {
        let rule = self.rule;
        let program = &self.batch.pass.program;
        let plan = self.batch.pass.plans.get(rule.0).ok_or(OperatingError::NotRun)?;
        let nodes = rule_nodes(program, rule)?;
        let handed = self.inputs.handed;
        let bound = self.inputs.bound;
        let frontier = &plan.frontier;
        step_coverage_with(self.batch.pass, rule, step, |input, scope| {
            let place = frontier.iter().position(|each| *each == input);
            match input {
                Input::Relation(relation) => {
                    let kinds = relation_kinds(relation);
                    Some(Ok((kinds.to_vec(), handed_at(handed, place), one_file(kinds))))
                }
                Input::Step(id) if scope == StepScope::PerFile => {
                    let kinds = match nodes.steps.get(usize::try_from(id.0).unwrap_or(usize::MAX)) {
                        Some(Ok(node)) => schema(program, *node).map_or_else(|_| Vec::new(), <[CellKind]>::to_vec),
                        Some(Err(_)) | None => Vec::new(),
                    };
                    Some(Ok((kinds, handed_at(handed, place), Lineage::Mixed)))
                }
                Input::Step(_) => None,
                // A binding fed in each file was handed over with the frontier; one fed after the files was bound.
                Input::BoundObservation(id) => Some(Ok((
                    Vec::new(),
                    place
                        .and_then(|at| handed_coverage(handed, InputNumber(at)))
                        .or_else(|| bound_coverage(bound, id))
                        .cloned()
                        .unwrap_or(CoverageState::Complete),
                    Lineage::Mixed,
                ))),
            }
        })
    }

    /// The rows each step of the rule was fed after the files: a step that runs in each file was fed in the files.
    pub(crate) fn fed(
        &mut self,
        symbols: &mut Symbols,
        plan: &Plan,
        nodes: &RuleNodes,
    ) -> Result<LogicalCounts, OperatingError> {
        let rule = ordinal(self.rule)?;
        let mut fed = Vec::with_capacity(plan.steps.len());
        for each in &plan.steps {
            let mut counts = [0_u64; 3];
            if each.scope == StepScope::CrossFile {
                for (slot, input) in counts.iter_mut().zip(named(&inputs_of(&each.operation))) {
                    let place = plan.frontier.iter().position(|entry| entry == input);
                    *slot = match *input {
                        Input::Step(id)
                            if plan
                                .steps
                                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                                .is_some_and(|read| read.scope == StepScope::CrossFile) =>
                        {
                            let read = nodes
                                .steps
                                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                                .ok_or(OperatingError::NotRun)?
                                .clone()?;
                            self.ran(symbols, read)?;
                            match self.buffers.rows(read) {
                                NodeRows::Materialized(table) => {
                                    u64::try_from(table.count.0).unwrap_or(u64::MAX)
                                }
                                NodeRows::Absent => 0,
                            }
                        }
                        Input::Relation(_) | Input::Step(_) => place.map_or(0, |at| {
                            match held_frontier(
                                self.inputs.handed,
                                FrontierPlace::of(rule, InputNumber(at)),
                            ) {
                                HeldRows::Held(table) => u64::try_from(table.count.0).unwrap_or(u64::MAX),
                                HeldRows::Withheld(_) => 0,
                            }
                        }),
                        Input::BoundObservation(_) if place.is_some_and(|at| handed_coverage(self.inputs.handed, InputNumber(at)).is_some()) => {
                            place.map_or(0, |at| match held_frontier(self.inputs.handed, FrontierPlace::of(rule, InputNumber(at))) {
                                HeldRows::Held(table) => u64::try_from(table.count.0).unwrap_or(u64::MAX),
                                HeldRows::Withheld(_) => 0,
                            })
                        }
                        Input::BoundObservation(id) => {
                            match held_observation(self.inputs.bound, BoundObservationPlace::of(rule, id)) {
                                HeldRows::Held(table) => u64::try_from(table.count.0).unwrap_or(u64::MAX),
                                HeldRows::Withheld(_) => 0,
                            }
                        }
                    };
                }
            }
            fed.push(RowsFed(counts));
        }
        Ok(LogicalCounts {
            fed: fed.into_boxed_slice(),
        })
    }
}

/// The coverage the handover proved for frontier entry `place`.
fn handed_at(handed: &HandedOver, place: Option<usize>) -> CoverageState {
    place
        .and_then(|at| handed_coverage(handed, InputNumber(at)))
        .cloned()
        .unwrap_or(CoverageState::Complete)
}

/// `existing` with what `added` lacks: a partial answer stays partial, a file out of scope lacks any row of itself.
pub(crate) fn with_coverage(existing: CoverageState, added: CoverageState) -> CoverageState {
    match added {
        CoverageState::Complete => existing,
        CoverageState::Partial { files, lacks } => {
            let mut all = lacks_of(&files, &lacks);
            match &existing {
                CoverageState::Complete => {}
                CoverageState::Partial {
                    files: held,
                    lacks: held_lacks,
                } => all.extend(lacks_of(held, held_lacks)),
                CoverageState::Inapplicable { files: held } => {
                    all.extend(lacks_of(&held.iter().copied().map(Some).collect::<Vec<_>>(), &[]));
                }
            }
            match partial_of_lacks(all) {
                partial @ CoverageState::Partial { .. } => partial,
                CoverageState::Complete | CoverageState::Inapplicable { .. } => existing,
            }
        }
        CoverageState::Inapplicable { files } => match existing {
            CoverageState::Complete => CoverageState::Inapplicable { files },
            CoverageState::Partial {
                files: mut held,
                mut lacks,
            } => {
                for file in files {
                    lacks.retain(|each| each.file != Some(file));
                    if !held.contains(&Some(file)) {
                        held.push(Some(file));
                    }
                }
                CoverageState::Partial { files: held, lacks }
            }
            CoverageState::Inapplicable { files: mut held } => {
                for file in files {
                    if !held.contains(&file) {
                        held.push(file);
                    }
                }
                CoverageState::Inapplicable { files: held }
            }
        },
    }
}

/// `rule`'s place among the program's rules, as the plan names it.
pub(crate) fn ordinal(rule: RuleIndex) -> Result<RuleOrdinal, OperatingError> {
    u32::try_from(rule.0)
        .map(RuleOrdinal)
        .map_err(|_| OperatingError::ShapeMismatch)
}
