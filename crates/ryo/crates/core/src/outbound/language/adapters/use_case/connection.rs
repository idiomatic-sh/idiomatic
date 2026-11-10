//! The languages this adapter was handed by the edge that linked them.

use crate::outbound::language::types::LanguageBuild;
use ryo_values::domain::descriptions::types::LanguageName;

/// The parts of one language an edge hands over: its name, its build, and the two use cases it implements. One value for each
/// language, so no part can be missing or out of step with another.
pub struct LanguageConnection {
    /// What the language is called, in listings.
    pub name: LanguageName,
    /// Which build of the language this is.
    pub build: LanguageBuild,
    /// Reads one file.
    pub parse: std::sync::Arc<dyn ryo_language::inbound::use_cases::parse::types::ParseEngine>,
    /// Reads the repository's manifests.
    pub project:
        std::sync::Arc<dyn ryo_language::inbound::use_cases::project::types::ProjectEngine>,
}

impl std::fmt::Debug for LanguageConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
