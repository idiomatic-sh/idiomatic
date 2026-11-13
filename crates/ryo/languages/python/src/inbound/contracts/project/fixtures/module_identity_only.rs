//! A group of the contract's cases.
//!
//! The case `module_identity_only_preserves_inputs_without_projecting_or_validating`.

use crate::inbound::contracts::project::contract::ProjectContract;
use crate::outbound::manifest_reading::fixtures::fake::ManifestReadingFake;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::module_context::interface::ModuleContextPort;
use crate::outbound::notes::factory::postcard;
use ryo::languages;
use ryo::languages::Project as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The text of the request's one manifest: not TOML, so a projection of it would refuse.
const NOT_TOML: &str = "not TOML: projection must not be attempted";

/// How many times the readers were built for manifests: only the initial declared-name discovery, which passes none, may build them.
static PROJECTED: AtomicUsize = AtomicUsize::new(0);

/// The readers of a run that must build none for a module identity request; counts the builds that were given manifests.
fn no_projection(manifests: &[languages::ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
    if !manifests.is_empty() {
        let _previous = PROJECTED.fetch_add(1, Ordering::Relaxed);
    }
    vec![Arc::new(ManifestReadingFake)]
}

pub(crate) fn module_identity_only_preserves_inputs_without_projecting_or_validating() -> String {
    let mut seen: Vec<String> = Vec::new();
    let projected_before = PROJECTED.load(Ordering::Relaxed);
    let (probe, lookups) = ContextProbe::around(Arc::new(
        crate::outbound::module_context::factory::filesystem(),
    ));
    let contract = ProjectContract::new(no_projection, Arc::new(postcard()), Arc::new(probe));
    let request = languages::ProjectRequest {
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Default::default(),
        repository: languages::Repository::Given,
        root: languages::RootPath("project".into()),
        files: vec![languages::FilePath("module.py".into())],
        notes: vec![],
        manifests: vec![languages::ManifestFile {
            path: languages::FilePath("pyproject.toml".into()),
            text: languages::ManifestText(NOT_TOML.into()),
        }],
        wanted: vec![languages::Relation::ModuleIdentity],
    };
    let outcome = contract.resolve(&request);
    seen.push(format!("{:?}", lookups.count()));
    seen.push(format!("{:?}", lookups.manifests()));
    seen.push(format!(
        "{:?}",
        PROJECTED.load(Ordering::Relaxed) - projected_before
    ));
    match outcome {
        Ok(resolved) => {
            seen.push(format!("{:?}", resolved.refused.is_empty()));
            seen.push(format!(
                "{:?}",
                resolved.derived.packages.packages.is_empty()
            ));
            seen.push(format!(
                "{:?}",
                resolved.derived.manifest_validation.files.is_empty()
            ));
            seen.push(format!(
                "{:?}",
                resolved.derived.manifest_validation.incomplete.is_empty()
            ));
        }
        Err(error) => seen.push(format!("{error:?}")),
    }
    seen.join("|")
}

/// A module context that counts the lookups it answers, the manifests they carry, and those of them that still hold the text the
/// scenario sent.
pub(super) struct ContextProbe {
    inner: Arc<dyn ModuleContextPort>,
    seen: Arc<Seen>,
}
/// What a probe has been given.
#[derive(Default)]
pub(super) struct Seen {
    lookups: AtomicUsize,
    manifests: AtomicUsize,
    unchanged: AtomicUsize,
}
/// What a probe has been given, read after the run.
pub(super) struct Lookups(Arc<Seen>);
impl Lookups {
    pub(super) fn count(&self) -> usize {
        self.0.lookups.load(Ordering::Relaxed)
    }
    /// The manifests the lookups carried, and how many of them hold the text the scenario sent.
    pub(super) fn manifests(&self) -> (usize, usize) {
        (
            self.0.manifests.load(Ordering::Relaxed),
            self.0.unchanged.load(Ordering::Relaxed),
        )
    }
}
impl ContextProbe {
    pub(super) fn around(inner: Arc<dyn ModuleContextPort>) -> (Self, Lookups) {
        let seen = Arc::new(Seen::default());
        (
            Self {
                inner,
                seen: Arc::clone(&seen),
            },
            Lookups(seen),
        )
    }
}
impl ModuleContextPort for ContextProbe {
    fn resolve(
        &self,
        request: &languages::ProjectRequest,
    ) -> Result<crate::domain::module_context::types::ModuleContexts, languages::ProjectError> {
        let _previous = self.seen.lookups.fetch_add(1, Ordering::Relaxed);
        let _previous = self
            .seen
            .manifests
            .fetch_add(request.manifests.len(), Ordering::Relaxed);
        let unchanged = request
            .manifests
            .iter()
            .filter(|manifest| manifest.text.0.as_ref() == NOT_TOML)
            .count();
        let _previous = self.seen.unchanged.fetch_add(unchanged, Ordering::Relaxed);
        self.inner.resolve(request)
    }
}
