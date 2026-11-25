#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]



pub(crate) mod inbound;
pub(crate) mod outbound;

mod domain;

/// The go language, composed over its ports: what an edge links to read go.
#[must_use]
pub fn linked() -> ryo::languages::Language {
    use std::sync::Arc;

    use crate::inbound::contracts::parse::contract::ParseContract;
    use crate::inbound::contracts::project::contract::ProjectContract;
    use crate::outbound::manifest_reading::factory::gomod;
    use crate::outbound::notes::factory::postcard;
    use crate::domain::file_view::types::GoWalk;
    use crate::inbound::contracts::walk::contract::WalkPorts;
    use crate::outbound::syntax::factory::tree_sitter_go;

    ryo::link!(
        domain::language::constants::NAME,
        parse: Arc::new(ParseContract::new(Arc::new(tree_sitter_go()), Arc::new(postcard()))),
        project: Arc::new(ProjectContract::new(Arc::new(gomod()), Arc::new(postcard()))),
        walk: Arc::new(GoWalk { ports: WalkPorts { syntax: Arc::new(tree_sitter_go()) } }),
    )
}


