//! The types of the `open` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_language::inbound::use_cases::project::types::RootPath;

use ryo_values::domain::runs::types::Session;

/// What a session is opened over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenRequest {
    /// The repository's root directory.
    pub root: RootPath,
    /// Where its files are: on the disk below the root, or only those given.
    pub repository: ryo_language::inbound::use_cases::project::types::Repository,
    /// Ordered project lookup roots supplied by the caller.
    pub search_scope: ryo_language::domain::project_context::types::ProjectSearchScope,
    /// Host-provided entry authority for a Given repository.
    pub given_inventory: ryo_language::domain::project_context::types::GivenProjectInventory,
}

/// The session `lint` is given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenReport {
    /// The session.
    pub session: Session,
}
