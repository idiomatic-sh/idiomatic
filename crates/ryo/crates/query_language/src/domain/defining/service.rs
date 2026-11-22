//! The `defining` service: free functions named by verb over one definition of a linked program lowered into the plan
//! under construction: the fragment contracts checked where an expansion starts and returns, the steps the definition
//! makes, the refusals of its steps, and the place of a refusal in the documents.

use crate::domain::calling::service::lower_call;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{Builder, PlannedColumn, StepLabel};
use crate::domain::refusing::service::{refuse_mixed_vertices, refuse_spans_alone};
use crate::domain::resolving::service::read_source;
use crate::domain::selecting::service::lower_select;
use crate::domain::syntax::constants::relation_cell_kind_named;
use ryo_values::domain::program::types as syntax;

/// Lowers one definition of a linked program into `builder`: the fragment inputs checked where the expansion starts,
/// the steps it makes, the refusals of its steps, the fragment output checked where the expansion returns.
pub(crate) fn lower_definition(
    builder: &mut Builder<'_>,
    program: &syntax::WrittenProgram,
    step: &syntax::LetStep,
) -> Result<(), LoweringError> {
    for contract in program.fragment_contracts.iter().filter(|contract| {
        Some(contract.first) == ryo_values::domain::program::service::identity_of(step.name.1)
    }) {
        check_fragment_inputs(builder, contract)?;
    }
    builder.span = step.span;
    builder.origin = ryo_values::domain::program::service::identity_of(step.name.1)
        .and_then(|identity| program.step_origins.get(&identity).cloned());
    let from = crate::domain::planning::service::next_step(&builder.steps);
    let lowered = match &step.expression {
        ryo_values::domain::query_syntax::types::Expression::Select(select) => {
            lower_select(builder, &step.name, select)
        }
        ryo_values::domain::query_syntax::types::Expression::Call(call) => {
            lower_call(builder, &step.name, call)
        }
    }
    .and_then(|()| refuse_spans_alone(builder, from))
    .and_then(|()| refuse_mixed_vertices(builder, from));
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
        check_fragment_output(builder, contract)?;
    }
    Ok(())
}

/// Attaches the expanded fragment origin to a lowering error for one root step.
pub(crate) fn locate_step_error(
    program: &syntax::WrittenProgram,
    identity: Option<ryo_values::domain::query_tokens::types::StepIdentity>,
    error: LoweringError,
) -> LoweringError {
    locate_error(
        match identity.and_then(|identity| program.step_origins.get(&identity).cloned()) {
            Some(origin) => ryo_values::domain::query_tokens::types::Attribution::Located(origin),
            None => ryo_values::domain::query_tokens::types::Attribution::Unlocated,
        },
        error,
    )
}

/// Attaches an optional source origin to a lowering error.
pub(crate) fn locate_error(
    origin: ryo_values::domain::query_tokens::types::Attribution,
    error: LoweringError,
) -> LoweringError {
    match origin {
        ryo_values::domain::query_tokens::types::Attribution::Located(origin) => {
            LoweringError::Located {
                origin,
                error: Box::new(error),
            }
        }
        ryo_values::domain::query_tokens::types::Attribution::Unlocated => error,
    }
}

/// Checks that each linked fragment input has the declared relation shape.
pub(crate) fn check_fragment_inputs(
    builder: &Builder<'_>,
    contract: &syntax::FragmentContract,
) -> Result<(), LoweringError> {
    for input in &contract.inputs {
        let mut origin = contract.origin.clone();
        origin.span = input.span;
        let reference = match &input.value {
            ryo_values::domain::query_syntax::types::ArgumentValue::Text(text) => {
                ryo_values::domain::query_syntax::types::SourceReference::Relation(
                    ryo_values::domain::query_tokens::types::WrittenRelation(text.0.clone()),
                )
            }
            ryo_values::domain::query_syntax::types::ArgumentValue::Step(step) => {
                ryo_values::domain::query_syntax::types::SourceReference::Step(step.clone())
            }
            ryo_values::domain::query_syntax::types::ArgumentValue::Word(_) => {
                return Err(LoweringError::FragmentContract {
                    message: format!(
                        "input `{}` did not link to a visible relation or step",
                        input.name.0
                    ),
                    origin,
                });
            }
            ryo_values::domain::query_syntax::types::ArgumentValue::Number(_)
            | ryo_values::domain::query_syntax::types::ArgumentValue::Parameter(_)
            | ryo_values::domain::query_syntax::types::ArgumentValue::List(_)
            | ryo_values::domain::query_syntax::types::ArgumentValue::Map(_) => {
                return Err(LoweringError::FragmentContract {
                    message: format!("input `{}` is not a relation source", input.name.0),
                    origin,
                });
            }
        };
        let source = ryo_values::domain::query_syntax::types::QuerySource {
            reference,
            alias: ryo_values::domain::query_syntax::types::Alias::Unwritten,
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
pub(crate) fn check_fragment_output(
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
    origin: ryo_values::domain::query_tokens::types::QueryOrigin,
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
