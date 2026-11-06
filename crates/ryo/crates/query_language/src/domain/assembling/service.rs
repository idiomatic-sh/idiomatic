//! The `assembling` service: free functions that turn a program into the plan the engine walks,
//! with no IO and no ports. This is where names are resolved once: a quoted relation becomes a
//! `Relation`, a column becomes its position, a step becomes its number, a test becomes an enum.
//! Nothing after this works on names.

use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::terms::types::Vocabulary;

use crate::domain::calling::service::lower_call;
use crate::domain::fusing::service::{fuse_keyed_reaches, fuse_selects};
use crate::domain::inlining::service::{
    cut_relations, cut_relations_roots, inline_views, reduced_frontier, reduced_frontier_roots,
};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{Builder, HostedStep, PlannedColumn, StepLabel, Uniqueness};
use crate::domain::planning::service::{frontier_of, pushed};
use crate::domain::pruning::service::{without_unused, without_unused_roots};
use crate::domain::pushing::service::{
    push_below_captures, push_below_captures_roots, push_lookups, push_lookups_roots,
};
use crate::domain::resolving::service::{position, read_source, step_named, step_of};
use crate::domain::rewriting::types::Planned;
use crate::domain::selecting::service::lower_select;
use crate::domain::syntax::constants::{parameter_kind_named, relation_cell_kind_named};
use crate::domain::walking::service::{fuse_descents, fuse_neighbours};
use ryo_values::domain::hosted::types as hosted;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::program::types::{self as syntax, QuerySpelling};

/// Lowers a program that validation accepted.
///
/// # Errors
///
/// The first name that resolves to nothing, or comparison of unlike types, with its place.
#[expect(
    clippy::too_many_lines,
    reason = "lowering keeps the ordered whole-plan optimization pipeline explicit"
)]
#[cfg(test)]
pub(crate) fn lower(
    program: &syntax::WrittenProgram,
    catalog: &[Relation],
    hosted: &[hosted::HostedRelation],
) -> Result<plans::Plan, LoweringError> {
    lower_with_observations(program, catalog, hosted, &[])
}

/// Resolve every declared producer source through the same typed lowering pass.
pub(crate) fn lower_with_observations(
    program: &syntax::WrittenProgram,
    catalog: &[Relation],
    hosted: &[hosted::HostedRelation],
    observations: &[ryo_values::domain::observations::types::BoundObservationInput],
) -> Result<plans::Plan, LoweringError> {
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
                name: syntax::ColumnName(column.name.0.clone()),
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
    for step in &program.steps {
        for contract in program.fragment_contracts.iter().filter(|contract| {
            Some(contract.first) == ryo_values::domain::program::service::identity_of(step.name.1)
        }) {
            check_fragment_inputs(&builder, contract)?;
        }
        builder.span = step.span;
        builder.origin = step
            .name
            .1
            .and_then(|identity| program.step_origins.get(&identity).cloned());
        let lowered = match &step.expression {
            syntax::Expression::Select(select) => lower_select(&mut builder, &step.name, select),
            syntax::Expression::Call(call) => lower_call(&mut builder, &step.name, call),
        };
        lowered.map_err(|error| {
            locate_step_error(
                program,
                ryo_values::domain::program::service::identity_of(step.name.1),
                error,
            )
        })?;
        for contract in program.fragment_contracts.iter().filter(|contract| {
            Some(contract.result) == ryo_values::domain::program::service::identity_of(step.name.1)
        }) {
            check_fragment_output(&builder, contract)?;
        }
    }
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
        let Planned { steps, emitted } = reduced_frontier(steps, emitted);
        let Planned { steps, emitted } = cut_relations(steps, emitted);
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
        let steps = reduced_frontier_roots(steps, &mut roots);
        let steps = cut_relations_roots(steps, &mut roots);
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
                    scope: steps
                        [usize::try_from(step.0).expect(super::constants::RETAINED_ROOT_INDEX)]
                    .scope,
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
    Ok(plans::Plan {
        observation_inputs,
        observations,
        parameters: program
            .parameters
            .iter()
            .zip(&builder.kinds)
            .map(|(parameter, kind)| plans::PlanParameter {
                name: parts::ParameterName(parameter.name.0.clone()),
                kind: *kind,
            })
            .collect(),
        frontier: frontier_of(&steps),
        steps,
        comparison_origins: builder.comparison_origins,
        emit: plans::Emit {
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

/// Attaches the expanded fragment origin to a lowering error for one root step.
fn locate_step_error(
    program: &syntax::WrittenProgram,
    identity: Option<syntax::StepIdentity>,
    error: LoweringError,
) -> LoweringError {
    locate_error(
        identity.and_then(|identity| program.step_origins.get(&identity).cloned()),
        error,
    )
}

/// Attaches an optional source origin to a lowering error.
fn locate_error(
    origin: ryo_values::domain::program::types::EmitOrigin,
    error: LoweringError,
) -> LoweringError {
    match origin {
        ryo_values::domain::program::types::EmitOrigin::Located(origin) => LoweringError::Located {
            origin,
            error: Box::new(error),
        },
        ryo_values::domain::program::types::EmitOrigin::Unlocated => error,
    }
}

/// Checks that each linked fragment input has the declared relation shape.
fn check_fragment_inputs(
    builder: &Builder<'_>,
    contract: &syntax::FragmentContract,
) -> Result<(), LoweringError> {
    for input in &contract.inputs {
        let mut origin = contract.origin.clone();
        origin.span = input.span;
        let reference = match &input.value {
            syntax::ArgumentValue::Text(text) => {
                syntax::SourceReference::Relation(syntax::WrittenRelation(text.0.clone()))
            }
            syntax::ArgumentValue::Step(step) => syntax::SourceReference::Step(step.clone()),
            syntax::ArgumentValue::Word(_) => {
                return Err(LoweringError::FragmentContract {
                    message: format!(
                        "input `{}` did not link to a visible relation or step",
                        input.name.0
                    ),
                    origin,
                });
            }
            syntax::ArgumentValue::Number(_)
            | syntax::ArgumentValue::Parameter(_)
            | syntax::ArgumentValue::List(_)
            | syntax::ArgumentValue::Map(_) => {
                return Err(LoweringError::FragmentContract {
                    message: format!("input `{}` is not a relation source", input.name.0),
                    origin,
                });
            }
        };
        let source = syntax::QuerySource {
            reference,
            alias: syntax::Alias::Unwritten,
            span: input.span,
        };
        let actual =
            read_source(builder, &source).map_err(|error| LoweringError::FragmentContract {
                message: error.to_string(),
                origin: origin.clone(),
            })?;
        check_relation_shape(&actual.fields, &input.relation, origin)?;
    }
    Ok(())
}

/// Checks the fragment result against its declared output relation.
fn check_fragment_output(
    builder: &Builder<'_>,
    contract: &syntax::FragmentContract,
) -> Result<(), LoweringError> {
    let fields = builder
        .lowered
        .iter()
        .find_map(|step| match &step.name {
            StepLabel::Named(name)
                if ryo_values::domain::program::service::identity_of(name.1)
                    == Some(contract.result) =>
            {
                Some(&step.fields)
            }
            StepLabel::Named(_) | StepLabel::Unnamed => None,
        })
        .ok_or_else(|| LoweringError::FragmentContract {
            message: super::constants::MISSING_RETURNED_STEP.to_owned(),
            origin: contract.origin.clone(),
        })?;
    check_relation_shape(fields, &contract.output, contract.origin.clone())
}

/// Checks a lowered set of columns against a source relation signature.
fn check_relation_shape(
    actual: &[PlannedColumn],
    expected: &syntax::RelationSignature,
    origin: ryo_values::domain::program::types::QueryOrigin,
) -> Result<(), LoweringError> {
    if actual.len() != expected.columns.len() {
        return Err(LoweringError::FragmentContract {
            message: format!(
                "expected {} columns, found {}",
                expected.columns.len(),
                actual.len()
            ),
            origin,
        });
    }
    for (actual_column, expected_column) in actual.iter().zip(&expected.columns) {
        let Some(expected_kind) = relation_cell_kind_named(&expected_column.kind.0) else {
            return Err(LoweringError::FragmentContract {
                message: format!("unknown relation cell type `{}`", expected_column.kind.0),
                origin,
            });
        };
        if actual_column.name != expected_column.name || actual_column.kind != expected_kind {
            return Err(LoweringError::FragmentContract {
                message: format!(
                    "expected column `{}` of type `{}`, found `{}` of type `{:?}`",
                    expected_column.name.0,
                    expected_column.kind.0,
                    actual_column.name.0,
                    actual_column.kind
                ),
                origin,
            });
        }
    }
    Ok(())
}
