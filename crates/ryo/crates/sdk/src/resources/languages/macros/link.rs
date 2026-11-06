use std::sync::Arc;

use ryo_language::inbound::use_cases::parse::types::ParseEngine;
use ryo_language::inbound::use_cases::project::types::ProjectEngine;

use crate::domain::language::types::{Language, LanguageBuild, LanguageName};

/// A language as an edge links it: its name, which is the name of its directory, its two use cases, and the build, which is its
/// crate's version: the language does not write the build.
#[macro_export]
macro_rules! link {
    ($name:expr, parse: $parse:expr, project: $project:expr $(,)?) => {
        $crate::languages::link(
            $crate::languages::LanguageName(($name).into()),
            $crate::languages::LanguageBuild(env!("CARGO_PKG_VERSION").into()),
            $parse,
            $project,
        )
    };
}

/// Builds a language from its parts. Called by [`link!`](crate::link), never by hand.
#[doc(hidden)]
#[must_use]
pub fn link(
    name: LanguageName,
    build: LanguageBuild,
    engine: Arc<dyn ParseEngine>,
    project: Arc<dyn ProjectEngine>,
) -> Language {
    Language::from_parts(name, build, engine, project)
}
