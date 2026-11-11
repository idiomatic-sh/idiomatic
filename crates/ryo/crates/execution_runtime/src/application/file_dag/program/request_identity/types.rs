//! The types of the `request_identity` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::files::types::CorpusInput;
use std::sync::Arc;

/// Whether a file is one of the listed corpus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CorpusMembership {
    Listed,
    Unlisted,
}
