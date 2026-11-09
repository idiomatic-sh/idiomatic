//! The completeness of the answer of an operator, from the completeness of the operands it was given.
use super::types::Proof;
use super::types::{InputCoverage, OperandOrdinal, OperandReads};
use crate::domain::completion::service::{
    after_select, any_of, lacks_of, partial_any, partial_of_lacks, remapped,
};
use crate::domain::completion::types::{CoverageState, FileLacks, MissingRows};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::lineage::service::{anchor_at, anchor_of, anchor_through, pairing};
use ryo_values::domain::lineage::types::{ColumnAnchor, KeyPairing};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn};

/// Whether operand `ordinal` of the operation reads a relation ryo owns. Such an operand's file and node columns lie in the file
/// its row is partitioned by; a step's columns may carry files and nodes of another operand's rows, so a step's key anchors a
/// missing file only where the step's lineage proves its rows and that key lie in one file ([`proven_by_lineage`]).
fn reads_relation(
    operation: &ryo_values::domain::plans::types::Operation,
    ordinal: OperandOrdinal,
) -> OperandReads {
    use ryo_values::domain::{plan_parts::types::Input, plans::types::Operation};
    let (Operation::Join { left, right, .. }
    | Operation::Antijoin { left, right, .. }
    | Operation::OrderedMatch { left, right, .. }) = operation
    else {
        return OperandReads::Step;
    };
    if matches!(
        if ordinal.0 == 0 { left } else { right },
        Input::Relation(_)
    ) {
        OperandReads::Relation
    } else {
        OperandReads::Step
    }
}

/// Whether the lineage of the two operands of a join proves where the rows missing from either would lie: a key
/// pairs a column of each operand that lies in its row's file, so the two rows a missing one would have been combined with come
/// from its file, and every file or node column of the answer lies in its row's file, so it lies in the missing file too. The
/// right keys and the answer's right columns are read through the view.
fn proven_by_lineage(
    inputs: &[InputCoverage<'_>],
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
    output: &[JoinColumn],
    view: &[ColumnIndex],
    output_kinds: &[ryo_relations::domain::relations::types::CellKind],
) -> Proof {
    use ryo_values::domain::plan_parts::types::Side;
    let lineage_of = |at: OperandOrdinal| {
        inputs
            .iter()
            .find(|each| each.ordinal == at)
            .map(|each| &each.lineage)
    };
    let (Some(left), Some(right)) = (lineage_of(OperandOrdinal(0)), lineage_of(OperandOrdinal(1)))
    else {
        return Proof::Unproven;
    };
    if pairing(left, left_on, right, right_on, view) == KeyPairing::Unproven {
        return Proof::Unproven;
    }
    let anchored = output.iter().zip(output_kinds).all(|(column, kind)| {
        anchor_of(*kind) != ColumnAnchor::InFile
            || match column.side {
                Side::Left => anchor_at(left, column.column),
                Side::Right => anchor_through(right, view, column.column),
            } == ColumnAnchor::InFile
    });
    if anchored && output.len() == output_kinds.len() {
        Proof::Proven
    } else {
        Proof::Unproven
    }
}

/// Whether every file or node column of the answer lies in the file the rows missing from `missing` come from, by that operand's
/// lineage alone: a column of the missing operand lies there when its lineage says so, and a column of the other operand when it
/// is a key paired with a key of the missing operand that lies in its row's file and holds the same kind, as the two hold the same
/// file or node. The other operand's own lineage is not needed: the key equality carries the missing file into its columns. The
/// right keys and the answer's right columns are read through the view.
fn transported(
    missing: &InputCoverage<'_>,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
    output: &[JoinColumn],
    view: &[ColumnIndex],
    output_kinds: &[ryo_relations::domain::relations::types::CellKind],
) -> Proof {
    use ryo_values::domain::plan_parts::types::Side;
    let own_side = if missing.ordinal.0 == 0 {
        Side::Left
    } else {
        Side::Right
    };
    // The missing operand's own column a join names: through the view on the right.
    let own = |column: ColumnIndex| {
        if own_side == Side::Right && !view.is_empty() {
            view.get(column.0 as usize).copied()
        } else {
            Some(column)
        }
    };
    let in_file = |column: ColumnIndex| anchor_at(&missing.lineage, column) == ColumnAnchor::InFile;
    let (missing_on, other_on) = if own_side == Side::Left {
        (left_on, right_on)
    } else {
        (right_on, left_on)
    };
    let anchored = output.iter().zip(output_kinds).all(|(column, kind)| {
        if anchor_of(*kind) != ColumnAnchor::InFile {
            return true;
        }
        if column.side == own_side {
            return own(column.column).is_some_and(in_file);
        }
        other_on.iter().zip(missing_on).any(|(other, key)| {
            *other == column.column
                && own(*key).is_some_and(|key| {
                    in_file(key) && missing.kinds.get(key.0 as usize) == Some(kind)
                })
        })
    });
    if anchored && output.len() == output_kinds.len() {
        Proof::Proven
    } else {
        Proof::Unproven
    }
}

/// Transport physical uncertainty through an operator's output file identity.
/// A missing row may affect a different file after a non-file join. Keep exact
/// scopes only when every emitted file identity is anchored to the uncertain
/// operand itself or to an explicitly equal file/node key of a relation it reads.
pub fn output_coverage(
    operation: &ryo_values::domain::plans::types::Operation,
    output_kinds: &[ryo_relations::domain::relations::types::CellKind],
    inputs: &[InputCoverage<'_>],
) -> Result<CoverageState, OperatingError> {
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::{plan_parts::types::Side, plans::types::Operation};
    let file_kind = |kind: CellKind| anchor_of(kind) == ColumnAnchor::InFile;
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
            CoverageState::Complete => continue,
            // A file out of scope matters to a count only through the file or node the count is keyed by; any other operator
            // treats it as a partial input.
            CoverageState::Inapplicable { files } if counted => {
                if output_kinds.iter().copied().any(file_kind) {
                    inapplicable.extend_from_slice(files);
                }
                continue;
            }
            CoverageState::Inapplicable { files } => {
                mapped = files.iter().copied().map(Some).collect();
                entries = lacks_of(&mapped, &[]);
                &mapped
            }
            CoverageState::Partial { files, lacks } => {
                entries = lacks_of(files, lacks);
                files
            }
        };
        if files.is_empty() {
            return Err(OperatingError::ObservationIncomplete {
                reason: super::constants::NO_AFFECTED_SCOPE.into(),
            });
        }
        let base = reads_relation(operation, input.ordinal) == OperandReads::Relation;
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
                let proven =
                    proven_by_lineage(inputs, left_on, right_on, output, view, output_kinds);
                let own = if input.ordinal.0 == 0 {
                    left_on
                } else {
                    right_on
                };
                // A relation's rows are partitioned by their file column and its nodes lie in that file, so when the join key
                // is a file or a node of a relation the operand reads, the rows it matches of the other operand lie in the missing
                // file as well; a step's key says nothing of where its rows came from.
                let mut file_keyed = false;
                for own in own {
                    let own = if input.ordinal.0 == 1 && !view.is_empty() {
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
                    } != input.ordinal.0;
                }
                (base && file_keyed)
                    || !other_identity
                    || proven == Proof::Proven
                    || transported(input, left_on, right_on, output, view, output_kinds)
                        == Proof::Proven
            }
            Operation::Antijoin {
                left_on, right_on, ..
            } if input.ordinal.0 == 1 => {
                // The left rows a missing right row could keep out share a key with it that lies in its row's file, so they
                // come from that file, and so do their own columns that lie in their row's file.
                let proven = inputs
                    .iter()
                    .find(|each| each.ordinal.0 == 0)
                    .is_some_and(|left| {
                        pairing(&left.lineage, left_on, &input.lineage, right_on, &[])
                            == KeyPairing::SameFile
                            && output_kinds.iter().enumerate().all(|(at, kind)| {
                                !file_kind(*kind)
                                    || u32::try_from(at).is_ok_and(|at| {
                                        anchor_at(&left.lineage, ColumnIndex(at))
                                            == ColumnAnchor::InFile
                                    })
                            })
                    });
                // The answer is the left rows: its file or node columns are carried from the missing right rows by the keys.
                let left_columns: Vec<JoinColumn> = (0..output_kinds.len())
                    .map(|at| JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(u32::try_from(at).unwrap_or(u32::MAX)),
                    })
                    .collect();
                proven
                    || transported(input, left_on, right_on, &left_columns, &[], output_kinds)
                        == Proof::Proven
                    || (base
                        && (right_on.iter().any(|right| {
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
                            })))
            }
            // The rows an ordered match pairs agree on every key, so a file or node key keeps them in the missing file.
            Operation::OrderedMatch {
                left_on,
                right_on,
                output,
                ..
            } => {
                let proven =
                    proven_by_lineage(inputs, left_on, right_on, output, &[], output_kinds);
                let own = if input.ordinal.0 == 0 {
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
                    || proven == Proof::Proven
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
        CoverageState::Complete
    } else {
        CoverageState::Inapplicable {
            files: inapplicable,
        }
    })
}

/// The ordered existence alternative compares a source with groups derived
/// from that same source. Only a declared file/node partition key certifies
/// that missing rows cannot change answers in another physical file.
pub fn ordered_output_coverage(
    partition: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    output_kinds: &[ryo_relations::domain::relations::types::CellKind],
    source: &InputCoverage<'_>,
) -> CoverageState {
    use ryo_relations::domain::relations::types::CellKind;
    let file_kind = |kind: CellKind| anchor_of(kind) == ColumnAnchor::InFile;
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
    } else if matches!(source.coverage, CoverageState::Complete) {
        CoverageState::Complete
    } else {
        partial_any(vec![None])
    }
}
