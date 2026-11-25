use std::sync::Arc;

use ryo_language::inbound::use_cases::parse::types::ParseEngine;
use ryo_language::inbound::use_cases::project::types::ProjectEngine;
use ryo_language::inbound::use_cases::walk::types::WalkEngine;

use crate::domain::language::types::{Language, LanguageBuild, LanguageName};

/// A language as an edge links it: its name, which is the name of its directory, its two use cases, and the build, which is its
/// crate's version: the language does not write the build. Every language implements `Walk` and links it with `walk:`, beside the
/// two: the engine scans its files through that walk, and a language linked without one does not compile.
///
/// # Examples
///
/// ```
/// use ryo::languages::{Parse, Project, Walk};
/// use std::sync::Arc;
///
/// fn linked<P: Parse + 'static, J: Project + 'static, W: Walk>(parse: P, project: J, walk: W) -> ryo::languages::Language {
///     ryo::link!("words", parse: Arc::new(parse), project: Arc::new(project), walk: Arc::new(walk))
/// }
/// ```
#[macro_export]
macro_rules! link {
    ($name:expr, parse: $parse:expr, project: $project:expr, walk: $walk:expr $(,)?) => {
        $crate::languages::walking(
            $crate::languages::link(
                $crate::languages::LanguageName(($name).into()),
                $crate::languages::LanguageBuild($crate::language_crate_version!().into()),
                $parse,
                $project,
            ),
            $walk,
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

/// A linked language with its walk for the new engine. Called by the `walk:` arm of [`link!`](crate::link), never by hand.
///
/// # Examples
///
/// ```
/// use ryo::languages::{Language, Walk};
/// use std::sync::Arc;
///
/// fn with_its_walk<W: Walk>(linked: Language, walk: W) -> Language {
///     ryo::languages::walking(linked, Arc::new(walk))
/// }
/// ```
#[doc(hidden)]
#[must_use]
pub fn walking(language: Language, walk: Arc<dyn WalkEngine>) -> Language {
    language.walking(walk)
}
