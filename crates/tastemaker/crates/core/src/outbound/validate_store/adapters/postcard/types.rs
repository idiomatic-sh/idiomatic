//! The types of the postcard validate store adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::proofs::types::ProvenVersion;

/// What a `-n/--dry-run` kept and did not write: the set of proofs the last such keep gave, which the next keep replaces.
#[derive(Debug, Default)]
pub(crate) struct Unwritten(pub std::sync::Mutex<Option<Vec<ProvenVersion>>>);
