//! The contracts of the language over its own ports, for the tests and benches outside the crate.

use std::sync::Arc;

use ryo::languages::{self, ManifestFile};

use crate::inbound::contracts::parse::contract::ParseContract;
use crate::inbound::contracts::project::contract::ProjectContract;
use crate::outbound::manifest_reading::factory as manifest_reading;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::manifest_reading::types::AvailableManifests;
use crate::outbound::notes::factory::postcard;
use crate::outbound::syntax::factory::ruff_python_parser;

/// The `parse` contract, over the language's syntax and notes ports.
#[must_use]
pub fn parse() -> impl languages::Parse {
    ParseContract::new(
        crate::PythonParserSettings::default(),
        Arc::new(ruff_python_parser()),
        Arc::new(postcard()),
    )
}

/// The `project` contract with the pep621 adapter alone reading manifests: what the bench of the `manifest_reading` adapters runs.
#[must_use]
pub fn pep621() -> impl languages::Project {
    /// The pep621 adapter, whatever the run.
    fn adapters(_available: &[ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
        vec![Arc::new(manifest_reading::pep621())]
    }
    ProjectContract::new(adapters, Arc::new(postcard()))
}

/// The `project` contract with the uv adapter alone reading manifests: what the bench of the `manifest_reading` adapters runs.
#[must_use]
pub fn uv() -> impl languages::Project {
    /// The uv adapter, whatever the run.
    fn adapters(_available: &[ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
        vec![Arc::new(manifest_reading::uv())]
    }
    ProjectContract::new(adapters, Arc::new(postcard()))
}

/// The `project` contract with the poetry adapter alone reading manifests: what the bench of the `manifest_reading` adapters runs.
#[must_use]
pub fn poetry() -> impl languages::Project {
    /// The poetry adapter, whatever the run.
    fn adapters(_available: &[ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
        vec![Arc::new(manifest_reading::poetry())]
    }
    ProjectContract::new(adapters, Arc::new(postcard()))
}

/// The `project` contract with the pip adapter alone reading manifests, over the run's: what the bench of the `manifest_reading`
/// adapters runs.
#[must_use]
pub fn pip() -> impl languages::Project {
    /// The pip adapter of one run, over its manifests.
    fn adapters(available: &[ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
        vec![Arc::new(manifest_reading::pip(AvailableManifests(
            available.to_vec(),
        )))]
    }
    ProjectContract::new(adapters, Arc::new(postcard()))
}

/// The `project` contract with the pipenv adapter alone reading manifests: what the bench of the `manifest_reading` adapters runs.
#[must_use]
pub fn pipenv() -> impl languages::Project {
    /// The pipenv adapter, whatever the run.
    fn adapters(_available: &[ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
        vec![Arc::new(manifest_reading::pipenv())]
    }
    ProjectContract::new(adapters, Arc::new(postcard()))
}
