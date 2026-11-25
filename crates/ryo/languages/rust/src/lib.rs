#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]



pub(crate) mod inbound;
pub(crate) mod outbound;

mod domain;

/// The Rust language, composed over its ports: what an edge links to read Rust.
#[must_use]
pub fn linked() -> ryo::languages::Language {
    linked_revision(domain::model::types::ProviderRevision::Current)
}

/// The 0.1.4 Rust provider's facts, delivered through the native reader.
#[must_use]
pub fn linked_0_1_4() -> ryo::languages::Language {
    linked_revision(domain::model::types::ProviderRevision::Released014)
}

fn linked_revision(revision: domain::model::types::ProviderRevision) -> ryo::languages::Language {
    use std::sync::Arc;

    use ryo::languages::RootPath;

    use crate::domain::syntax::constants::OPAQUE_MACROS;
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
    use crate::outbound::syntax::types::OpaqueMacros;

    ryo::link!(
        domain::language::constants::NAME,
        parse: Arc::new(ParseContract::new(
            Arc::new(ra_ap_syntax(OpaqueMacros(&OPAQUE_MACROS)).with_revision(revision)),
            Arc::new(postcard()),
        ).with_revision(revision)),
        project: Arc::new(ProjectContract::new(
            Arc::new(toml()),
            Arc::new(|root: &RootPath| -> Box<dyn RepositoryPort> { Box::new(native(root.clone())) }),
            Arc::new(globset()),
            Arc::new(rayon()),
            Arc::new(cargo_platform()),
            Arc::new(postcard()),
        )),
        walk: Arc::new(walk_over(revision)),
    )
}

/// The Rust language's walk for the engine, composed over its parser: what an edge that runs the engine links to walk Rust files
/// into the engine's node table and fill its relations.
///
/// # Examples
///
/// ```
/// use ryo::languages::Walk;
///
/// fn kinds<W: Walk>(_: &W) -> ryo::languages::Count {
///     W::KIND_COUNT
/// }
/// assert!(kinds(&ryo_rust::walk()).0 > 0);
/// ```
#[must_use]
pub fn walk() -> impl ryo::languages::Walk {
    walk_over(domain::model::types::ProviderRevision::Current)
}

fn walk_over(revision: domain::model::types::ProviderRevision) -> impl ryo::languages::Walk {
    use std::sync::Arc;

    use crate::domain::file_view::types::RustWalk;
    use crate::domain::syntax::constants::OPAQUE_MACROS;
    use crate::inbound::contracts::walk::contract::WalkPorts;
    use crate::outbound::syntax::factory::ra_ap_syntax;
    use crate::outbound::syntax::types::OpaqueMacros;

    RustWalk {
        ports: WalkPorts {
            syntax: Arc::new(ra_ap_syntax(OpaqueMacros(&OPAQUE_MACROS)).with_revision(revision)),
        },
    }
}


