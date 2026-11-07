//! Data owned by process output framing.
use crate::domain::canonical_text::types::Label;

/// The exact text of the three parts a receipt identity frames.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ObservationIdentity {
    /// The rule observed.
    pub rule: Label,
    /// The case observed.
    pub name: Label,
    /// The contestant side observed.
    pub side: Label,
}
