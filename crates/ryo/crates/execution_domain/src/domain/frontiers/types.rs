//! The types of the `frontiers` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// One required source that cannot provide rows; never represented by an empty Table.
#[derive(Clone, Debug)]
pub struct RefusedFrontier {
    /// The source that cannot provide rows.
    pub input: ryo_values::domain::plan_parts::types::Input,
    /// Why it cannot.
    pub error: crate::domain::operating::errors::OperatingError,
}
