//! The `contract` service: pure functions over the domain's types, with no IO and no ports.
//! Logical mutable-set requirements are separate from acquisition and storage.
use super::types::{
    DistinctWorkspace, PrimitiveWorkspace, WorkspaceLifetime as Lifetime, WorkspaceProgram,
};
use crate::domain::{operating::errors::OperatingError, operation_schema::types::InputShape};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::{
    plan_parts::types::RepeatedRows,
    plans::types::{Operation, OperationKind},
};
pub(crate) fn prepare(
    operation: &Operation,
    inputs: &[InputShape<'_>],
    output: &Result<Vec<CellKind>, OperatingError>,
) -> WorkspaceProgram {
    use OperationKind::*;
    let first = |lifetime, kinds| WorkspaceProgram::FirstOccurrences { lifetime, kinds };
    match ryo_values::domain::plans::service::kind_of(operation) {
        Project => match operation {
            Operation::Project {
                distinct: RepeatedRows::Dropped,
                ..
            } => first(Lifetime::Invocation, output.clone()),
            _ => WorkspaceProgram::None,
        },
        Union | Intersection | Difference => first(Lifetime::Invocation, output.clone()),
        Join => match operation {
            Operation::Join {
                reach: Some(reach), ..
            } => first(
                Lifetime::PrimaryRow,
                crate::domain::operation_schema::service::reach_kinds(
                    inputs.get(1).map_or(&[], |input| input.kinds),
                    reach,
                ),
            ),
            _ => WorkspaceProgram::None,
        },
        Select | Antijoin | Extend | Replace | Capture | Rank | Words | OrderedMatch
        | GroupCount | GroupDigest | Closure | Cycles | OrderedDfs | Axis | Rows | Hosted
        | SourceMatches | RenderSyntax | FitsRendered | SpanBounds | SpanBetween => {
            WorkspaceProgram::None
        }
    }
}
/// Reset logical contents while retaining allocation. A schema fault stays deferred
/// until the primitive actually requests first-occurrence semantics.
pub(crate) fn begin(
    program: &WorkspaceProgram,
    lifetime: Lifetime,
    workspace: &mut PrimitiveWorkspace,
) {
    let WorkspaceProgram::FirstOccurrences {
        lifetime: declared,
        kinds,
    } = program
    else {
        return;
    };
    if *declared != lifetime {
        return;
    }
    let Some(state) = workspace.distinct.as_mut() else {
        return;
    };
    state.seen.clear();
    match kinds {
        Ok(kinds) => crate::domain::table_model::service::reset(&mut state.rows, kinds),
        Err(_) => crate::domain::table_model::service::reset(&mut state.rows, &[]),
    }
    state.schema = kinds.as_ref().map(|_| ()).map_err(Clone::clone);
}
impl Default for DistinctWorkspace {
    fn default() -> Self {
        Self {
            rows: Default::default(),
            seen: Default::default(),
            schema: Err(OperatingError::NotRun),
        }
    }
}
