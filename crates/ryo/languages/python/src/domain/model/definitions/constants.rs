//! The constants of the model's `definitions` part.

use super::types::DefinitionCode;

/// The code of no definition: a node that defines nothing.
pub(crate) const NO_DEFINITION: DefinitionCode = DefinitionCode(u32::MAX);
