//! The shared walk's census as trace events.
use crate::domain::executable_program::types::ExecutableProgram;
use crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk;
use ryo_values::domain::rule_requests::types::RuleSpec;

/// Emits the census of the compiled walk when its diagnostic target is on.
pub(in crate::application::file_dag::program::preparation) fn trace(
    rules: &[RuleSpec],
    walk: &PreparedWalk,
    program: &ExecutableProgram,
) {
    if !tracing::enabled!(target: crate::application::file_dag::program::constants::WALK_TARGET, tracing::Level::TRACE)
    {
        return;
    }
    let census = walk.census();
    tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET, event = crate::application::file_dag::program::constants::WALK_CENSUS,
        rules = ?rules.iter().map(|rule| rule.id.0.as_ref()).collect::<Vec<_>>(),
        instructions = census.instructions.0,
        physical_instructions = program.regions().iter().map(|region| region.instructions.len()).sum::<usize>(),
        shared = census.shared.0,
        classes = census.classes.len(), representation = census.representation.0,
        dependent = census.dependent.0, observation = census.observation.0,
        row = census.row.0, derived = census.derived.0, request = census.request.0,
        continuation = census.continuation.0);
    for class in census.classes {
        tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET, event = crate::application::file_dag::program::constants::WALK_CLASS,
            class = class.0, relation = ?walk.classes()[class.0].source,
            heads = ?program.regions().iter().flat_map(|region| region.instructions.iter()).filter(|instruction| walk.class_for(instruction) == Some(class)).map(|instruction| instruction.logical.0).collect::<Vec<_>>(),
            rules = ?program.regions().iter().flat_map(|region| region.instructions.iter()).filter(|instruction| walk.class_for(instruction) == Some(class)).map(|instruction| instruction.producer.0).collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>());
    }
}
