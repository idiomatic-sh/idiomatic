//! Certify physical completeness at the file-frontier boundary.
use super::types::{FileLacks, MissingRows};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Operand, PlanComparison};
pub(crate) fn file_frontier_coverage(
    coverage: crate::domain::completion::types::CoverageState,
    file: FileId,
) -> Result<ryo_values::domain::records::types::FrontierCoverage, OperatingError> {
    use ryo_values::domain::records::types::{FrontierAffected, FrontierCoverage};
    match affected_files(&coverage) {
        None => Ok(FrontierCoverage::Complete),
        Some(files) => {
            let lacks = match &coverage {
                super::types::CoverageState::Partial { lacks, .. } => lacks.as_slice(),
                _ => &[],
            };
            let mut affected = lacks_of(&files, lacks)
                .into_iter()
                .map(|entry| {
                    let missing = frontier_missing(entry.missing);
                    match entry.file {
                        None => Ok(FrontierAffected::Repository(missing)),
                        Some(at) if at == file => Ok(FrontierAffected::CurrentFile(missing)),
                        Some(_) => Err(OperatingError::ObservationIncomplete {
                            reason: super::constants::FOREIGN_PARTITION.into(),
                        }),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            affected.sort_unstable();
            affected.dedup();
            if affected.is_empty() {
                return Err(OperatingError::ObservationIncomplete {
                    reason: super::constants::EMPTY_PARTIAL.into(),
                });
            }
            Ok(FrontierCoverage::Partial(affected))
        }
    }
}

/// What a frontier proof says is missing, from what a coverage says of its file.
fn frontier_missing(
    missing: MissingRows,
) -> ryo_values::domain::records::types::FrontierMissing {
    use ryo_values::domain::records::types::FrontierMissing;
    match missing {
        MissingRows::Any => FrontierMissing::Any,
        MissingRows::Kinds { column, kinds } => FrontierMissing::Kinds {
            column,
            kinds: kinds.into_iter().map(|kind| kind.0).collect(),
        },
    }
}

/// What a frontier proof of `file` says is missing, as the lacks of a coverage.
pub(crate) fn restored_lacks(
    file: FileId,
    affected: &[ryo_values::domain::records::types::FrontierAffected],
) -> Vec<FileLacks> {
    use ryo_values::domain::records::types::FrontierAffected;
    affected
        .iter()
        .map(|scope| match scope {
            FrontierAffected::CurrentFile(missing) => FileLacks {
                file: Some(file),
                missing: coverage_missing(missing),
            },
            FrontierAffected::Repository(missing) => FileLacks {
                file: None,
                missing: coverage_missing(missing),
            },
        })
        .collect()
}

/// What a coverage says is missing, from what a frontier proof says of its file.
fn coverage_missing(
    missing: &ryo_values::domain::records::types::FrontierMissing,
) -> MissingRows {
    use ryo_values::domain::records::types::FrontierMissing;
    match missing {
        FrontierMissing::Any => MissingRows::Any,
        FrontierMissing::Kinds { column, kinds } => MissingRows::Kinds {
            column: *column,
            kinds: kinds
                .iter()
                .map(|kind| crate::domain::coverage::types::LackingKind(kind.clone()))
                .collect(),
        },
    }
}

/// The files a coverage lacks input of, an inapplicable one as the file it is; none when nothing lacks input.
pub(crate) fn affected_files(
    coverage: &crate::domain::completion::types::CoverageState,
) -> Option<Vec<Option<FileId>>> {
    use crate::domain::completion::types::CoverageState;
    match coverage {
        CoverageState::Complete => None,
        CoverageState::Partial { files, .. } => Some(files.clone()),
        CoverageState::Inapplicable { files } => Some(files.iter().copied().map(Some).collect()),
    }
}

/// MissingRows named populations obey the same absence semantics as physical reads.
/// Positive operators retain admitted rows; absence-sensitive operators can
/// trust a keyed answer only outside the affected physical partitions.
pub(crate) fn filter_contributor_rows(
    operation: &ryo_values::domain::plans::types::Operation,
    absence_inputs: u8,
    inputs: &[super::types::InputCoverage<'_>],
    rows: &mut ryo_values::domain::rows::types::Table,
) {
    for input in inputs {
        if input.ordinal >= u8::BITS as usize || absence_inputs & (1 << input.ordinal) == 0 {
            continue;
        }
        match &input.coverage {
            super::types::CoverageState::Complete => {}
            super::types::CoverageState::Partial { files, .. } => {
                let mut affected = files.clone();
                affected.sort_unstable();
                affected.dedup();
                crate::domain::coverage::service::drop_uncertain(
                    operation,
                    input.kinds,
                    rows,
                    &affected,
                );
            }
            super::types::CoverageState::Inapplicable { files } => {
                let mut inapplicable = files.clone();
                inapplicable.sort_unstable();
                inapplicable.dedup();
                crate::domain::coverage::service::drop_inapplicable(
                    operation,
                    input.kinds,
                    rows,
                    &inapplicable,
                );
            }
        }
    }
}

/// Whether operand `ordinal` of the operation reads a relation ryo owns. Only such an operand's file and node columns lie in the
/// file its row is partitioned by; a step's columns may carry files and nodes of another operand's rows, and no per-column lineage
/// says which, so a step operand anchors nothing.
fn reads_relation(operation: &ryo_values::domain::plans::types::Operation, ordinal: usize) -> bool {
    use ryo_values::domain::{plan_parts::types::Input, plans::types::Operation};
    let (Operation::Join { left, right, .. }
    | Operation::Antijoin { left, right, .. }
    | Operation::OrderedMatch { left, right, .. }) = operation
    else {
        return false;
    };
    matches!(if ordinal == 0 { left } else { right }, Input::Relation(_))
}

/// Transport physical uncertainty through an operator's output file identity.
/// A missing row may affect a different file after a non-file join. Keep exact
/// scopes only when every emitted file identity is anchored to the uncertain
/// operand itself or to an explicitly equal file/node key of a relation it reads.
pub(crate) fn output_coverage(
    operation: &ryo_values::domain::plans::types::Operation,
    output_kinds: &[ryo_relations::domain::relations::types::CellKind],
    inputs: &[super::types::InputCoverage<'_>],
) -> Result<super::types::CoverageState, OperatingError> {
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::{plan_parts::types::Side, plans::types::Operation};
    let file_kind = |kind: CellKind| matches!(kind, CellKind::File | CellKind::Node);
    let mut affected = Vec::new();
    let mut inapplicable = Vec::new();
    for input in inputs {
        let counted = matches!(
            operation,
            Operation::GroupCount { .. } | Operation::GroupDigest { .. }
        );
        let mapped: Vec<Option<FileId>>;
        let mut entries: Vec<FileLacks>;
        let files = match &input.coverage {
            super::types::CoverageState::Complete => continue,
            // A file out of scope matters to a count only through the file or node the count is keyed by; any other operator
            // treats it as a partial input.
            super::types::CoverageState::Inapplicable { files } if counted => {
                if output_kinds.iter().copied().any(file_kind) {
                    inapplicable.extend_from_slice(files);
                }
                continue;
            }
            super::types::CoverageState::Inapplicable { files } => {
                mapped = files.iter().copied().map(Some).collect();
                entries = lacks_of(&mapped, &[]);
                &mapped
            }
            super::types::CoverageState::Partial { files, lacks } => {
                entries = lacks_of(files, lacks);
                files
            }
        };
        if files.is_empty() {
            return Err(OperatingError::ObservationIncomplete {
                reason: "partial contributor has no affected physical scope".into(),
            });
        }
        let base = reads_relation(operation, input.ordinal);
        // The rows an operator passes on are the rows it was given, so what is missing from them follows its columns; a select
        // on the kind column may rule the missing kinds out altogether.
        entries = if let Operation::Select {
            comparisons, kept, ..
        } = operation
        {
            after_select(comparisons, kept, entries)
        } else if let Operation::Project { columns, .. } = operation {
            remapped(entries, columns)
        } else if matches!(operation, Operation::Extend { .. }) {
            entries
        } else {
            any_of(entries)
        };
        if entries.is_empty() {
            continue;
        }
        let preserves = match operation {
            Operation::Join {
                left_on,
                right_on,
                output,
                view,
                ..
            } => {
                let own = if input.ordinal == 0 {
                    left_on
                } else {
                    right_on
                };
                // A relation's rows are partitioned by their file column and its nodes lie in that file, so when the join key
                // is a file or a node of a relation the operand reads, the rows it matches of the other operand lie in the missing
                // file as well; a step's key says nothing of where its rows came from.
                let mut file_keyed = false;
                for own in own {
                    let own = if input.ordinal == 1 && !view.is_empty() {
                        *view.get(own.0 as usize).ok_or(OperatingError::NotRun)?
                    } else {
                        *own
                    };
                    file_keyed |= input
                        .kinds
                        .get(own.0 as usize)
                        .copied()
                        .is_some_and(file_kind);
                }
                // Only the other operand's file identity depends on the key; the missing operand's own is the missing file.
                let mut other_identity = false;
                for (at, _) in output_kinds
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(_, kind)| file_kind(*kind))
                {
                    let column = output.get(at).ok_or(OperatingError::NotRun)?;
                    other_identity |= match column.side {
                        Side::Left => 0,
                        Side::Right => 1,
                    } != input.ordinal;
                }
                (base && file_keyed) || !other_identity
            }
            Operation::Antijoin {
                left_on, right_on, ..
            } if input.ordinal == 1 => {
                // The rows of the left operand a file or node key matches lie in the missing file, as the join above has it.
                base && (right_on.iter().any(|right| {
                    input
                        .kinds
                        .get(right.0 as usize)
                        .copied()
                        .is_some_and(file_kind)
                }) || output_kinds
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(_, kind)| file_kind(*kind))
                    .all(|(at, kind)| {
                        left_on.iter().zip(right_on).any(|(left, right)| {
                            left.0 as usize == at
                                && input.kinds.get(right.0 as usize).copied() == Some(kind)
                        })
                    }))
            }
            // The rows an ordered match pairs agree on every key, so a file or node key keeps them in the missing file.
            Operation::OrderedMatch {
                left_on, right_on, ..
            } => {
                let own = if input.ordinal == 0 {
                    left_on
                } else {
                    right_on
                };
                (base
                    && own.iter().any(|column| {
                        input
                            .kinds
                            .get(column.0 as usize)
                            .copied()
                            .is_some_and(file_kind)
                    }))
                    || !output_kinds.iter().copied().any(file_kind)
            }
            Operation::OrderedDfs { .. }
            | Operation::Axis { .. }
            | Operation::Closure { .. }
            | Operation::Cycles { .. } => !output_kinds.iter().copied().any(file_kind),
            Operation::Select { .. }
            | Operation::Project { .. }
            | Operation::Extend { .. }
            | Operation::Union { .. }
            | Operation::Intersection { .. }
            | Operation::Difference { .. }
            | Operation::Antijoin { .. }
            | Operation::GroupCount { .. }
            | Operation::GroupDigest { .. }
            | Operation::Capture { .. }
            | Operation::Rank { .. }
            | Operation::Replace { .. }
            | Operation::Words { .. }
            | Operation::Rows { .. }
            | Operation::Hosted { .. }
            | Operation::SpanBetween { .. }
            | Operation::SourceMatches { .. }
            | Operation::RenderSyntax { .. }
            | Operation::FitsRendered { .. }
            | Operation::SpanBounds { .. } => true,
        };
        if preserves {
            affected.extend(entries);
        } else {
            affected.push(FileLacks {
                file: None,
                missing: MissingRows::Any,
            });
        }
    }
    inapplicable.sort_unstable();
    inapplicable.dedup();
    Ok(if !affected.is_empty() {
        affected.extend(inapplicable.into_iter().map(|file| FileLacks {
            file: Some(file),
            missing: MissingRows::Any,
        }));
        partial_of_lacks(affected)
    } else if inapplicable.is_empty() {
        super::types::CoverageState::Complete
    } else {
        super::types::CoverageState::Inapplicable {
            files: inapplicable,
        }
    })
}

/// The ordered existence alternative compares a source with groups derived
/// from that same source. Only a declared file/node partition key certifies
/// that missing rows cannot change answers in another physical file.
pub(crate) fn ordered_output_coverage(
    partition: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    output_kinds: &[ryo_relations::domain::relations::types::CellKind],
    source: &super::types::InputCoverage<'_>,
) -> super::types::CoverageState {
    use ryo_relations::domain::relations::types::CellKind;
    let file_kind = |kind| matches!(kind, CellKind::File | CellKind::Node);
    if !output_kinds.iter().copied().any(file_kind)
        || partition.iter().any(|column| {
            source
                .kinds
                .get(column.0 as usize)
                .copied()
                .is_some_and(file_kind)
        })
    {
        source.coverage.clone()
    } else if matches!(source.coverage, super::types::CoverageState::Complete) {
        super::types::CoverageState::Complete
    } else {
        partial_any(vec![None])
    }
}

/// A coverage partial in these files, any row of which could be missing.
pub(crate) fn partial_any(files: Vec<Option<FileId>>) -> super::types::CoverageState {
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
pub(crate) fn lacks_of(files: &[Option<FileId>], lacks: &[FileLacks]) -> Vec<FileLacks> {
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
pub(crate) fn partial_of_lacks(mut all: Vec<FileLacks>) -> super::types::CoverageState {
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
    super::types::CoverageState::Partial { files, lacks }
}

/// What a select leaves missing: rows of kinds that its comparisons on the kind column stop never reach it, so a file whose every
/// missing kind is stopped is no longer partial.
fn after_select(
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
fn remapped(entries: Vec<FileLacks>, columns: &[ColumnIndex]) -> Vec<FileLacks> {
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
fn any_of(entries: Vec<FileLacks>) -> Vec<FileLacks> {
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
