//! The completeness of a coverage at the file-frontier boundary, and back.
use crate::domain::completion::service::lacks_of;
use crate::domain::completion::types::{CoverageState, FileLacks, MissingRows};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::FileId;

pub fn file_frontier_coverage(
    coverage: CoverageState,
    file: FileId,
) -> Result<ryo_values::domain::records::types::FrontierCoverage, OperatingError> {
    use ryo_values::domain::records::types::{FrontierAffected, FrontierCoverage};
    match affected_files(&coverage) {
        None => Ok(FrontierCoverage::Complete),
        Some(files) => {
            let lacks = match &coverage {
                CoverageState::Partial { lacks, .. } => lacks.as_slice(),
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
            kinds: kinds
                .into_iter()
                .map(|kind| ryo_values::domain::stored_facts::types::PartialKind(kind.0))
                .collect(),
        },
    }
}

/// What a frontier proof of `file` says is missing, as the lacks of a coverage.
pub fn restored_lacks(
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
                .map(|kind| crate::domain::coverage::types::LackingKind(kind.0.clone()))
                .collect(),
        },
    }
}

/// The files a coverage lacks input of, an inapplicable one as the file it is; none when nothing lacks input.
pub fn affected_files(
    coverage: &CoverageState,
) -> Option<Vec<Option<FileId>>> {
    match coverage {
        CoverageState::Complete => None,
        CoverageState::Partial { files, .. } => Some(files.clone()),
        CoverageState::Inapplicable { files } => Some(files.iter().copied().map(Some).collect()),
    }
}
