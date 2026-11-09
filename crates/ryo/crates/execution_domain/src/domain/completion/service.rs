//! What a coverage lacks: the rows missing from each file, and how an operator passes them on.
use super::types::{CoverageState, FileLacks, MissingRows};
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Operand, PlanComparison};

/// A coverage partial in these files, any row of which could be missing.
pub fn partial_any(files: Vec<Option<FileId>>) -> CoverageState {
    partial_of_lacks(
        files
            .into_iter()
            .map(|file| FileLacks {
                file,
                missing: MissingRows::Any,
            })
            .collect(),
    )
}

/// What is missing from each of `files`: what `lacks` says of a file, any row of one it does not name.
pub fn lacks_of(files: &[Option<FileId>], lacks: &[FileLacks]) -> Vec<FileLacks> {
    files
        .iter()
        .map(|file| FileLacks {
            file: *file,
            missing: lacks
                .iter()
                .find(|each| each.file == *file)
                .map_or(MissingRows::Any, |each| each.missing.clone()),
        })
        .collect()
}

/// What is missing when either of two things is: rows of kinds of one column stay named only while both name that column.
fn union_missing(first: MissingRows, second: MissingRows) -> MissingRows {
    match (first, second) {
        (
            MissingRows::Kinds { column, mut kinds },
            MissingRows::Kinds {
                column: other,
                kinds: more,
            },
        ) if column == other => {
            kinds.extend(more);
            kinds.sort_unstable();
            kinds.dedup();
            MissingRows::Kinds { column, kinds }
        }
        _ => MissingRows::Any,
    }
}

/// The partial coverage of what is missing from files, sorted, once each; what is missing is the union of what was said of a file.
/// A file that lacks any row is not named, as `lacks` leaves it out.
pub fn partial_of_lacks(mut all: Vec<FileLacks>) -> CoverageState {
    all.sort();
    let mut files: Vec<Option<FileId>> = Vec::with_capacity(all.len());
    let mut lacks: Vec<FileLacks> = Vec::new();
    for each in all {
        match files.last() {
            Some(last) if *last == each.file => {
                if let Some(at) = lacks.iter().position(|existing| existing.file == each.file) {
                    let existing = lacks.swap_remove(at);
                    let missing = union_missing(existing.missing, each.missing);
                    if missing != MissingRows::Any {
                        lacks.push(FileLacks {
                            file: each.file,
                            missing,
                        });
                    }
                }
                // A file already lacking any row stays so.
            }
            _ => {
                files.push(each.file);
                if each.missing != MissingRows::Any {
                    lacks.push(each);
                }
            }
        }
    }
    lacks.sort();
    CoverageState::Partial { files, lacks }
}

/// What a select leaves missing: rows of kinds that its comparisons on the kind column stop never reach it, so a file whose every
/// missing kind is stopped is no longer partial.
pub(crate) fn after_select(
    comparisons: &[PlanComparison],
    kept: &[ColumnIndex],
    entries: Vec<FileLacks>,
) -> Vec<FileLacks> {
    use crate::domain::trust::types::Passage;
    entries
        .into_iter()
        .filter_map(|entry| match entry.missing {
            MissingRows::Any => Some(entry),
            MissingRows::Kinds { column, kinds } => {
                let stopped = kinds.iter().all(|kind| {
                    comparisons.iter().any(|comparison| {
                        comparison.left == Operand::Column(column)
                            && crate::domain::trust::service::admits(comparison, kind)
                                == Passage::Blocks
                    })
                });
                if stopped {
                    return None;
                }
                let place = if kept.is_empty() {
                    Some(column)
                } else {
                    moved(column, kept)
                };
                Some(FileLacks {
                    file: entry.file,
                    missing: place.map_or(MissingRows::Any, |column| MissingRows::Kinds {
                        column,
                        kinds,
                    }),
                })
            }
        })
        .collect()
}

/// What a projection leaves missing: the kind column where the projection puts it, any row when it drops it.
pub(crate) fn remapped(entries: Vec<FileLacks>, columns: &[ColumnIndex]) -> Vec<FileLacks> {
    entries
        .into_iter()
        .map(|entry| FileLacks {
            file: entry.file,
            missing: match entry.missing {
                MissingRows::Kinds { column, kinds } => {
                    moved(column, columns).map_or(MissingRows::Any, |column| MissingRows::Kinds {
                        column,
                        kinds,
                    })
                }
                MissingRows::Any => MissingRows::Any,
            },
        })
        .collect()
}

/// What an operator that does not carry columns through leaves missing: any row of each file.
pub(crate) fn any_of(entries: Vec<FileLacks>) -> Vec<FileLacks> {
    entries
        .into_iter()
        .map(|entry| FileLacks {
            file: entry.file,
            missing: MissingRows::Any,
        })
        .collect()
}

/// What the partly read relation `partial` leaves missing from the relation's own columns: the rows of its kind when the reader
/// names one and the relation has a kind column, any row otherwise.
fn missing_of_partial(partial: &crate::domain::coverage::types::Partial) -> MissingRows {
    use crate::domain::coverage::types::Lacking;
    match &partial.lacking {
        Lacking::Any => MissingRows::Any,
        Lacking::Kind(kind) => crate::domain::trust::service::kind_place(partial.relation).map_or(
            MissingRows::Any,
            |column| MissingRows::Kinds {
                column,
                kinds: vec![kind.clone()],
            },
        ),
    }
}

/// What the partly read `partials` of a relation leave missing together: their union, any row when none is named.
pub(crate) fn missing_of_partials<'a>(
    partials: impl Iterator<Item = &'a crate::domain::coverage::types::Partial>,
) -> MissingRows {
    partials
        .map(missing_of_partial)
        .reduce(union_missing)
        .unwrap_or(MissingRows::Any)
}

/// Where `column` of the input is among the columns an operator keeps, in order, when it is kept.
fn moved(column: ColumnIndex, kept: &[ColumnIndex]) -> Option<ColumnIndex> {
    let at = kept.iter().position(|each| *each == column)?;
    match u32::try_from(at) {
        Ok(at) => Some(ColumnIndex(at)),
        Err(_) => None,
    }
}

/// What a publication covers when it was computed from these inputs: complete when each was, partial when one was (any file out of
/// scope then lacking any row of itself), and inapplicable in the files any input was inapplicable in otherwise.
///
/// # Errors
///
/// `NotRun` when partial inputs merge to no partial coverage, which no coverage of theirs can.
pub fn contributed<'a>(
    inputs: impl IntoIterator<Item = &'a CoverageState>,
) -> Result<CoverageState, crate::domain::operating::errors::OperatingError> {
    let mut affected = Vec::new();
    let mut inapplicable = Vec::new();
    let mut partial = false;
    for input in inputs {
        match input {
            CoverageState::Complete => {}
            CoverageState::Partial { files, lacks } => {
                partial = true;
                affected.extend(lacks_of(files, lacks));
            }
            CoverageState::Inapplicable { files } => {
                for file in files {
                    if !inapplicable.contains(file) {
                        inapplicable.push(*file);
                    }
                }
            }
        }
    }
    if partial {
        affected.extend(inapplicable.into_iter().map(|file| FileLacks {
            file: Some(file),
            missing: MissingRows::Any,
        }));
        match partial_of_lacks(affected) {
            partial @ CoverageState::Partial { .. } => Ok(partial),
            _ => Err(crate::domain::operating::errors::OperatingError::NotRun),
        }
    } else if inapplicable.is_empty() {
        Ok(CoverageState::Complete)
    } else {
        Ok(CoverageState::Inapplicable {
            files: inapplicable,
        })
    }
}
