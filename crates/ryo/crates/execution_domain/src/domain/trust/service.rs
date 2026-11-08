//! The `trust` service: which reads of the relations a plan trusts the absence of rows of (an antijoin, a difference, a count), and
//! whether a relation read in part could change what such a read makes, with no IO and no ports.

use super::reads::{absorb, kind_column_place, needs_with_masks, uses_of};
use ryo_relations::domain::relations::types::Relation;

use super::types::{AbsenceInputs, Bearing, Blocking, Needs, Passage, RelationRead, StepReads};
use crate::domain::operating::errors::OperatingError;
use crate::domain::coverage::constants::REPOSITORY;
use crate::domain::coverage::service::unknown_message;
use crate::domain::coverage::types::{
    CoverageScope, Lacking, LackingKind, Partial, Partials, RuleIndex, Unknown,
};
use crate::domain::rules::service::id_at;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::runs::types::RuleId;

/// The place of the `kind` column of a relation, when it has one.
pub(crate) fn kind_place(relation: Relation) -> Option<ColumnIndex> {
    kind_column_place(relation)
}

/// Whether a comparison lets a row of this kind through, as far as it can be told from the comparison alone.
pub(crate) fn admits(comparison: &PlanComparison, kind: &LackingKind) -> Passage {
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

/// The reads whose absence of rows each step trusts: for an antijoin or a difference, those its right input has; for a count or a
/// digest of a group, those it groups. Empty for a step that only passes on the rows it is given.
#[must_use]
#[cfg(test)]
pub(crate) fn needs_of(plan: &Plan) -> Vec<Vec<RelationRead>> {
    needs_from(plan, &uses_of(plan))
}

/// What a step input reads, from the reads of the plan's steps already worked out: a step refused for an authoring error reads
/// nothing here because it never runs; its rule is refused on its own roots by its errored origin.
fn input_reads(reads: &[Result<StepReads, OperatingError>], input: &Input) -> Vec<RelationRead> {
    match input {
        Input::BoundObservation(_) => Vec::new(),
        Input::Relation(relation) => vec![RelationRead {
            relation: *relation,
            kinds: Vec::new(),
        }],
        Input::Step(id) => match reads.get(id.0 as usize) {
            Some(Ok(each)) => each.uses.clone(),
            Some(Err(_)) | None => Vec::new(),
        },
    }
}

/// What [`needs_of`] answers, from the reads of the plan's steps already worked out.
fn needs_from(plan: &Plan, reads: &[Result<StepReads, OperatingError>]) -> Vec<Vec<RelationRead>> {
    let masks = plan.steps.iter().map(|step| {
        AbsenceInputs(
            crate::domain::operator::contract::service::semantics(
                ryo_values::domain::plans::service::kind_of(&step.operation),
            )
            .absence
            .0,
        )
    });
    needs_with_masks(plan, |input| input_reads(reads, input), masks)
}

/// The reads of the relations the steps of a plan trust the absence of rows of, by where the steps run.
#[must_use]
pub fn scoped_needs(plan: &Plan) -> Needs {
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
    for (place, reads) in steps.iter().enumerate() {
        if read_by_another.get(place).copied().unwrap_or(false) {
            continue;
        }
        // A step refused for an authoring error never runs: its rule is refused on its roots, and it reads nothing here.
        let Ok(StepReads { uses: each, .. }) = reads else {
            continue;
        };
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
pub fn matters(read: &RelationRead, partial: &Partial) -> Bearing {
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
pub fn blocked(needs: &[RelationRead], partial: &[Partial]) -> Blocking {
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
/// A file skipped for the rule is not among them: see [`skipped_files`].
#[must_use]
pub(crate) fn partial_files_for(
    partials: &Partials,
    needs: &[RelationRead],
) -> Vec<Option<FileId>> {
    let mut files: Vec<Option<FileId>> = partials
        .entries
        .iter()
        .filter(|each| {
            needs
                .iter()
                .any(|read| matches!(matters(read, &each.partial), Bearing::Matters))
        })
        .map(|each| match each.file {
            crate::domain::coverage::types::PartialPlace::File(file) => Some(file),
            crate::domain::coverage::types::PartialPlace::Repository => None,
        })
        .collect();
    files.sort();
    files.dedup();
    files
}

/// The files skipped for the rule before its cross-file steps, sorted, once each; they lack input without a relation read in part.
#[must_use]
pub(crate) fn skipped_files(partials: &Partials) -> Vec<FileId> {
    let mut files = partials.unavailable_files.clone();
    files.sort();
    files.dedup();
    files
}

/// The files in which any of the reads `needs` is of a relation read in part, or which were skipped for the rule, once each.
#[must_use]
pub(crate) fn files_for(partials: &Partials, needs: &[RelationRead]) -> Vec<Option<FileId>> {
    let mut files = partial_files_for(partials, needs);
    files.extend(skipped_files(partials).into_iter().map(Some));
    files.sort();
    files.dedup();
    files
}

/// What the rules could not decide because a relation of the project they trust the absence of rows of is read in part: at the file
/// the partial relation is about, among `named` (the files the run lists, then the manifests it does not), or at the repository.
#[must_use]
pub fn project_unknowns(
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
                let at = match partial.at {
                    crate::domain::coverage::types::PartialPlace::File(file) => {
                        named.get(usize::try_from(file.0).unwrap_or(usize::MAX))
                    }
                    crate::domain::coverage::types::PartialPlace::Repository => None,
                };
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

/// What [`needs_of`] answers from the prepared operator contracts of the plan's steps.
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
        |input| input_reads(&reads, input),
        contracts
            .iter()
            .map(|contract| AbsenceInputs(contract.absence_inputs.0)),
    ))
}

/// One topological physical-read pass for all demanded roots: each step's reads, or the refusal of a step that reads a step not
/// before it.
pub fn physical_reads(plan: &Plan) -> Vec<Result<Vec<RelationRead>, OperatingError>> {
    uses_of(plan)
        .into_iter()
        .map(|reads| reads.map(|reads| reads.uses))
        .collect()
}
/// All physical reads needed to certify demanded observations, including an observed
/// intermediate whose diagnostic descendant narrows the same relation further.
pub fn observed_reads(plan: &Plan) -> Vec<RelationRead> {
    if plan.observations.is_empty() {
        return Vec::new();
    }
    let by_step = physical_reads(plan);
    let mut reads = Vec::new();
    for output in &plan.observations {
        let each = match output.source {
            Input::Step(step) => match by_step.get(step.0 as usize) {
                Some(Ok(reads)) => reads.as_slice(),
                Some(Err(_)) | None => &[],
            },
            Input::Relation(_) | Input::BoundObservation(_) => &[],
        };
        absorb(&mut reads, each.iter().cloned());
    }
    reads
}
