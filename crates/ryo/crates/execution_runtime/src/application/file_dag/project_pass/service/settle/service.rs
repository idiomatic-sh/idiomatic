//! A rule's steps after the files settled in plan order: the rows each antijoin or difference answered from the absence of
//! rows that a partial operand could change are dropped where the step stands.

use crate::application::file_dag::project_pass::service::step::service::StepRun;
use crate::application::file_dag::project_pass::types::SettledStep;
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::contributor_rows::service::filter_contributor_rows;
use ryo_execution_domain::domain::executable_program::service::rule as rule_nodes;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::service::semantics;
use ryo_execution_domain::domain::output_coverage::types::{InputCoverage, OperandOrdinal};
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_operators::domain::physical_plan::types::NodeRows;
use ryo_values::domain::plan_parts::types::{StepId, StepScope};
use ryo_values::domain::plans::service::kind_of;

impl StepRun<'_, '_, '_> {
    /// Drops, in plan order up to the rule's step `step`, the rows each antijoin or difference after the files answered from the
    /// absence of rows that a partial operand could change, where the step stands and before any later step reads them: the
    /// one change the pass makes to rows the operators lent it (`FileBuffers::retain`).
    pub(crate) fn settled(
        &mut self,
        symbols: &mut Symbols,
        step: StepId,
    ) -> Result<(), OperatingError> {
        let pass = self.batch.pass;
        let plan = pass.plans.get(self.rule.0).ok_or(OperatingError::NotRun)?;
        let mut pending: Vec<SettledStep> = Vec::new();
        let _coverage = self.coverage_seen(step, |id, operands| {
            let Some(each) = plan.steps.get(usize::try_from(id.0).unwrap_or(usize::MAX)) else {
                return;
            };
            let absence = semantics(kind_of(&each.operation)).absence;
            if each.scope == StepScope::CrossFile
                && absence.0 != 0
                && operands
                    .iter()
                    .any(|(_, coverage, _)| *coverage != CoverageState::Complete)
            {
                pending.push(SettledStep {
                    step: id,
                    operands: operands.to_vec(),
                    absence,
                });
            }
        })?;
        let nodes = rule_nodes(&pass.program, self.rule)?;
        for settled in pending {
            let at = usize::try_from(settled.step.0).unwrap_or(usize::MAX);
            let Some(Ok(node)) = nodes.steps.get(at) else {
                continue;
            };
            let Some(each) = plan.steps.get(at) else {
                continue;
            };
            self.ran(symbols, *node)?;
            let NodeRows::Materialized(table) = self.buffers.rows(*node) else {
                continue;
            };
            let mut kept = table.clone();
            let inputs: Vec<InputCoverage<'_>> = settled
                .operands
                .iter()
                .enumerate()
                .map(|(ordinal, (kinds, coverage, lineage))| InputCoverage {
                    ordinal: OperandOrdinal(ordinal),
                    kinds,
                    coverage: coverage.clone(),
                    lineage: lineage.clone(),
                })
                .collect();
            filter_contributor_rows(&each.operation, settled.absence, &inputs, &mut kept);
            if kept.count == table.count {
                continue;
            }
            // The kept rows are the step's rows less the uncertain ones, in their order: each row is kept when it is the next.
            let width = kept.kinds.len();
            let mut next = 0_usize;
            self.buffers.retain(*node, |row| {
                let start = next.saturating_mul(width);
                let wanted = kept.cells.get(start..start.saturating_add(width));
                if next < kept.count.0 && wanted == Some(row) {
                    next = next.saturating_add(1);
                    PredicateOutcome::Holds
                } else {
                    PredicateOutcome::Fails
                }
            });
        }
        Ok(())
    }
}
