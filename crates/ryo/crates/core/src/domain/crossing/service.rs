//! The `crossing` service: free functions named by verb over the cross-file steps of one rule, with no IO and no ports.

use super::types::{CrossingInput, Evaluated, Placeability};
use crate::domain::answering::types::Todo;
use crate::domain::coverage::types::{FilePartial, Partials, RuleIndex};
use crate::domain::finish::service::carry;
use crate::domain::finish::types::{Arenas, Resolving};
use crate::domain::hosted::service::table_of;
use crate::domain::hosted::types::NumberedFiles;
use crate::domain::layout::service::shaped;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::service::evaluate_cross_with_executor;
use crate::domain::operator::types::RowsFed;
use crate::domain::spelling::service::check_spellings;

use crate::domain::rules::errors::RulesError;
use crate::domain::rules::service::failed;
use crate::domain::table_model::types::{Frontier, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use crate::domain::trust::service::matters;
use crate::domain::trust::types::Bearing;
use ryo_language::domain::relations::types::CellKind;

use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plans::types::{Operation, Plan};

/// Whether a finding of the plan can have a place in a file: a stretch, a node or a name among the columns that say what it is.
#[must_use]
pub(crate) fn placeable(plan: &Plan, table: &Table) -> Placeability {
    let places = plan
        .emit
        .key
        .iter()
        .chain(std::iter::once(&plan.emit.subject))
        .any(|column| {
            table
                .kinds
                .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                .is_some_and(|kind| {
                    matches!(kind, CellKind::Span | CellKind::Node | CellKind::Symbol)
                })
        });
    if places {
        Placeability::Placeable
    } else {
        Placeability::Unplaceable
    }
}

/// The files that govern a rule, in which a relation its cross-file steps trust the absence of rows of is read in part.
#[must_use]
pub(crate) fn partials_for(crossing: &CrossingInput<'_>, rule: RuleIndex) -> Partials {
    let trusted = crossing
        .needs
        .get(rule.0)
        .map_or(&[][..], |needs| needs.cross.as_slice());
    let matter = |partial: &&crate::domain::coverage::types::Partial| {
        trusted
            .iter()
            .any(|read| matters(read, partial) == Bearing::Matters)
    };
    Partials {
        entries: crossing
            .project
            .iter()
            .filter(matter)
            .map(|partial| FilePartial {
                file: partial.at,
                partial: partial.clone(),
            })
            .chain(
                crossing
                    .covered
                    .iter()
                    .filter(|each| each.rules.contains(&rule))
                    .flat_map(|each| {
                        each.partial
                            .iter()
                            .filter(matter)
                            .map(move |partial| FilePartial {
                                file: Some(each.file),
                                partial: partial.clone(),
                            })
                    }),
            )
            .collect(),
        unavailable_files: crossing
            .ids
            .get(rule.0)
            .into_iter()
            .flat_map(|id| crossing.skipped.iter().filter(move |skip| skip.rule == *id))
            .filter_map(|skip| {
                crossing
                    .paths
                    .iter()
                    .position(|path| *path == skip.path)
                    .and_then(|place| u32::try_from(place).ok())
                    .map(ryo_language::domain::relations::types::FileId)
            })
            .collect(),
    }
}

/// The rows the host declared, made once for the finish with their text in the names of the finish; none when the plan reads none.
fn hosted_tables(crossing: &CrossingInput<'_>, plan: &Plan, cross: &mut Symbols) -> Vec<Table> {
    let reads_hosted = plan
        .steps
        .iter()
        .any(|step| matches!(step.operation, Operation::Hosted { .. }));
    if !reads_hosted {
        return Vec::new();
    }
    let numbered: Vec<FilePath> = crossing
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect();
    crossing
        .hosted
        .iter()
        .map(|relation| {
            let mut text = Symbols::default();
            let mut table = table_of(relation, NumberedFiles(&numbered), &mut text);
            carry(&mut table, &text, cross);
            table
        })
        .collect()
}

/// Runs the cross-file steps on resident input rows, without loading sources or rendering output.
///
/// # Errors
///
/// `Failed` when a name is spelled two ways, an operator refuses or a finding cannot be rendered.
pub(crate) fn cross_rule(
    crossing: &CrossingInput<'_>,
    todo: &Todo,
    mut frontier: Vec<Frontier>,
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
    let mut fed = vec![RowsFed::default(); plan.steps.len()];
    let hosted = hosted_tables(crossing, plan, cross);
    let table = {
        let mut context = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            stored_indexes: None,
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
        let evaluated = evaluate_cross_with_executor(
            plan,
            frontier,
            &partials,
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
    Ok(Evaluated { table, fed })
}
