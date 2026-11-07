//! The contracts of the language over its own ports, for the tests and benches outside the crate.

use std::sync::Arc;

use ryo::languages::{self, FilePath};

use crate::inbound::contracts::parse::contract::ParseContract;
use crate::inbound::contracts::project::contract::ProjectContract;
use crate::outbound::embedded::factory;
use crate::outbound::embedded::interface::EmbeddedPort;
use crate::outbound::manifest_reading::factory as manifest_reading;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::notes::factory::postcard;
use crate::outbound::syntax::factory::oxc_parser;

/// The `parse` contract, over the language's syntax, embedded and notes ports.
#[must_use]
pub fn parse() -> impl languages::Parse {
    ParseContract::new(Arc::new(oxc_parser()), embedded(), Arc::new(postcard()))
}

/// The embedded adapters the language reads blocks with, for the tests of the contracts over the real parser.
#[must_use]
pub(crate) fn embedded() -> Vec<Arc<dyn EmbeddedPort>> {
    vec![Arc::new(factory::vue()), Arc::new(factory::svelte())]
}

/// The `parse` contract with the vue adapter alone reading embedded blocks: what the bench of the `embedded` adapters runs.
#[must_use]
pub fn vue() -> impl languages::Parse {
    ParseContract::new(
        Arc::new(oxc_parser()),
        vec![Arc::new(factory::vue())],
        Arc::new(postcard()),
    )
}

/// The `parse` contract with the svelte adapter alone reading embedded blocks: what the bench of the `embedded` adapters runs.
#[must_use]
pub fn svelte() -> impl languages::Parse {
    ParseContract::new(
        Arc::new(oxc_parser()),
        vec![Arc::new(factory::svelte())],
        Arc::new(postcard()),
    )
}

/// The `project` contract with the npm adapter alone reading manifests, over the run's: what the bench of the `manifest_reading`
/// adapters runs.
#[must_use]
pub fn npm() -> impl languages::Project {
    /// The npm adapter of one run, over its manifests.
    fn adapters(candidates: &[FilePath]) -> Vec<Arc<dyn ManifestReadingPort>> {
        vec![Arc::new(manifest_reading::npm(candidates.to_vec()))]
    }
    ProjectContract::new(adapters, Arc::new(postcard()))
}

/// The `project` contract with the pnpm adapter alone reading manifests, over the run's: what the bench of the `manifest_reading`
/// adapters runs.
#[must_use]
pub fn pnpm() -> impl languages::Project {
    /// The pnpm adapter of one run, over its manifests.
    fn adapters(candidates: &[FilePath]) -> Vec<Arc<dyn ManifestReadingPort>> {
        vec![Arc::new(manifest_reading::pnpm(candidates.to_vec()))]
    }
    ProjectContract::new(adapters, Arc::new(postcard()))
}

/// The `project` contract with the bun adapter alone reading manifests: what the bench of the `manifest_reading` adapters runs.
#[must_use]
pub fn bun() -> impl languages::Project {
    /// The bun adapter, whatever the run.
    fn adapters(_candidates: &[FilePath]) -> Vec<Arc<dyn ManifestReadingPort>> {
        vec![Arc::new(manifest_reading::bun())]
    }
    ProjectContract::new(adapters, Arc::new(postcard()))
}

/// The `project` contract with the tsconfig adapter alone reading manifests: what the bench of the `manifest_reading` adapters runs.
#[must_use]
pub fn tsconfig() -> impl languages::Project {
    /// The tsconfig adapter, whatever the run.
    fn adapters(_candidates: &[FilePath]) -> Vec<Arc<dyn ManifestReadingPort>> {
        vec![Arc::new(manifest_reading::tsconfig())]
    }
    ProjectContract::new(adapters, Arc::new(postcard()))
}
