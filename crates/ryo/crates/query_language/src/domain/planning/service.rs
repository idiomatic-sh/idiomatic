//! The `planning` service: a step is added to the plan under construction, and where it runs is
//! decided: per file, or after the files.

use ryo_relations::domain::relations::service::provenance_of;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Provenance;
use ryo_values::domain::lineage::service::anchor_of;
use ryo_values::domain::lineage::types::ColumnAnchor;

use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{
    Builder, FileOrigin, LoweredStep, PlannedColumn, ScopedInput, StepLabel,
};
use crate::domain::resolving::service::{column_of, relation_fields, step_of};
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::program::types as syntax;
use ryo_values::domain::query_tokens::types as tokens;

/// The step the next one pushed onto `items` will be.
pub(crate) fn next_step<T>(items: &[T]) -> parts::StepId {
    parts::StepId(u32::try_from(items.len()).unwrap_or(u32::MAX))
}

/// What a step reads, in the order its operator reads its inputs.
pub(crate) fn inputs_of(operation: &plans::Operation) -> Vec<parts::Input> {
    match operation {
        plans::Operation::SpanBetween { from, .. }
        | plans::Operation::SpanBounds { from, .. }
        | plans::Operation::SourceMatches { from, .. }
        | plans::Operation::RenderSyntax { from, .. }
        | plans::Operation::FitsRendered { from, .. }
        | plans::Operation::Replace { from, .. }
        | plans::Operation::Select { from, .. }
        | plans::Operation::Project { from, .. }
        | plans::Operation::Extend { from, .. }
        | plans::Operation::GroupCount { from, .. }
        | plans::Operation::GroupDigest { from, .. }
        | plans::Operation::Capture { from, .. }
        | plans::Operation::Rank { from, .. }
        | plans::Operation::Words { from, .. } => vec![*from],
        plans::Operation::OrderedMatch { left, right, .. }
        | plans::Operation::Join { left, right, .. }
        | plans::Operation::Union { left, right }
        | plans::Operation::Intersection { left, right }
        | plans::Operation::Difference { left, right }
        | plans::Operation::Antijoin { left, right, .. } => vec![*left, *right],
        plans::Operation::OrderedDfs { from, edges, .. } => vec![*from, *edges],
        plans::Operation::Axis {
            from,
            edges,
            domain,
            ..
        } => vec![*from, *edges, *domain],
        plans::Operation::Closure { edges, .. } | plans::Operation::Cycles { edges, .. } => {
            vec![*edges]
        }
        plans::Operation::Rows { .. } | plans::Operation::Hosted { .. } => Vec::new(),
    }
}

/// Whether a row holding these fields is kept in its file: one of them is held in its file by `held`.
fn keyed(fields: &[PlannedColumn], held: fn(CellKind) -> FileOrigin) -> FileOrigin {
    if fields
        .iter()
        .any(|field| held(field.kind) == FileOrigin::Held)
    {
        FileOrigin::Held
    } else {
        FileOrigin::Loose
    }
}

/// Whether a value of this kind keeps the rows a step combines on it in one file: a value that states its row's file by the one
/// lineage rule (`anchor_of`). A span states no file.
fn held_in_file(kind: CellKind) -> FileOrigin {
    if anchor_of(kind) == ColumnAnchor::InFile {
        FileOrigin::Held
    } else {
        FileOrigin::Loose
    }
}

/// Whether a value of this kind keeps the rows a step combines on it in one file when the step runs inside one file: a value held
/// in its file, or a span, a place in the file the step runs in.
fn held_inside_one_file(kind: CellKind) -> FileOrigin {
    if kind == CellKind::Span {
        FileOrigin::Held
    } else {
        held_in_file(kind)
    }
}

/// Whether one of these columns of the fields is held in its file by `held`.
fn keyed_on(
    fields: &[PlannedColumn],
    columns: &[parts::ColumnIndex],
    held: fn(CellKind) -> FileOrigin,
) -> FileOrigin {
    if columns
        .iter()
        .any(|index| held(column_of(fields, *index).kind) == FileOrigin::Held)
    {
        FileOrigin::Held
    } else {
        FileOrigin::Loose
    }
}

/// Where a step runs. A step that reads a cross-file step's rows is cross-file. Otherwise a step
/// that combines rows is per-file when what it combines them on says which file they are of, so rows
/// of two files can never meet in it; a step that does not combine rows is always per-file, except
/// a projection that drops the file from every column, whose repeated rows may come from two files.
pub(crate) fn scope_of(
    operation: &plans::Operation,
    inputs: &[ScopedInput],
    out: &[PlannedColumn],
) -> ryo_values::domain::plan_parts::types::StepScope {
    if inputs
        .iter()
        .any(|input| input.scope == ryo_values::domain::plan_parts::types::StepScope::CrossFile)
    {
        return ryo_values::domain::plan_parts::types::StepScope::CrossFile;
    }
    let first = inputs
        .first()
        .map_or(&[][..], |input| input.fields.as_slice());
    let second = inputs
        .get(1)
        .map_or(&[][..], |input| input.fields.as_slice());
    // Every input is per-file, so a step keyed on spans alone runs inside one file: its spans are places in that file.
    if placed(operation, first, second, out, held_inside_one_file) == FileOrigin::Held {
        ryo_values::domain::plan_parts::types::StepScope::PerFile
    } else {
        ryo_values::domain::plan_parts::types::StepScope::CrossFile
    }
}

/// Whether a step whose inputs are all per-file keeps the rows it combines in one file, so it may run in each file, when the values
/// `held` says are held in their file are: what it combines rows on says which file they are of. A step that does not combine rows
/// is always per-file, except a projection that drops the file from every column, whose repeated rows may come from two files.
fn placed(
    operation: &plans::Operation,
    first: &[PlannedColumn],
    second: &[PlannedColumn],
    out: &[PlannedColumn],
    held: fn(CellKind) -> FileOrigin,
) -> FileOrigin {
    let per_file = match operation {
        plans::Operation::SpanBetween { .. }
        | plans::Operation::SpanBounds { .. }
        | plans::Operation::SourceMatches { .. }
        | plans::Operation::RenderSyntax { .. }
        | plans::Operation::FitsRendered { .. }
        | plans::Operation::Replace { .. }
        | plans::Operation::Select { .. }
        | plans::Operation::Extend { .. }
        | plans::Operation::Capture { .. }
        | plans::Operation::Rank { .. }
        | plans::Operation::Words { .. } => true,
        plans::Operation::Project { .. } => {
            keyed(out, held) == FileOrigin::Held || keyed(first, held) == FileOrigin::Loose
        }
        plans::Operation::OrderedMatch {
            left_on, right_on, ..
        }
        | plans::Operation::Join {
            left_on, right_on, ..
        }
        | plans::Operation::Antijoin {
            left_on, right_on, ..
        } => {
            keyed_on(first, left_on, held) == FileOrigin::Held
                && keyed_on(second, right_on, held) == FileOrigin::Held
        }
        plans::Operation::Union { .. }
        | plans::Operation::Intersection { .. }
        | plans::Operation::Difference { .. } => {
            keyed(first, held) == FileOrigin::Held && keyed(second, held) == FileOrigin::Held
        }
        plans::Operation::GroupCount { keys, .. } | plans::Operation::GroupDigest { keys, .. } => {
            keyed_on(first, keys, held) == FileOrigin::Held
        }
        plans::Operation::OrderedDfs {
            root,
            source,
            root_keys,
            edge_keys,
            ..
        } => {
            (keyed_on(first, &[*root], held) == FileOrigin::Held
                || keyed_on(first, root_keys, held) == FileOrigin::Held)
                && (keyed_on(second, &[*source], held) == FileOrigin::Held
                    || keyed_on(second, edge_keys, held) == FileOrigin::Held)
        }
        plans::Operation::Axis { id, .. } => keyed_on(first, &[*id], held) == FileOrigin::Held,
        plans::Operation::Closure { source, keys, .. } => {
            keyed_on(first, &[*source], held) == FileOrigin::Held
                || keyed_on(first, keys, held) == FileOrigin::Held
        }
        plans::Operation::Cycles { source, .. } => {
            keyed_on(first, &[*source], held) == FileOrigin::Held
        }
        // The host's rows are the same for every file, so they are made once, after the files.
        plans::Operation::Rows { .. } | plans::Operation::Hosted { .. } => false,
    };
    if per_file {
        FileOrigin::Held
    } else {
        FileOrigin::Loose
    }
}

/// Refuses a step from `from` on of the plan under construction that compares rows on spans alone, with no file or node key,
/// over rows made after the files: a span states no file, so equal spans of two files would meet, merge or cancel. Such a step is
/// kept only when every input is per-file, so it runs inside one file. This covers a join, an antijoin, an ordered match, a group
/// count or digest, a closure, an ordered depth-first visit, the cycles of a graph, a union, an intersection, a difference, and
/// the distinct rows of a projection whose rows hold a span and no file or node.
pub(crate) fn refuse_spans_alone(builder: &Builder<'_>, from: usize) -> Result<(), LoweringError> {
    for (step, lowered) in builder.steps.iter().zip(&builder.lowered).skip(from) {
        let inputs: Vec<ScopedInput> = inputs_of(&step.operation)
            .into_iter()
            .map(|input| scoped(builder, input))
            .collect();
        if !inputs
            .iter()
            .any(|input| input.scope == ryo_values::domain::plan_parts::types::StepScope::CrossFile)
        {
            continue;
        }
        let first = inputs
            .first()
            .map_or(&[][..], |input| input.fields.as_slice());
        let second = inputs
            .get(1)
            .map_or(&[][..], |input| input.fields.as_slice());
        let on_spans_alone = |held: fn(CellKind) -> FileOrigin| match &step.operation {
            // The distinct rows compare every column they keep, whatever their input held.
            plans::Operation::Project {
                distinct: parts::RepeatedRows::Dropped,
                ..
            } => keyed(&lowered.fields, held),
            plans::Operation::Join { .. }
            | plans::Operation::Antijoin { .. }
            | plans::Operation::OrderedMatch { .. }
            | plans::Operation::GroupCount { .. }
            | plans::Operation::GroupDigest { .. }
            | plans::Operation::Closure { .. }
            | plans::Operation::OrderedDfs { .. }
            | plans::Operation::Cycles { .. }
            | plans::Operation::Union { .. }
            | plans::Operation::Intersection { .. }
            | plans::Operation::Difference { .. } => {
                placed(&step.operation, first, second, &lowered.fields, held)
            }
            plans::Operation::Project {
                distinct: parts::RepeatedRows::Kept,
                ..
            }
            | plans::Operation::SpanBetween { .. }
            | plans::Operation::SpanBounds { .. }
            | plans::Operation::SourceMatches { .. }
            | plans::Operation::RenderSyntax { .. }
            | plans::Operation::FitsRendered { .. }
            | plans::Operation::Replace { .. }
            | plans::Operation::Select { .. }
            | plans::Operation::Extend { .. }
            | plans::Operation::Capture { .. }
            | plans::Operation::Rank { .. }
            | plans::Operation::Words { .. }
            | plans::Operation::Axis { .. }
            | plans::Operation::Rows { .. }
            | plans::Operation::Hosted { .. } => FileOrigin::Loose,
        };
        if on_spans_alone(held_in_file) == FileOrigin::Loose
            && on_spans_alone(held_inside_one_file) == FileOrigin::Held
        {
            return Err(LoweringError::CrossFileSpanKey { span: builder.span });
        }
    }
    Ok(())
}

/// Refuses a closure or cycles step from `from` on of the plan under construction whose source and destination are of two kinds:
/// a walk goes from a source to a destination that is the next source, so both are of one kind, and where the step runs is decided
/// on its source alone.
pub(crate) fn refuse_mixed_vertices(
    builder: &Builder<'_>,
    from: usize,
) -> Result<(), LoweringError> {
    for step in builder.steps.iter().skip(from) {
        let (plans::Operation::Closure {
            edges,
            source,
            destination,
            ..
        }
        | plans::Operation::Cycles {
            edges,
            source,
            destination,
        }) = &step.operation
        else {
            continue;
        };
        let fields = scoped(builder, *edges).fields;
        if column_of(&fields, *source).kind != column_of(&fields, *destination).kind {
            return Err(LoweringError::MixedVertices { span: builder.span });
        }
    }
    Ok(())
}

/// What the per-file steps hand over: every relation and every per-file step that a cross-file step reads.
pub(crate) fn frontier_of(steps: &[plans::PlanStep]) -> Vec<parts::Input> {
    let mut frontier = Vec::new();
    for step in steps
        .iter()
        .filter(|step| step.scope == ryo_values::domain::plan_parts::types::StepScope::CrossFile)
    {
        for input in inputs_of(&step.operation) {
            let handed_over = match input {
                parts::Input::Relation(_) | parts::Input::BoundObservation(_) => true,
                parts::Input::Step(id) => {
                    step_of(steps, id).scope
                        == ryo_values::domain::plan_parts::types::StepScope::PerFile
                }
            };
            if handed_over && !frontier.contains(&input) {
                frontier.push(input);
            }
        }
    }
    frontier
}

/// Where the rows of `input` of a planned step are made: a relation's in each file, unless the project's manifests make it; a step's
/// where it runs; none for a bound input, whose scope the plan does not hold.
pub(crate) fn planned_scope(
    steps: &[plans::PlanStep],
    input: parts::Input,
) -> Option<ryo_values::domain::plan_parts::types::StepScope> {
    match input {
        parts::Input::Relation(relation) => {
            Some(if provenance_of(relation) == Provenance::Manifests {
                ryo_values::domain::plan_parts::types::StepScope::CrossFile
            } else {
                ryo_values::domain::plan_parts::types::StepScope::PerFile
            })
        }
        parts::Input::BoundObservation(_) => None,
        parts::Input::Step(id) => {
            ryo_values::domain::plan_parts::service::step_in(steps, id).map(|step| step.scope)
        }
    }
}

/// Where an input's rows are produced, and the columns they have: a relation's rows are one
/// file's, a step's are wherever the step runs.
pub(crate) fn scoped(builder: &Builder<'_>, input: parts::Input) -> ScopedInput {
    match input {
        parts::Input::Relation(relation) => ScopedInput {
            scope: if provenance_of(relation) == Provenance::Manifests {
                ryo_values::domain::plan_parts::types::StepScope::CrossFile
            } else {
                ryo_values::domain::plan_parts::types::StepScope::PerFile
            },
            fields: relation_fields(relation),
        },
        parts::Input::BoundObservation(id) => {
            let bound = &builder.observation_inputs
                [usize::try_from(id.0).expect(super::constants::VALID_BINDING_INDEX)];
            ScopedInput {
                scope: bound.scope,
                fields: bound
                    .columns
                    .iter()
                    .map(|column| PlannedColumn {
                        name: ryo_values::domain::descriptions::types::ColumnName(
                            column.name.0.clone(),
                        ),
                        kind: column.kind,
                        unique: crate::domain::lowering::types::Uniqueness::Shared,
                        words: ryo_relations::domain::terms::types::Vocabulary::Open,
                    })
                    .collect(),
            }
        }
        parts::Input::Step(id) => {
            let lowered = step_of(&builder.lowered, id);
            ScopedInput {
                scope: lowered.scope,
                fields: lowered.fields.clone(),
            }
        }
    }
}

/// Adds a step to the plan and keeps its columns for the steps below: for a step no later step of the statement reads by number.
pub(crate) fn push(
    builder: &mut Builder<'_>,
    name: Option<&tokens::StepName>,
    operation: plans::Operation,
    fields: Vec<PlannedColumn>,
) {
    let inputs: Vec<ScopedInput> = inputs_of(&operation)
        .into_iter()
        .map(|input| scoped(builder, input))
        .collect();
    let scope = scope_of(&operation, &inputs, &fields);
    builder.steps.push(plans::PlanStep { operation, scope });
    builder.lowered.push(LoweredStep {
        name: name.map_or(StepLabel::Unnamed, |name| StepLabel::Named(name.clone())),
        fields,
        scope,
    });
}

/// Adds a step to the plan, as [`push`] does, and answers its number: for a step the next steps of the statement read.
pub(crate) fn pushed(
    builder: &mut Builder<'_>,
    name: Option<&tokens::StepName>,
    operation: plans::Operation,
    fields: Vec<PlannedColumn>,
) -> parts::StepId {
    let id = next_step(&builder.steps);
    push(builder, name, operation, fields);
    id
}
