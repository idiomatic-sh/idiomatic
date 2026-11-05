//! The `crossing` service: free functions named by verb over the cross-file steps of one rule, with no IO and no ports.

use super::types::{CrossingInput, Placeability};
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::interface::{FilePartial, Partials, RuleIndex};
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;

use ryo_execution::inbound::use_cases::metadata::interface::Bearing;
use ryo_relations::domain::relations::types::CellKind;

use ryo_values::domain::plans::types::Emit;

/// Whether an emitted finding can have a place in a file: a stretch, a node or a name among its columns.
#[must_use]
pub(crate) fn placeable(emit: &Emit, table: &Table) -> Placeability {
    let places = emit
        .key
        .iter()
        .chain(std::iter::once(&emit.subject))
        .any(|column| {
            table
                .kinds
                .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                .is_some_and(|kind| {
                    matches!(kind, CellKind::Span | CellKind::Node | CellKind::Symbol)
                })
        });
    if places {
        Placeability::Placeable
    } else {
        Placeability::Unplaceable
    }
}

/// The files that govern a rule, in which a relation its cross-file steps trust the absence of rows of is read in part.
#[must_use]
pub(crate) fn partials_for(crossing: &CrossingInput<'_>, rule: RuleIndex) -> Partials {
    let trusted = crossing
        .needs
        .get(rule.0)
        .map_or(&[][..], |needs| needs.cross.as_slice());
    partials_for_reads(crossing, rule, trusted)
}

/// Publication completeness uses actual root reads, not only absence-sensitive diagnostic Needs.
pub(crate) fn partials_for_reads(
    crossing: &CrossingInput<'_>,
    rule: RuleIndex,
    trusted: &[ryo_execution::inbound::use_cases::metadata::interface::RelationRead],
) -> Partials {
    let matter = |partial: &&ryo_execution::inbound::use_cases::metadata::interface::Partial| {
        trusted
            .iter()
            .any(|read| MetadataUseCaseImpl.matters(read, partial) == Bearing::Matters)
    };
    Partials {
        entries: crossing
            .project
            .iter()
            .filter(matter)
            .map(|partial| FilePartial {
                file: partial.at,
                partial: partial.clone(),
            })
            .chain(
                crossing
                    .covered
                    .iter()
                    .filter(|each| each.rules.contains(&rule))
                    .flat_map(|each| {
                        each.partial
                            .iter()
                            .filter(matter)
                            .map(move |partial| FilePartial {
                                file: Some(each.file),
                                partial: partial.clone(),
                            })
                    }),
            )
            .collect(),
        unavailable_files: crossing
            .ids
            .get(rule.0)
            .into_iter()
            .flat_map(|id| crossing.skipped.iter().filter(move |skip| skip.rule == *id))
            .filter_map(|skip| {
                crossing
                    .paths
                    .iter()
                    .position(|path| *path == skip.path)
                    .and_then(|place| u32::try_from(place).ok())
                    .map(ryo_relations::domain::relations::types::FileId)
            })
            .collect(),
    }
}

use ryo_values::domain::rows::types::Table;
