//! Certify physical completeness at the file-frontier boundary.
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::FileId;
pub(crate) fn file_frontier_coverage(
    coverage: crate::domain::completion::types::CoverageState,
    file: FileId,
) -> Result<ryo_values::domain::records::types::FrontierCoverage, OperatingError> {
    use ryo_values::domain::records::types::{FrontierAffected, FrontierCoverage};
    match affected_files(&coverage) {
        None => Ok(FrontierCoverage::Complete),
        Some(files) => {
            let mut affected = files
                .into_iter()
                .map(|cause| match cause {
                    None => Ok(FrontierAffected::Repository),
                    Some(at) if at == file => Ok(FrontierAffected::CurrentFile),
                    Some(_) => Err(OperatingError::ObservationIncomplete {
                        reason: super::constants::FOREIGN_PARTITION.into(),
                    }),
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

/// The files a coverage lacks input of, an inapplicable one as the file it is; none when nothing lacks input.
pub(crate) fn affected_files(
    coverage: &crate::domain::completion::types::CoverageState,
) -> Option<Vec<Option<FileId>>> {
    use crate::domain::completion::types::CoverageState;
    match coverage {
        CoverageState::Complete => None,
        CoverageState::Partial { files } => Some(files.clone()),
        CoverageState::Inapplicable { files } => Some(files.iter().copied().map(Some).collect()),
    }
}

/// Missing named populations obey the same absence semantics as physical reads.
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
            super::types::CoverageState::Partial { files } => {
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

/// Transport physical uncertainty through an operator's output file identity.
/// A missing row may affect a different file after a non-file join. Keep exact
/// scopes only when every emitted file identity is anchored to the uncertain
/// operand itself or to an explicitly equal file/node key.
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
                &mapped
            }
            super::types::CoverageState::Partial { files } => files,
        };
        if files.is_empty() {
            return Err(OperatingError::ObservationIncomplete {
                reason: "partial contributor has no affected physical scope".into(),
            });
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
                // is a file or a node, the rows it matches of the other operand lie in the missing file as well.
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
                file_keyed || !other_identity
            }
            Operation::Antijoin {
                left_on, right_on, ..
            } if input.ordinal == 1 => {
                // The rows of the left operand a file or node key matches lie in the missing file, as the join above has it.
                right_on.iter().any(|right| {
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
                    })
            }
            Operation::OrderedMatch { .. }
            | Operation::OrderedDfs { .. }
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
            affected.extend_from_slice(files);
        } else {
            affected.push(None);
        }
    }
    affected.sort_unstable();
    affected.dedup();
    inapplicable.sort_unstable();
    inapplicable.dedup();
    Ok(if !affected.is_empty() {
        affected.extend(inapplicable.into_iter().map(Some));
        affected.sort_unstable();
        affected.dedup();
        super::types::CoverageState::Partial { files: affected }
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
        super::types::CoverageState::Partial { files: vec![None] }
    }
}
