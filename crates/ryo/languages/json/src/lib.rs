#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]



pub(crate) mod inbound;
pub(crate) mod outbound;

mod domain;

/// The json language, composed over its syntax port: what an edge links to read json.
#[must_use]
pub fn linked() -> ryo::languages::Language {
    use std::sync::Arc;

    use crate::inbound::contracts::parse::contract::ParseContract;
    use crate::inbound::contracts::project::contract::ProjectContract;
    use crate::inbound::contracts::walk::contract::WalkPorts;
    use crate::domain::file_view::types::JsonWalk;
    use crate::outbound::syntax::factory::jsonc_parser;

    ryo::link!(
        domain::language::constants::NAME,
        parse: Arc::new(ParseContract::new(Arc::new(jsonc_parser()))),
        project: Arc::new(ProjectContract::new()),
        walk: Arc::new(JsonWalk { ports: WalkPorts { syntax: Arc::new(jsonc_parser()) } }),
    )
}


