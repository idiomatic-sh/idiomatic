//! The test data of the `create` use case: the kind of definition a scenario's word names.

use tastemaker_values::domain::packages::service::kind_of;
use tastemaker_values::domain::packages::types::{DefinitionKind, KindWord};

/// The kind `word` names, as the lock and `create` write it.
#[must_use]
pub fn kind_named(word: &str) -> Option<DefinitionKind> {
    kind_of(KindWord(word))
}
