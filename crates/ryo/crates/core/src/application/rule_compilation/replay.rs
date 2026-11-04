//! Expand warm producer dependencies using the prepared program's compact root edges.
use crate::application::file_dag::program::facade::PreparedProgram;
use crate::domain::{operating::errors::OperatingError, storing::types::Reuse};
use ryo_values::domain::stored_facts::types::Bucket;

pub(crate) fn observation_closure(
    again: Reuse,
    program: &PreparedProgram,
    first_bucket: &[Bucket],
) -> Result<(Reuse, Vec<Bucket>), OperatingError> {
    program.replay_closure(again, first_bucket)
}
