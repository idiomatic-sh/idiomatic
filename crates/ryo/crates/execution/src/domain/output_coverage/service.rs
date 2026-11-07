//! The completeness of the answer of an operator, from the completeness of the operands it was given.
use super::types::{InputCoverage, OperandOrdinal, OperandReads};
use crate::domain::completion::service::{
    after_select, any_of, lacks_of, partial_any, partial_of_lacks, remapped,
};
use crate::domain::completion::types::{CoverageState, FileLacks, MissingRows};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::FileId;

/// Whether operand `ordinal` of the operation reads a relation ryo owns. Only such an operand's file and node columns lie in the
/// file its row is partitioned by; a step's columns may carry files and nodes of another operand's rows, and no per-column lineage
/// says which, so a step operand anchors nothing.
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
    if matches!(if ordinal.0 == 0 { left } else { right }, Input::Relation(_)) {
        OperandReads::Relation
    } else {
        OperandReads::Step
    }
}

/// Transport physical uncertainty through an operator's output file identity.
/// A missing row may affect a different file after a non-file join. Keep exact
/// scopes only when every emitted file identity is anchored to the uncertain
/// operand itself or to an explicitly equal file/node key of a relation it reads.
pub(crate) fn output_coverage(
    operation: &ryo_values::domain::plans::types::Operation,
    output_kinds: &[ryo_relations::domain::relations::types::CellKind],
    inputs: &[InputCoverage<'_>],
) -> Result<CoverageState, OperatingError> {
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
                (base && file_keyed) || !other_identity
            }
            Operation::Antijoin {
                left_on, right_on, ..
            } if input.ordinal.0 == 1 => {
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
pub(crate) fn ordered_output_coverage(
    partition: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    output_kinds: &[ryo_relations::domain::relations::types::CellKind],
    source: &InputCoverage<'_>,
) -> CoverageState {
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
    } else if matches!(source.coverage, CoverageState::Complete) {
        CoverageState::Complete
    } else {
        partial_any(vec![None])
    }
}
