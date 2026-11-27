#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]
#![cfg_attr(
    all(feature = "fixtures", not(test)),
    allow(
        dead_code,
        reason = "the ports' own fixtures are used by the tests of the crate; outside tests the feature surface is `fixtures` at the root"
    )
)]

pub(crate) mod inbound;
pub(crate) mod outbound;

mod domain;

/// The typescript language, composed over its ports: what an edge links to read typescript.
#[must_use]
pub fn linked() -> ryo::languages::Language {
    use std::sync::Arc;

    use crate::inbound::contracts::parse::contract::ParseContract;
    use crate::inbound::contracts::project::contract::ProjectContract;
    use crate::outbound::manifest_reading::interface::ManifestReadingPort;
    use crate::outbound::notes::factory::postcard;
    use crate::outbound::syntax::factory::oxc_parser;

    /// The manifest adapters of one run, over the manifests of that run, which the project contract builds for each.
    fn project_adapters(
        candidates: &[ryo::languages::FilePath],
    ) -> Vec<Arc<dyn ManifestReadingPort>> {
        vec![
            Arc::new(outbound::manifest_reading::factory::npm(
                candidates.to_vec(),
            )),
            Arc::new(outbound::manifest_reading::factory::pnpm(
                candidates.to_vec(),
            )),
            Arc::new(outbound::manifest_reading::factory::bun()),
            Arc::new(outbound::manifest_reading::factory::tsconfig()),
        ]
    }

    ryo::link!(
        domain::language::constants::NAME,
        parse: Arc::new(ParseContract::new(
            Arc::new(oxc_parser()),
            vec![
                Arc::new(outbound::embedded::factory::vue()),
                Arc::new(outbound::embedded::factory::svelte()),
            ],
        )),
        project: Arc::new(ProjectContract::new(project_adapters, Arc::new(postcard()))),
        walk: Arc::new(walk()),
    )
}

/// The typescript language's walk for the engine, composed over its parser and its embedded adapters: what an edge that runs the
/// engine links to walk typescript and javascript files into the engine's node table and fill its relations.
#[must_use]
#[cfg(any(test, feature = "fixtures"))]
pub(crate) fn walk() -> impl ryo::languages::Walk {
    use std::sync::Arc;

    use crate::domain::file_view::types::TypeScriptWalk;
    use crate::inbound::contracts::walk::contract::WalkPorts;
    use crate::outbound::syntax::factory::oxc_parser;

    TypeScriptWalk {
        ports: WalkPorts {
            syntax: Arc::new(oxc_parser()),
            embedded: vec![
                Arc::new(outbound::embedded::factory::vue()),
                Arc::new(outbound::embedded::factory::svelte()),
            ],
        },
    }
}

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
