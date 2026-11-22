//! The `assembling` service: free functions that turn a program into the plan the engine walks,
//! with no IO and no ports. This is where names are resolved once: a quoted relation becomes a
//! `Relation`, a column becomes its position, a step becomes its number, a test becomes an enum.
//! Nothing after this works on names.

use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::terms::types::Vocabulary;

use crate::domain::capture_pushing::service::{push_below_captures, push_below_captures_roots};
use crate::domain::cutting::service::{cut_handover, cut_handover_roots};
use crate::domain::defining::service::locate_error;
use crate::domain::fusing::service::{fuse_keyed_reaches, fuse_selects};
use crate::domain::inlining::service::inline_views;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{Builder, HostedStep, PlannedColumn, Uniqueness};
use crate::domain::planning::service::{frontier_of, pushed};
use crate::domain::pruning::service::{without_unused, without_unused_roots};
use crate::domain::pushing::service::{push_lookups, push_lookups_roots};
use crate::domain::resolving::service::{position, step_named, step_of};
use crate::domain::rewriting::types::Planned;
use crate::domain::selection_pushing::service::{push_selections, push_selections_roots};
use crate::domain::splicing::service::lower_definitions;
use crate::domain::splicing::types::LoweredExpansions;
use crate::domain::syntax::constants::parameter_kind_named;
use crate::domain::walking::service::fuse_descents;
use crate::domain::walking::service::neighbours::service::fuse_neighbours;
use ryo_values::domain::hosted::types as hosted;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::program::types as syntax;
use ryo_values::domain::query_tokens::types::QuerySpelling;

/// Lowers a program that validation accepted.
///
/// # Errors
///
/// The first name that resolves to nothing, or comparison of unlike types, with its place.
#[cfg(test)]
pub(crate) fn lower(
    program: &syntax::WrittenProgram,
    catalog: &[Relation],
    hosted: &[hosted::HostedRelation],
) -> Result<ryo_values::domain::query_plans::types::Plan, LoweringError> {
    lower_with_observations(program, catalog, hosted, &[], &LoweredExpansions::default())
}

/// Resolve every declared producer source through the same typed lowering pass; the expansions lowered before over the
/// same fragments are spliced from `expansions`, and the ones lowered here kept there.
pub(crate) fn lower_with_observations(
    program: &syntax::WrittenProgram,
    catalog: &[Relation],
    hosted: &[hosted::HostedRelation],
    observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    expansions: &LoweredExpansions,
) -> Result<ryo_values::domain::query_plans::types::Plan, LoweringError> {
    let observation_inputs = observations.to_vec();
    let mut kinds = Vec::new();
    for parameter in &program.parameters {
        let kind = parameter_kind_named(&parameter.kind.0).ok_or_else(|| {
            LoweringError::UnknownParameterType {
                name: parameter.kind.0.to_string(),
                span: parameter.span,
            }
        })?;
        kinds.push(kind);
    }
    let mut builder = Builder {
        catalog,
        observation_inputs: observations,
        hosted_steps: Vec::new(),
        parameters: &program.parameters,
        kinds,
        steps: Vec::new(),
        lowered: Vec::new(),
        names: crate::domain::lowering::types::StepNames::default(),
        span: program.emit.span,
        origin: None,
        comparison_origins: Vec::new(),
    };
    // The relations the host declared are steps that run once, after the files; the ones nothing reads are dropped with the unused steps.
    for (place, relation) in hosted.iter().enumerate() {
        let number = parts::HostedNumber(u32::try_from(place).unwrap_or(u32::MAX));
        let fields = relation
            .columns
            .iter()
            .map(|column| PlannedColumn {
                name: ryo_values::domain::descriptions::types::ColumnName(column.name.0.clone()),
                kind: column.kind,
                unique: Uniqueness::Shared,
                words: Vocabulary::Open,
            })
            .collect();
        let id = pushed(
            &mut builder,
            None,
            plans::Operation::Hosted { relation: number },
            fields,
        );
        builder.hosted_steps.push(HostedStep {
            name: relation.name.clone(),
            step: id,
        });
    }
    lower_definitions(
        &mut builder,
        program,
        expansions,
        catalog,
        hosted,
        observations,
    )?;
    builder.span = program.emit.span;
    let named = step_named(&builder, &program.emit.step)
        .map_err(|error| locate_error(program.emit_origin.clone(), error))?;
    let fields = step_of(&builder.lowered, named).fields.clone();
    let observed = program
        .observations
        .iter()
        .map(|output| {
            let step = step_named(&builder, &output.step)
                .map_err(|error| locate_error(output.origin.clone(), error))?;
            let columns = step_of(&builder.lowered, step)
                .fields
                .iter()
                .map(
                    |field| ryo_values::domain::observations::types::ObservationColumn {
                        name: ryo_values::domain::observations::types::ObservationColumnName(
                            field.name.0.clone(),
                        ),
                        kind: field.kind,
                    },
                )
                .collect();
            Ok((output.name.clone(), step, columns))
        })
        .collect::<Result<Vec<_>, LoweringError>>()?;
    let (steps, emitted, observations) = if observed.is_empty() {
        let Planned {
            steps: pushed,
            emitted: named,
        } = push_lookups(
            fuse_neighbours(fuse_descents(fuse_selects(inline_views(std::mem::take(
                &mut builder.steps,
            ))))),
            named,
        );
        let Planned { steps, emitted } = without_unused(pushed, named);
        let Planned { steps, emitted } = push_below_captures(steps, emitted);
        let Planned { steps, emitted } = push_selections(steps, emitted);
        let Planned { steps, emitted } = cut_handover(steps, emitted);
        // Full-scan/exported consumers remain ordinary; pruning removes only fully replaced closure chains.
        let Planned { steps, emitted } =
            without_unused(fuse_keyed_reaches(steps, &[emitted]), emitted);
        (steps, emitted, Vec::new())
    } else {
        let mut roots = std::iter::once(named)
            .chain(observed.iter().map(|output| output.1))
            .collect::<Vec<_>>();
        let steps = fuse_neighbours(fuse_descents(fuse_selects(inline_views(std::mem::take(
            &mut builder.steps,
        )))));
        let steps = push_lookups_roots(steps, &mut roots);
        let steps = without_unused_roots(steps, &mut roots);
        let steps = push_below_captures_roots(steps, &mut roots);
        let steps = push_selections_roots(steps, &mut roots);
        let steps = cut_handover_roots(steps, &mut roots);
        let steps = fuse_keyed_reaches(steps, &roots);
        let steps = without_unused_roots(steps, &mut roots);
        let observations = observed
            .into_iter()
            .zip(roots.iter().skip(1))
            .map(|((name, _, columns), step)| {
                ryo_values::domain::observations::types::PlanObservation {
                    name,
                    source: parts::Input::Step(*step),
                    columns,
                    scope: step_of(&steps, *step).scope,
                }
            })
            .collect();
        (steps, roots[0], observations)
    };
    let subject = position(
        &fields,
        QuerySpelling(&program.emit.subject.0),
        builder.span,
    )
    .map_err(|error| locate_error(program.emit_origin.clone(), error))?;
    let mut key = Vec::new();
    for name in &program.emit.key {
        key.push(
            position(&fields, QuerySpelling(&name.0), builder.span)
                .map_err(|error| locate_error(program.emit_origin.clone(), error))?,
        );
    }
    Ok(ryo_values::domain::query_plans::types::Plan {
        observation_inputs,
        observations,
        parameters: program
            .parameters
            .iter()
            .zip(&builder.kinds)
            .map(
                |(parameter, kind)| ryo_values::domain::query_bindings::types::PlanParameter {
                    name: parts::ParameterName(parameter.parameter.0.clone()),
                    kind: *kind,
                },
            )
            .collect(),
        frontier: frontier_of(&steps),
        steps,
        comparison_origins: builder.comparison_origins,
        emit: ryo_values::domain::query_plans::types::Emit {
            step: emitted,
            subject,
            key,
            headings: fields
                .into_iter()
                .map(|field| parts::Heading(field.name.0))
                .collect(),
        },
    })
}
