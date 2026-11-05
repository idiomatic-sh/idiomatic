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
            Arc::new(ruff_python_parser()),
            Arc::new(postcard()),
        )),
        project: Arc::new(ProjectContract::new(adapters, Arc::new(postcard()), Arc::new(outbound::module_context::factory::filesystem()))),
    )
}

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
