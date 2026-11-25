#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]
#![cfg_attr(
    test,
    expect(
        dead_code_pub_in_binary,
        reason = "the unit-test build is the one binary that compiles this language's public items with no dependent: the command line crate links the language and calls its contracts and fixtures, and the language's own unit tests do not"
    )
)]
#![cfg_attr(
    all(feature = "fixtures", not(test)),
    allow(
        dead_code,
        reason = "the ports' own fixtures are used by the tests of the crate; outside tests the feature surface is `fixtures` at the root"
    )
)]

pub(crate) mod inbound;
pub(crate) mod outbound;

mod application;
mod domain;

pub mod settings;

/// The Python language, composed over its ports: what an edge links to read Python.
#[must_use]
pub fn linked() -> ryo::languages::Language {
    linked_with_settings(settings::PythonParserSettings::default())
}

/// The Python language with explicit parser settings.
#[must_use]
pub fn linked_with_settings(settings: settings::PythonParserSettings) -> ryo::languages::Language {
    use std::sync::Arc;

    use crate::inbound::contracts::parse::contract::ParseContract;
    use crate::inbound::contracts::project::contract::ProjectContract;
    use crate::outbound::manifest_reading::factory::{pep621, pip, pipenv, poetry, uv};
    use crate::outbound::manifest_reading::interface::ManifestReadingPort;
    use crate::outbound::manifest_reading::types::AvailableManifests;
    use crate::outbound::notes::factory::postcard;
    use crate::outbound::syntax::factory::ruff_python_parser;

    /// The manifest adapters of a run over `available`, every manifest of its repository: one for each package manager.
    fn adapters(available: &[ryo::languages::ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
        vec![
            Arc::new(pep621()),
            Arc::new(uv()),
            Arc::new(poetry()),
            Arc::new(pip(AvailableManifests(available.to_vec()))),
            Arc::new(pipenv()),
        ]
    }

    ryo::link!(
        domain::language::constants::NAME,
        parse: Arc::new(ParseContract::new(
            settings,
            Arc::new(ruff_python_parser(settings)),
            Arc::new(postcard()),
        )),
        project: Arc::new(ProjectContract::new(adapters, Arc::new(postcard()), Arc::new(outbound::module_context::factory::filesystem()))),
        walk: Arc::new(walk_with_settings(settings)),
    )
}

/// The Python language's walk for the engine, composed over its parser under the default settings: what an edge that runs the
/// engine links to walk Python files into the engine's node table and fill its relations.
///
/// # Examples
///
/// ```
/// use ryo::languages::Walk;
///
/// fn kinds<W: Walk>(_: &W) -> ryo::languages::Count {
///     W::KIND_COUNT
/// }
/// assert!(kinds(&ryo_python::walk()).0 > 0);
/// ```
#[must_use]
pub fn walk() -> impl ryo::languages::Walk {
    walk_with_settings(settings::PythonParserSettings::default())
}

/// The Python language's walk for the engine with explicit parser settings.
///
/// # Examples
///
/// ```
/// use ryo_python::settings::{PythonParserSettings, PythonTargetVersion};
///
/// let _walk = ryo_python::walk_with_settings(PythonParserSettings::new(PythonTargetVersion::Py310));
/// ```
#[must_use]
pub fn walk_with_settings(settings: settings::PythonParserSettings) -> impl ryo::languages::Walk {
    use crate::domain::file_view::types::PythonWalk;
    use crate::inbound::contracts::walk::contract::WalkPorts;
    use crate::outbound::syntax::factory::ruff_python_parser;
    use std::sync::Arc;

    PythonWalk {
        ports: WalkPorts {
            settings,
            syntax: Arc::new(ruff_python_parser(settings)),
        },
    }
}

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
