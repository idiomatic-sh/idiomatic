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

/// The javascript language, composed over its ports: what an edge links to read javascript.
#[must_use]
pub fn linked() -> ryo::languages::Language {
    use std::sync::Arc;

    use crate::inbound::contracts::parse::contract::ParseContract;
    use crate::inbound::contracts::project::contract::ProjectContract;
    use crate::outbound::notes::factory::postcard;
    use crate::outbound::syntax::factory::oxc_parser;

    ryo::link!(
        domain::language::constants::NAME,
        parse: Arc::new(ParseContract::new(Arc::new(oxc_parser()), vec![
                Arc::new(outbound::embedded::factory::vue()),
                Arc::new(outbound::embedded::factory::svelte()),
            ], Arc::new(postcard()))),
        project: Arc::new(ProjectContract::new(vec![
            Arc::new(outbound::manifest_reading::factory::npm()),
            Arc::new(outbound::manifest_reading::factory::pnpm()),
            Arc::new(outbound::manifest_reading::factory::bun()),
        ], Arc::new(postcard()))),
    )
}

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
