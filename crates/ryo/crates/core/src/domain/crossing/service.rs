//! The `crossing` service: free functions named by verb over the cross-file steps of one rule, with no IO and no ports.

use super::types::{CrossingInput, Placeability};
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::interface::{FilePartial, Partials, RuleIndex};

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
pub(crate) fn partials_for(
    metadata: &impl MetadataUseCase,
    crossing: &CrossingInput<'_>,
    rule: RuleIndex,
) -> Partials {
    let trusted = crossing
        .needs
        .get(rule.0)
        .map_or(&[][..], |needs| needs.cross.as_slice());
    partials_for_reads(metadata, crossing, rule, trusted)
}

/// Publication completeness uses actual root reads, not only absence-sensitive diagnostic Needs.
pub(crate) fn partials_for_reads(
    metadata: &impl MetadataUseCase,
    crossing: &CrossingInput<'_>,
    rule: RuleIndex,
    trusted: &[ryo_execution::inbound::use_cases::metadata::interface::RelationRead],
) -> Partials {
    let matter = |partial: &&ryo_execution::inbound::use_cases::metadata::interface::Partial| {
        trusted
            .iter()
            .any(|read| metadata.matters(read, partial) == Bearing::Matters)
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
                                file: ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::File(each.file),
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
                let place = crossing.paths.iter().position(|path| *path == skip.path)?;
                let Ok(number) = u32::try_from(place) else {
                    return None;
                };
                Some(ryo_relations::domain::relations::types::FileId(number))
            })
            .collect(),
    }
}

use ryo_values::domain::rows::types::Table;

/// For each rule, the files of the run it has no frame for: those its scopes do not place it on, and those no language read.
#[must_use]
pub(crate) fn inapplicable_files(
    metadata: &impl MetadataUseCase,
    crossing: &CrossingInput<'_>,
) -> Vec<ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles> {
    use ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles;
    use ryo_execution::inbound::use_cases::metadata::interface::{Governing, Mask};
    use ryo_values::domain::files::types::Hits;
    use ryo_values::domain::records::types::Unreadable;
    use ryo_values::domain::runs::types::RuleNumber;
    let no_language: std::collections::BTreeSet<&ryo_language::domain::files::types::FilePath> =
        crossing
            .unread
            .iter()
            .filter(|each| each.reason == Unreadable::NoLanguage)
            .map(|each| &each.path)
            .collect();
    let mut inapplicable = vec![InapplicableFiles::default(); crossing.ids.len()];
    let (mut mask, mut hits) = (Mask::default(), Hits::default());
    for (index, path) in crossing.paths.iter().take(crossing.files.len()).enumerate() {
        let Ok(number) = u32::try_from(index) else {
            break;
        };
        metadata.place(crossing.placed, path, &mut mask, &mut hits);
        for (rule, files) in inapplicable.iter_mut().enumerate() {
            let governs = metadata
                .governs(&mask, RuleNumber(u32::try_from(rule).unwrap_or(u32::MAX)))
                == Governing::Governs;
            if !governs || no_language.contains(path) {
                files
                    .0
                    .push(ryo_relations::domain::relations::types::FileId(number));
            }
        }
    }
    inapplicable
}

/// For each rule, the files of the run it was refused on: the file analysis left the rule's applicability there undecided, so the
/// rule has no frame for them and its answer lacks their input.
#[must_use]
pub(crate) fn refused_files(
    crossing: &CrossingInput<'_>,
) -> Vec<ryo_execution::inbound::use_cases::execution::interface::RefusedFiles> {
    use ryo_execution::inbound::use_cases::execution::interface::RefusedFiles;
    let mut refused = vec![RefusedFiles::default(); crossing.ids.len()];
    if crossing.refused.is_empty() {
        return refused;
    }
    // Each file's number by its path, once; each rule then probes only the files it was refused on.
    let numbers = crossing
        .paths
        .iter()
        .take(crossing.files.len())
        .enumerate()
        .map_while(|(index, path)| match u32::try_from(index) {
            Ok(number) => Some((
                path,
                ryo_relations::domain::relations::types::FileId(number),
            )),
            // No file past the last numbered one has a number, as in `inapplicable_files`.
            Err(_) => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    for (id, files) in crossing.ids.iter().zip(refused.iter_mut()) {
        let Some(paths) = crossing.refused.get(id) else {
            continue;
        };
        files.0 = paths
            .iter()
            .filter_map(|path| numbers.get(path).copied())
            .collect();
        files.0.sort_unstable();
    }
    refused
}
