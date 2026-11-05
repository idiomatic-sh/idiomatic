//! Merge physical completeness and certify file frontier provenance.
use super::types::CoverageState;
use crate::domain::operating::types::OperatingError;
use ryo_relations::domain::relations::types::FileId;
/// Physical uncertainty is monotone across contributing populations and the current region.
pub(crate) fn merge_coverage(left: CoverageState, right: CoverageState) -> CoverageState {
    match (left, right) {
        (CoverageState::Complete, coverage) | (coverage, CoverageState::Complete) => coverage,
        (CoverageState::Partial { mut files }, CoverageState::Partial { files: added }) => {
            for file in added {
                if !files.contains(&file) {
                    files.push(file);
                }
            }
            CoverageState::Partial { files }
        }
    }
}

pub(crate) fn file_frontier_coverage(
    coverage: crate::domain::completion::types::CoverageState,
    file: FileId,
) -> Result<ryo_values::domain::records::types::FrontierCoverage, OperatingError> {
    use ryo_values::domain::records::types::{FrontierAffected, FrontierCoverage};
    match coverage {
        crate::domain::completion::types::CoverageState::Complete => Ok(FrontierCoverage::Complete),
        crate::domain::completion::types::CoverageState::Partial { files } => {
            let mut affected = files
                .into_iter()
                .map(|cause| match cause {
                    None => Ok(FrontierAffected::Repository),
                    Some(at) if at == file => Ok(FrontierAffected::CurrentFile),
                    Some(_) => Err(OperatingError::ObservationIncomplete {
                        reason: "file frontier carries a foreign physical partition".into(),
                    }),
                })
                .collect::<Result<Vec<_>, _>>()?;
            affected.sort_unstable();
            affected.dedup();
            if affected.is_empty() {
                return Err(OperatingError::ObservationIncomplete {
                    reason: "partial frontier has no affected physical partition".into(),
                });
            }
            Ok(FrontierCoverage::Partial(affected))
        }
    }
}
