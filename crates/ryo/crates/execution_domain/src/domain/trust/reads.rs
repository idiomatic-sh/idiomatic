//! One part of the `trust` service: what each step of a plan reads of the relations, through the steps before it, with no IO and
//! no ports.

use super::types::{AbsenceInputs, ReadIndex, RelationRead, StepReads};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::service::{columns, kind_column};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, Operand};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Operation, Plan};

/// Adds the reads in `more` to `uses`, each once: a read the steps before already made is the same read, and the reads of a plan whose
/// steps meet again would otherwise double at every meeting.
pub(super) fn absorb(uses: &mut Vec<RelationRead>, more: impl IntoIterator<Item = RelationRead>) {
    for read in more {
        if !uses.contains(&read) {
            uses.push(read);
        }
    }
}

/// What the reads of the relations are, by step: for each step, the reads of the relations that feed it, through the steps before it,
/// and for each of its columns the read whose `kind` column it still is. A `select` on that column narrows the read: rows of a kind
/// the comparison lets through are all that get further, so rows of another kind that are missing cannot matter.
///
/// A step that reads a step not before it, a step refused before it, or a bound input the plan does not declare is an authoring
/// error a query-language provider can hand over: that step's reads are refused with the error its executable origin carries
/// ([`OperatingError::NotRun`] for a step that has not run), so the rule is refused on its own roots and no step is read as
/// reading nothing.
pub(super) fn uses_of(plan: &Plan) -> Vec<Result<StepReads, OperatingError>> {
    let mut steps: Vec<Result<StepReads, OperatingError>> = Vec::with_capacity(plan.steps.len());
    for step in &plan.steps {
        let reads = step_reads(plan, &steps, &step.operation);
        steps.push(reads);
    }
    steps
}

/// The reads of one step, from the reads of the steps before it.
fn step_reads(
    plan: &Plan,
    steps: &[Result<StepReads, OperatingError>],
    operation: &Operation,
) -> Result<StepReads, OperatingError> {
    let inputs = inputs_of(operation);
    let given = named(&inputs);
    // What each input is: its reads, and which read each of its columns is the kind of.
    let mut parts: Vec<StepReads> = Vec::new();
    for input in given {
        parts.push(match input {
            Input::Relation(relation) => relation_reads(*relation),
            // Named inputs have their own producer-readiness admission. They are not
            // catalog relations and cannot manufacture a physical relation completeness claim.
            Input::BoundObservation(id) => StepReads {
                uses: Vec::new(),
                provenance: vec![
                    None;
                    plan.observation_inputs
                        .get(id.0 as usize)
                        .ok_or(OperatingError::NotRun)?
                        .columns
                        .len()
                ],
            },
            Input::Step(id) => steps
                .get(id.0 as usize)
                .ok_or(OperatingError::NotRun)?
                .clone()?,
        });
    }
    // The kind provenance of the step's columns is the one column-origin propagation; a join's sides hand theirs on as
    // none, so a kind is only narrowed through the rows of one read.
    let origins: Vec<&[Option<ReadIndex>]> = parts.iter().map(|part| part.provenance.as_slice()).collect();
    let provenance = crate::domain::operation_schema::projection::output_origins(
        operation,
        &[],
        &origins,
        &mut |_, _| None,
    );
    // Each input's reads are its own, moved rather than copied: the first is what the step starts from.
    let mut parts = parts.into_iter();
    let StepReads {
        mut uses,
        provenance: first,
    } = parts.next().unwrap_or_default();
    match operation {
        Operation::Select { comparisons, .. } => {
            for comparison in comparisons {
                if let Operand::Column(column) = &comparison.left
                    && let Some(Some(read)) =
                        first.get(usize::try_from(column.0).unwrap_or(usize::MAX))
                    && let Some(each) = uses.get_mut(read.0)
                {
                    each.kinds.push(comparison.clone());
                }
            }
        }
        Operation::Antijoin { .. } | Operation::Difference { .. } => {
            // The left rows that remain, as they were; the right rows only decide which remain.
            let right = parts.next().unwrap_or_default();
            absorb(&mut uses, right.uses);
        }
        Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::GroupCount { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::OrderedDfs { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => {
            for part in parts {
                absorb(&mut uses, part.uses);
            }
        }
        Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::SpanBetween { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SourceMatches { .. }
        | Operation::SpanBounds { .. } => {}
    }
    Ok(StepReads { uses, provenance })
}

/// The place of the `kind` column of a relation, when it has one.
pub(super) fn kind_column_place(relation: Relation) -> Option<ColumnIndex> {
    let kind = kind_column(relation)?;
    columns(relation)
        .into_iter()
        .position(|column| column == kind)
        .map(|place| ColumnIndex(u32::try_from(place).unwrap_or(u32::MAX)))
}

/// Provenance of the initial columns read directly from a declared relation.
fn relation_reads(relation: Relation) -> StepReads {
    let kind = kind_column_place(relation);
    let width = columns(relation).len();
    StepReads {
        uses: vec![RelationRead {
            relation,
            kinds: Vec::new(),
        }],
        provenance: (0..width)
            .map(|place| {
                kind.filter(|column| usize::try_from(column.0) == Ok(place))
                    .map(|_| ReadIndex(0))
            })
            .collect(),
    }
}

/// For each step, the reads its operator trusts the absence of rows of: those of the inputs `masks` names, each read once.
pub(super) fn needs_with_masks(
    plan: &Plan,
    of: impl Fn(&Input) -> Vec<RelationRead>,
    masks: impl IntoIterator<Item = AbsenceInputs>,
) -> Vec<Vec<RelationRead>> {
    plan.steps
        .iter()
        .zip(masks)
        .map(|(step, AbsenceInputs(mask))| {
            let inputs = inputs_of(&step.operation);
            let mut result = Vec::new();
            for (position, input) in named(&inputs).iter().enumerate() {
                if mask & (1 << position) != 0 {
                    absorb(&mut result, of(input));
                }
            }
            result
        })
        .collect()
}
