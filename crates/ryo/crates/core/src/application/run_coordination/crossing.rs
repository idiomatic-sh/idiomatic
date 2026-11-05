//! Project source checks and invocation context; execution belongs to the common program.
use super::{
    cross_execution::{ProgramEvaluated, evaluate_cross_roots},
    settings::CrossingInput,
};
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecution;
use ryo_execution::inbound::use_cases::execution::types::ProjectScope;
use ryo_execution::inbound::use_cases::execution::types::{BoundInput, PreparedFrontiers};
use ryo_language::domain::symbols::types::Symbols;
use {
    crate::domain::answering::types::Todo, crate::domain::crossing::service::partials_for,
    crate::domain::crossing::service::partials_for_reads, crate::domain::rules::service::failed,
    ryo_execution::inbound::use_cases::metadata::interface::Partials,
    ryo_values::domain::rule_requests::errors::RulesError,
};

pub(crate) fn cross_rule_with_inputs(
    crossing: &CrossingInput<'_>,
    todo: &Todo,
    frontiers: PreparedFrontiers,
    bound_inputs: &[BoundInput],
    cross: &mut Symbols,
    scope: &ProjectScope,
) -> Result<ProgramEvaluated, RulesError> {
    let refuse = |error: &dyn std::fmt::Display| failed(&todo.id, error);
    let evaluation = match crossing.answering {
        crate::domain::render::types::Answering::Findings => {
            partials_for(&crossing.domain, todo.rule)
        }
        crate::domain::render::types::Answering::Rows(_) => Partials::default(),
    };
    let reads = crossing
        .program
        .observation_reads(todo.rule)
        .map_err(|error| refuse(&error))?;
    let publication_partials = partials_for_reads(&crossing.domain, todo.rule, &reads);
    evaluate_cross_roots(
        crossing.services,
        todo.rule,
        scope,
        &evaluation,
        &publication_partials,
        frontiers,
        bound_inputs,
        cross,
        crossing.arenas,
    )
    .map_err(|error| refuse(&error))
}
