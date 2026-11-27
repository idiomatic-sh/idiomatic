//! The `trust` service: whether a relation read in part could change what a read that trusts the absence of rows of it makes, with no
//! IO and no ports.

use super::types::relation_read::value::RelationRead;
use super::types::{Bearing, Blocking, Needs, Passage};
use crate::domain::coverage::constants::REPOSITORY;
use crate::domain::coverage::service::unknown_message;
use crate::domain::coverage::types::{CoverageScope, Lacking, LackingKind, Partial, RuleIndex, Unknown};
use crate::domain::rules::service::id_at;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, QueryLiteral, Test};
use ryo_values::domain::runs::types::RuleId;

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
                    cause: ryo_values::domain::runs::types::UnknownCause::Undecided,
                });
            }
        }
    }
    unknowns
}
