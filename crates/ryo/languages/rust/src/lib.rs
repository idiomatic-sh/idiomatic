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

mod domain;

/// The Rust language, composed over its ports: what an edge links to read Rust.
#[must_use]
pub fn linked() -> ryo::languages::Language {
    use std::sync::Arc;

    use ryo::languages::RootPath;

    use crate::domain::syntax::constants::EXPANDED_MACROS;
    use crate::inbound::contracts::parse::contract::ParseContract;
    use crate::inbound::contracts::project::contract::ProjectContract;
    use crate::outbound::conditions::factory::cargo_platform;
    use crate::outbound::job_scheduling::factory::rayon;
    use crate::outbound::manifest::factory::toml;
    use crate::outbound::notes::factory::postcard;
    use crate::outbound::pattern_matching::factory::globset;
    use crate::outbound::repository::factory::native;
    use crate::outbound::repository::interface::RepositoryPort;
    use crate::outbound::syntax::factory::ra_ap_syntax;
    use crate::outbound::syntax::types::Expanding;

    ryo::link!(
        domain::language::constants::NAME,
        parse: Arc::new(ParseContract::new(
            Arc::new(ra_ap_syntax(Expanding(&EXPANDED_MACROS))),
            Arc::new(postcard()),
        )),
        project: Arc::new(ProjectContract::new(
            Arc::new(toml()),
            Arc::new(|root: &RootPath| -> Box<dyn RepositoryPort> { Box::new(native(root.clone())) }),
            Arc::new(globset()),
            Arc::new(rayon()),
            Arc::new(cargo_platform()),
            Arc::new(postcard()),
        )),
    )
}

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
