//! A rule's node in the open file: run by the operators, its rows answered with their coverage, and every node's rows
//! released when the file ends.

use crate::application::file_dag::file_pass::types::{
    NodeAnswer, NodeReads, PassFile, PassProgram, PassResources,
};
use crate::application::file_dag::program::facade_runtime::file_execution::service::classify;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::completion::service::partial_any;
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::completion::types::RegionAdmission;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::service::{
    consumers, physical, rule as fused_rule, schema,
};
use ryo_execution_domain::domain::frontier_coverage::service::affected_files;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::service::portable_fault;
use ryo_execution_domain::domain::output_coverage::service::output_coverage;
use ryo_execution_domain::domain::output_coverage::types::{InputCoverage, OperandOrdinal};
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::physical_plan::types::{FactRows, PlanInputs as _};
use ryo_operators::domain::physical_plan::types::{NodeRows, PlanNodeId};
use ryo_operators::domain::tree_cells::service::native_form;
use ryo_relations::domain::relations::service::kinds as relation_kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::lineage::service::{of_step, one_file};
use ryo_values::domain::lineage::types::Lineage;
use ryo_values::domain::plan_parts::types::{Input, StepId};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::rows::service::reset;
use ryo_values::domain::rows::types::Table;

/// What a rule's node, the node of its plan's step `step`, answers in the open file: its rows, complete or partial as its
/// reads are; empty and partial when a read it trusts the absence of is partial; or the fault it raised.
pub(crate) fn node_outcome(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    rule: RuleIndex,
    node: Result<PlanNodeId, OperatingError>,
    step: StepId,
) -> RootOutcome {
    match node_rows(at, symbols, rule, node, step) {
        Ok(NodeAnswer { table, coverage }) => match affected_files(&coverage) {
            None => RootOutcome::Complete(table),
            Some(affected) => RootOutcome::Partial { table, affected },
        },
        Err(error) => classify(error),
    }
}

/// A rule's node, the node of its plan's step `step`, its rows in the open file and their coverage.
pub(crate) fn node_rows(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    rule: RuleIndex,
    node: Result<PlanNodeId, OperatingError>,
    step: StepId,
) -> Result<NodeAnswer, OperatingError> {
    let empty = NodeReads::default();
    let pass = at.pass;
    let reads = pass
        .reads
        .get(rule.0)
        .and_then(|reads| reads.get(usize::try_from(step.0).unwrap_or(usize::MAX)))
        .unwrap_or(&empty);
    let node = match node {
        Ok(node) => node,
        Err(error) => {
            // A step's schema fault is answered once its sources are admitted: a relation the facts refuse stops the file first.
            for read in &reads.uses {
                if native_form(read.relation).is_none()
                    && matches!(at.inputs.facts(read.relation), FactRows::Refused)
                {
                    break;
                }
            }
            return Err(error);
        }
    };
    let completion = CompletionPolicy::file_inputs(rule, at.file, at.evaluation, at.publication);
    if completion.region_admission(&reads.needs) == RegionAdmission::Blocked {
        let mut table = Table::default();
        reset(&mut table, schema(&at.pass.program, node)?);
        return Ok(NodeAnswer {
            table,
            coverage: completion.blocked_coverage(),
        });
    }
    ran_node(at, symbols, node)?;
    match at.buffers.rows(node) {
        NodeRows::Materialized(table) => {
            let table = table.clone();
            // What no read of the step lacks, no operator on the way can lack.
            let coverage = match completion.coverage(&reads.uses) {
                CoverageState::Complete => CoverageState::Complete,
                CoverageState::Partial { .. } | CoverageState::Inapplicable { .. } => {
                    step_coverage(pass, &completion, rule, step)?
                }
            };
            Ok(NodeAnswer { table, coverage })
        }
        NodeRows::Absent => Err(OperatingError::NotRun),
    }
}

/// What one step's rows lack, with their cell kinds and where they come from.
type StepCoverage = (Vec<CellKind>, CoverageState, Lineage);

/// The coverage of step `step` of `rule`'s plan in the open file: each relation it reads covered as the file's partiality says
/// of it, and each step on the way passing on what its inputs lack as its operator does (`output_coverage`), so a missing row
/// that could change another file's answer makes the repository partial.
fn step_coverage(
    pass: &PassProgram,
    completion: &CompletionPolicy,
    rule: RuleIndex,
    step: StepId,
) -> Result<CoverageState, OperatingError> {
    let plan = pass.plans.get(rule.0).ok_or(OperatingError::NotRun)?;
    let nodes = fused_rule(&pass.program, rule)?;
    let last = usize::try_from(step.0).unwrap_or(usize::MAX);
    let mut merged: Vec<StepCoverage> = Vec::with_capacity(last.saturating_add(1));
    for (place, each) in plan.steps.iter().enumerate().take(last.saturating_add(1)) {
        let operands = named(&inputs_of(&each.operation))
            .into_iter()
            .map(|input| match *input {
                Input::Relation(relation) => {
                    let kinds = relation_kinds(relation);
                    Ok((
                        kinds.to_vec(),
                        completion.relation_coverage(relation),
                        one_file(kinds),
                    ))
                }
                Input::Step(id) => merged
                    .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                    .cloned()
                    .ok_or(OperatingError::NotRun),
                Input::BoundObservation(_) => Err(OperatingError::NotRun),
            })
            .collect::<Result<Vec<StepCoverage>, OperatingError>>()?;
        let output = match nodes.steps.get(place) {
            Some(Ok(node)) => schema(&pass.program, *node).map(<[CellKind]>::to_vec),
            Some(Err(error)) => Err(error.clone()),
            None => Err(OperatingError::NotRun),
        };
        let (kinds, coverage) = match output {
            Ok(kinds) => {
                let inputs: Vec<InputCoverage<'_>> = operands
                    .iter()
                    .enumerate()
                    .map(|(ordinal, (kinds, coverage, lineage))| InputCoverage {
                        ordinal: OperandOrdinal(ordinal),
                        kinds,
                        coverage: coverage.clone(),
                        lineage: lineage.clone(),
                    })
                    .collect();
                let coverage = output_coverage(&each.operation, &kinds, &inputs)?;
                (kinds, coverage)
            }
            // No lineage is certified from a refused schema.
            Err(_) => (Vec::new(), partial_any(vec![None])),
        };
        let lineages: Vec<Lineage> = operands
            .into_iter()
            .map(|(_, _, lineage)| lineage)
            .collect();
        let lineage = of_step(&each.operation, &lineages, &kinds);
        merged.push((kinds, coverage, lineage));
    }
    merged
        .into_iter()
        .nth(last)
        .map(|(_, coverage, _)| coverage)
        .ok_or(OperatingError::NotRun)
}

/// Runs `node` for the open file through the operators.
pub(crate) fn ran_node(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    node: PlanNodeId,
) -> Result<(), OperatingError> {
    let mut resources = PassResources {
        symbols,
        source: at.source,
        paths: at.paths,
        elements: at.inputs.lookup.tree(),
    };
    at.pass
        .operators
        .run(
            physical(&at.pass.program),
            node,
            &mut at.buffers,
            &at.inputs,
            &mut resources,
        )
        .map_err(portable_fault)
}

/// Ends the file: every node's rows go back to the buffers.
pub(crate) fn released(at: &mut PassFile<'_, '_, '_, '_>) {
    let mut node = PlanNodeId(0);
    while consumers(&at.pass.program, node).is_ok() {
        at.buffers.release(node);
        node = PlanNodeId(node.0.saturating_add(1));
    }
}
