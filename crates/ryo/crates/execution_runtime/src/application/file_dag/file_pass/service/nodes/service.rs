//! A rule's node in the open file: run by the operators, its rows answered with their coverage, and every node's rows
//! released when the file ends.

use ryo_operators::domain::physical_plan::types::{FactRows, PlanInputs as _};
use ryo_operators::domain::tree_cells::service::native_form;
use crate::application::file_dag::program::facade_runtime::file_execution::service::classify;
use crate::application::file_dag::file_pass::types::{NodeAnswer, NodeReads, PassFile, PassResources};
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::completion::types::RegionAdmission;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::frontier_coverage::service::affected_files;
use ryo_execution_domain::domain::fused_program::service::{consumers, physical, schema};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::service::portable_fault;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::physical_plan::types::{NodeRows, PlanNodeId};
use ryo_values::domain::rows::service::reset;
use ryo_values::domain::rows::types::Table;

/// What a rule's node answers in the open file: its rows, complete or partial as its reads are; empty and partial when a
/// read it trusts the absence of is partial; or the fault it raised.
pub(crate) fn node_outcome(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    rule: RuleIndex,
    node: Result<PlanNodeId, OperatingError>,
    reads: Option<&NodeReads>,
) -> RootOutcome {
    match node_rows(at, symbols, rule, node, reads) {
        Ok(NodeAnswer { table, coverage }) => match affected_files(&coverage) {
            None => RootOutcome::Complete(table),
            Some(affected) => RootOutcome::Partial { table, affected },
        },
        Err(error) => classify(error),
    }
}

/// A rule's node's rows in the open file and their coverage.
pub(crate) fn node_rows(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    rule: RuleIndex,
    node: Result<PlanNodeId, OperatingError>,
    reads: Option<&NodeReads>,
) -> Result<NodeAnswer, OperatingError> {
    let empty = NodeReads::default();
    let reads = reads.unwrap_or(&empty);
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
        NodeRows::Materialized(table) => Ok(NodeAnswer {
            table: table.clone(),
            coverage: completion.coverage(&reads.uses),
        }),
        NodeRows::Absent => Err(OperatingError::NotRun),
    }
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
