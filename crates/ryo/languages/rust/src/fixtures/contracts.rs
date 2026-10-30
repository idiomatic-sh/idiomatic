//! The contracts of the language over its own ports, for the tests outside the crate: each answers the use case an edge links.

use std::sync::Arc;

use ryo::languages::{self, RootPath};

use crate::domain::syntax::constants::EXPANDED_MACROS;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::inbound::contracts::project::contract::ProjectContract;
use crate::outbound::conditions::factory::cargo_platform;
use crate::outbound::job_scheduling::factory::rayon;
use crate::outbound::manifest::factory::toml;
use crate::outbound::pattern_matching::factory::globset;
use crate::outbound::repository::factory::native;
use crate::outbound::repository::interface::RepositoryPort;
use crate::outbound::syntax::factory::ra_ap_syntax;
use crate::outbound::syntax::types::Expanding;

/// The `parse` contract, over the language's syntax port.
#[must_use]
pub fn parse() -> impl languages::Parse {
    ParseContract::new(
        Arc::new(ra_ap_syntax(Expanding(&EXPANDED_MACROS))),
        Arc::new(crate::outbound::notes::factory::postcard()),
    )
}

/// The `project` contract, over the language's manifest, repository and `pattern_matching` ports.
#[must_use]
pub fn project() -> impl languages::Project {
    ProjectContract::new(
        Arc::new(toml()),
        Arc::new(|root: &RootPath| -> Box<dyn RepositoryPort> { Box::new(native(root.clone())) }),
        Arc::new(globset()),
        Arc::new(rayon()),
        Arc::new(cargo_platform()),
        Arc::new(crate::outbound::notes::factory::postcard()),
    )
}
