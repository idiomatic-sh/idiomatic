//! The SDK's `project` concept: the project lookups a run is given: the search scope.

use crate::domain::file::types::RelativePath;

/// Explicit host authority for project import roots; an empty declared list is meaningful.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ProjectSearchScope {
    /// No roots were declared, so local membership cannot be proved absent.
    #[default]
    Undeclared,
    /// Repository-relative search roots, in the order the host uses them.
    Declared(Vec<RelativePath>),
}

// The conversions from and into the engine's project lookups are the crate's own: a public `From` impl would put the engine's types
// on the SDK's surface.
impl ProjectSearchScope {
    /// The search roots as the engine reads them.
    pub(crate) fn into_engine(
        self,
    ) -> ryo_language::domain::project_context::types::ProjectSearchScope {
        use ryo_language::domain::project_context::types::ProjectSearchScope as Engine;
        match self {
            Self::Undeclared => Engine::Undeclared,
            Self::Declared(roots) => Engine::Declared(
                roots
                    .into_iter()
                    .map(|root| ryo_language::domain::files::types::FilePath(root.0))
                    .collect(),
            ),
        }
    }
}
