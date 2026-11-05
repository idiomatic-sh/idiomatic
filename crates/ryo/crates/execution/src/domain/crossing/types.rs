//! Admitted cross-phase publication transfers; no raw-input fallback.
use ryo_values::domain::stored_facts::types::ArenaBytes;
/// One required source that cannot provide rows; never represented by an empty Table.
#[derive(Clone, Debug)]
pub(crate) struct RefusedFrontier {
    pub input: ryo_values::domain::plan_parts::types::Input,
    pub error: crate::domain::operating::errors::OperatingError,
}
