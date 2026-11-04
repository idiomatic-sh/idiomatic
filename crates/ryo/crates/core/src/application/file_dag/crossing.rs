//! The `crossing` service: free functions named by verb over the cross-file steps of one rule, with no IO and no ports.

use crate::application::file_dag::cross_execution::evaluate_cross_roots_with_executor;
use crate::domain::answering::types::Todo;
use crate::domain::coverage::types::{FilePartial, Partials, RuleIndex};
use crate::domain::crossing::types::{Evaluated, Placeability};
use crate::domain::finish::service::carry;
use crate::domain::finish::types::{Arenas, Resolving};
use crate::domain::hosted::service::table_of;
use crate::domain::hosted::types::NumberedFiles;
use crate::domain::layout::service::shaped;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::types::RowsFed;
use crate::domain::spelling::service::check_spellings;

use crate::domain::rules::errors::RulesError;
use crate::domain::rules::service::failed;
use crate::domain::table_model::types::Frontier;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use crate::domain::trust::service::matters;
use crate::domain::trust::types::Bearing;
use ryo_relations::domain::relations::types::CellKind;

use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plans::types::{Operation, Plan};

use super::settings::CrossingInput;
use crate::domain::crossing::service::{hosted_tables, partials_for};
/// Runs the cross-file steps on resident input rows, without loading sources or rendering output.
///
/// # Errors
///
/// `Failed` when a name is spelled two ways, an operator refuses or a finding cannot be rendered.
#[cfg(test)]
pub(crate) fn cross_rule(
    crossing: &CrossingInput<'_>,
    todo: &Todo,
    frontier: Vec<Frontier>,
    cross: &mut Symbols,
) -> Result<Evaluated, RulesError> {
    cross_rule_with_inputs(crossing, todo, frontier, &[], cross)
}

pub(crate) fn cross_rule_with_inputs(
    crossing: &CrossingInput<'_>,
    todo: &Todo,
    mut frontier: Vec<Frontier>,
    refused: &[crate::domain::crossing::types::RefusedFrontier],
    cross: &mut Symbols,
) -> Result<Evaluated, RulesError> {
    let rule = todo.rule;
    let (Some(plan), Some(bound)) = (crossing.plans.get(rule.0), crossing.bindings.get(rule.0))
    else {
        return Ok(Evaluated::default());
    };
    let refuse = |reason: &dyn std::fmt::Display| failed(&todo.id, reason);
    check_spellings(
        frontier.iter().map(|each| &each.table),
        &Resolving {
            paths: crossing.paths,
            arenas: Arenas(crossing.arenas),
            cross,
            unknown: crossing.unknown,
        },
    )
    .map_err(|error| refuse(&error))?;
    // Findings are not decided over a relation read in part; rows of the query's own columns are the rows of what was read.
    let partials = match crossing.answering {
        crate::domain::render::types::Answering::Findings => partials_for(crossing, rule),
        crate::domain::render::types::Answering::Rows(_) => Partials::default(),
    };
    let publication_reads = crate::domain::trust::service::observed_reads(plan);
    let observation_partials =
        crate::domain::crossing::service::partials_for_reads(crossing, rule, &publication_reads);
    let mut fed = vec![RowsFed::default(); plan.steps.len()];
    let hosted = hosted_tables(crossing, plan, cross);
    let roots = {
        let mut context = OperatorContext {
            semantic: None,
            prepared_kinds: None,
            prepared_matchers: None,
            bindings: bound,
            symbols: cross.into(),
            source: SourceView(&[]),
            paths: crossing.paths,
            arenas: crossing.arenas,
            memo: &mut Vec::new(),
            scratch: &mut OperatorScratch::default(),
        };
        shaped(plan, &mut frontier, &mut context).map_err(|error| refuse(&error))?;
        if tracing::enabled!(target: "ryo::diagnostic::lifecycle_memory", tracing::Level::TRACE) {
            let bytes =
                crate::domain::recording::types::retention::value::RetainedBytes::inputs(&frontier);
            tracing::trace!(target: "ryo::diagnostic::lifecycle_memory", phase = "after_cross_shape", owner = "decoded_inputs", objects = bytes.objects, decoded_live_bytes = bytes.cells_live, decoded_capacity_bytes = bytes.cells_capacity, metadata_capacity_bytes = bytes.metadata, payload_copied = false);
        }
        let evaluated = evaluate_cross_roots_with_executor(
            plan,
            frontier,
            refused,
            &partials,
            &observation_partials,
            &mut context,
            &mut fed,
            &hosted,
            crossing.executor,
            crossing
                .operators
                .rules
                .get(rule.0)
                .ok_or(crate::domain::operating::errors::OperatingError::NotRun)
                .map_err(|error| refuse(&error))?,
        );
        if tracing::enabled!(target: "ryo::diagnostic::lifecycle_memory", tracing::Level::TRACE) {
            tracing::trace!(target: "ryo::diagnostic::lifecycle_memory", phase = "after_cross_evaluate", owner = "decoded_inputs", caller_inputs_owned = false, decoded_capacity_bytes = 0_usize, success = evaluated.is_ok());
        }
        evaluated.map_err(|error| refuse(&error))?
    };
    Ok(Evaluated { roots, fed })
}

use ryo_values::domain::rows::types::Table;
