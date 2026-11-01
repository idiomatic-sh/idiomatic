//! The `assembling` service: free functions that turn a program into the plan the engine walks,
//! with no IO and no ports. This is where names are resolved once: a quoted relation becomes a
//! `Relation`, a column becomes its position, a step becomes its number, a test becomes an enum.
//! Nothing after this works on names.

use ryo_language::domain::relations::types::{CellKind, Relation};
use ryo_language::domain::terms::types::Vocabulary;

use crate::domain::calling::service::lower_call;
use crate::domain::fusing::service::fuse_selects;
use crate::domain::inlining::service::{cut_relations, inline_views, reduced_frontier};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{Builder, HostedStep, PlannedColumn, StepLabel, Uniqueness};
use crate::domain::planning::service::{frontier_of, pushed};
use crate::domain::pruning::service::without_unused;
use crate::domain::pushing::service::{push_below_captures, push_lookups};
use crate::domain::resolving::service::{position, read_source, step_named, step_of};
use crate::domain::rewriting::types::Planned;
use crate::domain::selecting::service::lower_select;
use crate::domain::syntax::constants::parameter_kind_named;
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
pub(crate) fn lower(
    program: &syntax::Program,
    catalog: &[Relation],
    hosted: &[hosted::HostedRelation],
) -> Result<plans::Plan, LoweringError> {
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
        hosted_steps: Vec::new(),
        parameters: &program.parameters,
        kinds,
        steps: Vec::new(),
        lowered: Vec::new(),
        span: program.emit.span,
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
        for contract in program
            .fragment_contracts
            .iter()
            .filter(|contract| Some(contract.first) == step.name.1)
        {
            check_fragment_inputs(&builder, contract)?;
        }
        builder.span = step.span;
        let lowered = match &step.expression {
            syntax::Expression::Select(select) => lower_select(&mut builder, &step.name, select),
            syntax::Expression::Call(call) => lower_call(&mut builder, &step.name, call),
        };
        lowered.map_err(|error| locate_step_error(program, step.name.1, error))?;
        for contract in program
            .fragment_contracts
            .iter()
            .filter(|contract| Some(contract.result) == step.name.1)
        {
            check_fragment_output(&builder, contract)?;
        }
    }
    builder.span = program.emit.span;
    let named = step_named(&builder, &program.emit.step)
        .map_err(|error| locate_error(program.emit_origin.clone(), error))?;
    let fields = step_of(&builder.lowered, named).fields.clone();
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

fn locate_step_error(
    program: &syntax::Program,
    identity: Option<syntax::StepIdentity>,
    error: LoweringError,
) -> LoweringError {
    locate_error(
        identity.and_then(|identity| program.step_origins.get(&identity).cloned()),
        error,
    )
}

fn locate_error(
    origin: Option<ryo_values::domain::program::types::QueryOrigin>,
    error: LoweringError,
) -> LoweringError {
    match origin {
        Some(origin) => LoweringError::Located {
            origin,
            error: Box::new(error),
        },
        None => error,
    }
}

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
            _ => {
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

fn check_fragment_output(
    builder: &Builder<'_>,
    contract: &syntax::FragmentContract,
) -> Result<(), LoweringError> {
    let fields = builder
        .lowered
        .iter()
        .find_map(|step| match &step.name {
            StepLabel::Named(name) if name.1 == Some(contract.result) => Some(&step.fields),
            StepLabel::Named(_) | StepLabel::Unnamed => None,
        })
        .ok_or_else(|| LoweringError::FragmentContract {
            message: "the returned step was not produced during lowering".to_owned(),
            origin: contract.origin.clone(),
        })?;
    check_relation_shape(fields, &contract.output, contract.origin.clone())
}

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
    for (actual, expected) in actual.iter().zip(&expected.columns) {
        let expected_kind = match expected.kind.0.as_ref() {
            "node" => CellKind::Node,
            "symbol" | "text" => CellKind::Symbol,
            "file" => CellKind::File,
            "span" => CellKind::Span,
            "number" | "integer" => CellKind::Number,
            "flag" | "boolean" => CellKind::Flag,
            other => {
                return Err(LoweringError::FragmentContract {
                    message: format!("unknown relation cell type `{other}`"),
                    origin,
                });
            }
        };
        if actual.name != expected.name || actual.kind != expected_kind {
            return Err(LoweringError::FragmentContract {
                message: format!(
                    "expected column `{}` of type `{}`, found `{}` of type `{:?}`",
                    expected.name.0, expected.kind.0, actual.name.0, actual.kind
                ),
                origin,
            });
        }
    }
    Ok(())
}
