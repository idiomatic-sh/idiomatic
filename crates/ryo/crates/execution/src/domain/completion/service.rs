//! Certify physical completeness at the file-frontier boundary.
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::FileId;
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
