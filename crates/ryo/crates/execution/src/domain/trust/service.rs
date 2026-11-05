//! The `trust` service: which reads of the relations a plan trusts the absence of rows of (an antijoin, a difference, a count), and
//! whether a relation read in part could change what such a read makes, with no IO and no ports.

use super::constants::READS_OF_EARLIER_STEP;
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::service::kind_column;
use ryo_relations::domain::relations::types::Relation;

use super::types::{Bearing, Blocking, Needs, Passage, ReadIndex, RelationRead, StepReads};
use crate::domain::coverage::constants::REPOSITORY;
use crate::domain::coverage::service::unknown_message;
use crate::domain::coverage::types::{
    CoverageScope, Lacking, LackingKind, Partial, Partials, RuleIndex, Unknown,
};
use crate::domain::rules::service::id_at;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Operation, Plan, StepScope};
use ryo_values::domain::runs::types::RuleId;

/// The place of the `kind` column of a relation, when it has one.
fn kind_place(relation: Relation) -> Option<ColumnIndex> {
    let kind = kind_column(relation)?;
    columns(relation)
        .into_iter()
        .position(|column| column == kind)
        .map(|place| ColumnIndex(u32::try_from(place).unwrap_or(u32::MAX)))
}

/// Whether a comparison lets a row of this kind through, as far as it can be told from the comparison alone.
fn admits(comparison: &Comparison, kind: &LackingKind) -> Passage {
    let kind: &str = &kind.0;
    let Operand::Literal(QueryLiteral::Text(text)) = &comparison.right else {
        return Passage::LetsThrough;
    };
    let lets = match comparison.test {
        Test::Equal => &*text.0 == kind,
        Test::NotEqual => &*text.0 != kind,
        Test::GlobAny | Test::NotGlobAny => {
            let inner = text
                .0
                .strip_prefix('{')
                .and_then(|rest| rest.strip_suffix('}'))
                .unwrap_or(&text.0);
            if inner.contains(['*', '?', '[', ']', '{', '}']) {
                return Passage::LetsThrough;
            }
            let listed = inner.split(',').any(|each| each == kind);
            listed == (comparison.test == Test::GlobAny)
        }
        Test::LessThan
        | Test::LessThanOrEqual
        | Test::GreaterThan
        | Test::GreaterThanOrEqual
        | Test::NaturalLessThan
        | Test::FloatTextGreaterThanOrEqual
        | Test::NumericGreaterThanOrEqual
        | Test::IntegerSuccessor
        | Test::In
        | Test::NotIn
        | Test::StartsWith
        | Test::Matches
        | Test::EndsWith => true,
    };
    if lets {
        Passage::LetsThrough
    } else {
        Passage::Blocks
    }
}

/// Adds the reads in `more` to `uses`, each once: a read the steps before already made is the same read, and the reads of a plan whose
/// steps meet again would otherwise double at every meeting.
fn absorb(uses: &mut Vec<RelationRead>, more: impl IntoIterator<Item = RelationRead>) {
    for read in more {
        if !uses.contains(&read) {
            uses.push(read);
        }
    }
}

/// What the reads of the relations are, by step: for each step, the reads of the relations that feed it, through the steps before it,
/// and for each of its columns the read whose `kind` column it still is. A `select` on that column narrows the read: rows of a kind
/// the comparison lets through are all that get further, so rows of another kind that are missing cannot matter.
#[expect(
    clippy::expect_used,
    reason = "a plan's steps read only the steps before them; a read of a later one is a bug that must crash, not a step read as reading nothing"
)]
fn uses_of(plan: &Plan) -> Vec<StepReads> {
    let mut steps: Vec<StepReads> = Vec::with_capacity(plan.steps.len());
    for step in &plan.steps {
        let inputs = inputs_of(&step.operation);
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
                            .get(
                                usize::try_from(id.0)
                                    .expect("compiled bound input index fits usize")
                            )
                            .expect("compiled bound input has its schema")
                            .columns
                            .len()
                    ],
                },
                Input::Step(id) => steps
                    .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                    .cloned()
                    .expect(READS_OF_EARLIER_STEP),
            });
        }
        // Each input's reads are its own, moved rather than copied: the first is what the step starts from.
        let mut parts = parts.into_iter();
        let StepReads {
            mut uses,
            mut provenance,
        } = parts.next().unwrap_or_default();
        match &step.operation {
            Operation::Select {
                comparisons, kept, ..
            } => {
                for comparison in comparisons {
                    if let Operand::Column(column) = &comparison.left
                        && let Some(Some(read)) =
                            provenance.get(usize::try_from(column.0).unwrap_or(usize::MAX))
                        && let Some(each) = uses.get_mut(read.0)
                    {
                        each.kinds.push(comparison.clone());
                    }
                }
                if !kept.is_empty() {
                    provenance = kept
                        .iter()
                        .map(|column| {
                            provenance
                                .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                                .copied()
                                .flatten()
                        })
                        .collect();
                }
            }
            Operation::Project { columns, .. } => {
                provenance = columns
                    .iter()
                    .map(|column| {
                        provenance
                            .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                            .copied()
                            .flatten()
                    })
                    .collect();
            }
            Operation::Extend { values, .. } => {
                provenance.extend(values.iter().map(|_| None));
            }
            Operation::SpanBetween { .. } => provenance.push(None),
            Operation::RenderSyntax { .. } => provenance.push(None),
            Operation::FitsRendered { .. } => {}
            Operation::SourceMatches {
                base_line,
                capture,
                capture_text,
                capture_line,
                ..
            } => {
                provenance.push(None);
                if base_line.is_some() {
                    provenance.push(None);
                }
                if capture.is_some() {
                    provenance.push(None);
                    if *capture_text {
                        provenance.push(None);
                    }
                    if *capture_line {
                        provenance.push(None);
                    }
                }
            }
            Operation::SpanBounds { .. } => provenance.extend([None, None]),
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
                provenance = Vec::new();
            }
        }
        steps.push(StepReads { uses, provenance });
    }
    steps
}

/// Provenance of the initial columns read directly from a declared relation.
fn relation_reads(relation: Relation) -> StepReads {
    let kind = kind_place(relation);
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

/// The reads whose absence of rows each step trusts: for an antijoin or a difference, those its right input has; for a count or a
/// digest of a group, those it groups. Empty for a step that only passes on the rows it is given.
#[must_use]
#[cfg(test)]
pub(crate) fn needs_of(plan: &Plan) -> Vec<Vec<RelationRead>> {
    needs_from(plan, &uses_of(plan))
}

/// What [`needs_of`] answers, from the reads of the plan's steps already worked out.
fn needs_from(plan: &Plan, reads: &[StepReads]) -> Vec<Vec<RelationRead>> {
    let of = |input: &Input| -> Vec<RelationRead> {
        match input {
            Input::BoundObservation(_) => Vec::new(),
            Input::Relation(relation) => vec![RelationRead {
                relation: *relation,
                kinds: Vec::new(),
            }],
            Input::Step(id) => reads
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .map(|each| each.uses.clone())
                .unwrap_or_default(),
        }
    };
    let masks = plan.steps.iter().map(|step| {
        crate::domain::operator::contract::service::semantics(
            ryo_values::domain::plans::service::kind_of(&step.operation),
        )
        .3
    });
    needs_with_masks(plan, of, masks)
}

/// The reads of the relations the steps of a plan trust the absence of rows of, by where the steps run.
#[must_use]
pub(crate) fn scoped_needs(plan: &Plan) -> Needs {
    let mut scoped = Needs::default();
    // What the plan reads is what its last steps have read, with every narrowing made on the way to them.
    let steps = uses_of(plan);
    let mut read_by_another = vec![false; plan.steps.len()];
    for step in &plan.steps {
        for input in named(&inputs_of(&step.operation)) {
            if let Input::Step(id) = input
                && let Some(slot) =
                    read_by_another.get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
            {
                *slot = true;
            }
        }
    }
    for (place, StepReads { uses: each, .. }) in steps.iter().enumerate() {
        if read_by_another.get(place).copied().unwrap_or(false) {
            continue;
        }
        for read in each {
            if !scoped.reads.contains(read) {
                scoped.reads.push(read.clone());
            }
        }
    }
    for (step, needs) in plan.steps.iter().zip(needs_from(plan, &steps)) {
        let into = match step.scope {
            StepScope::PerFile => &mut scoped.per_file,
            StepScope::CrossFile => &mut scoped.cross,
        };
        for read in needs {
            if !into.contains(&read) {
                into.push(read);
            }
        }
    }
    scoped
}

/// Whether a partly read relation could change what a read of it makes: it is that relation, and the rows that could be missing
/// are of a kind the read lets through.
#[must_use]
pub(crate) fn matters(read: &RelationRead, partial: &Partial) -> Bearing {
    let does = read.relation == partial.relation
        && match &partial.lacking {
            Lacking::Any => true,
            Lacking::Kind(kind) => read
                .kinds
                .iter()
                .all(|each| admits(each, kind) == Passage::LetsThrough),
        };
    if does {
        Bearing::Matters
    } else {
        Bearing::DoesNotMatter
    }
}

/// Whether any of the reads `needs` is of a relation read in part in the file.
#[must_use]
pub(crate) fn blocked(needs: &[RelationRead], partial: &[Partial]) -> Blocking {
    if needs.iter().any(|read| {
        partial
            .iter()
            .any(|each| matters(read, each) == Bearing::Matters)
    }) {
        Blocking::Blocked
    } else {
        Blocking::Free
    }
}

/// The files in which any of the reads `needs` is of a relation read in part, once each, in order; none for the repository as a whole.
#[must_use]
pub(crate) fn files_for(partials: &Partials, needs: &[RelationRead]) -> Vec<Option<FileId>> {
    let mut files: Vec<Option<FileId>> = partials
        .entries
        .iter()
        .filter(|each| {
            needs
                .iter()
                .any(|read| matters(read, &each.partial) == Bearing::Matters)
        })
        .map(|each| each.file)
        .chain(partials.unavailable_files.iter().copied().map(Some))
        .collect();
    files.sort();
    files.dedup();
    files
}

/// What the rules could not decide because a relation of the project they trust the absence of rows of is read in part: at the file
/// the partial relation is about, among `named` (the files the run lists, then the manifests it does not), or at the repository.
#[must_use]
pub(crate) fn project_unknowns(
    partials: &[Partial],
    named: &[FilePath],
    needs: &[Needs],
    ids: &[RuleId],
) -> Vec<Unknown> {
    let mut unknowns = Vec::new();
    for (rule, each) in needs.iter().enumerate() {
        for partial in partials {
            if each
                .reads
                .iter()
                .any(|read| matters(read, partial) == Bearing::Matters)
            {
                let at = partial
                    .at
                    .and_then(|file| named.get(usize::try_from(file.0).unwrap_or(usize::MAX)));
                unknowns.push(Unknown {
                    rule: id_at(ids, RuleIndex(rule)),
                    path: at.cloned().unwrap_or_else(|| FilePath(REPOSITORY.into())),
                    message: unknown_message(
                        partial,
                        at.map_or(CoverageScope::Repository, CoverageScope::File),
                    ),
                });
            }
        }
    }
    unknowns
}

fn needs_with_masks(
    plan: &Plan,
    of: impl Fn(&Input) -> Vec<RelationRead>,
    masks: impl IntoIterator<Item = u8>,
) -> Vec<Vec<RelationRead>> {
    plan.steps
        .iter()
        .zip(masks)
        .map(|(step, mask)| {
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
pub(crate) fn needs_of_contracts(
    plan: &Plan,
    contracts: &[std::sync::Arc<crate::domain::operator::contract::types::OperatorContract>],
) -> Result<Vec<Vec<RelationRead>>, crate::domain::operating::errors::OperatingError> {
    if contracts.len() != plan.steps.len() {
        return Err(crate::domain::operating::errors::OperatingError::NotRun);
    }
    let reads = uses_of(plan);
    Ok(needs_with_masks(
        plan,
        |input| match input {
            Input::BoundObservation(_) => Vec::new(),
            Input::Relation(relation) => vec![RelationRead {
                relation: *relation,
                kinds: Vec::new(),
            }],
            Input::Step(id) => reads
                .get(id.0 as usize)
                .map_or_else(Vec::new, |step| step.uses.clone()),
        },
        contracts.iter().map(|contract| contract.absence_inputs),
    ))
}

/// One topological physical-read pass for all demanded roots.
pub(crate) fn physical_reads(plan: &Plan) -> Vec<Vec<RelationRead>> {
    uses_of(plan).into_iter().map(|reads| reads.uses).collect()
}
/// All physical reads needed to certify demanded observations, including an observed
/// intermediate whose diagnostic descendant narrows the same relation further.
pub(crate) fn observed_reads(plan: &Plan) -> Vec<RelationRead> {
    if plan.observations.is_empty() {
        return Vec::new();
    }
    let by_step = physical_reads(plan);
    let mut reads = Vec::new();
    for output in &plan.observations {
        let each = match output.source {
            Input::Step(step) => by_step
                .get(step.0 as usize)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            Input::Relation(_) | Input::BoundObservation(_) => &[],
        };
        absorb(&mut reads, each.iter().cloned());
    }
    reads
}
