//! A group of the contract's cases.
//!
//! The case `module_identity_only_preserves_inputs_without_projecting_or_validating`.

use crate::cases::{Case, CaseName, Expect, Word};
use crate::inbound::contracts::project::contract::ProjectContract;
use crate::outbound::manifest_reading::fixtures::fake::ManifestReadingFake;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::module_context::interface::ModuleContextPort;
use crate::outbound::notes::factory::postcard;
use ryo::languages;
use ryo::languages::Project as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The readers of a run that must build none for a module identity request.
fn no_projection(manifests: &[languages::ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
    assert!(
        manifests.is_empty(),
        "only initial declared-name discovery may build these readers"
    );
    vec![Arc::new(ManifestReadingFake)]
}

fn module_identity_only_preserves_inputs_without_projecting_or_validating() {
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
            text: languages::ManifestText("not TOML: projection must not be attempted".into()),
        }],
        wanted: vec![languages::Relation::ModuleIdentity],
    };
    let resolved = contract
        .resolve(&request)
        .expect("module context owns its original complete request");
    assert_eq!(lookups.count(), 1);
    assert!(resolved.refused.is_empty());
    assert!(resolved.derived.packages.packages.is_empty());
    assert!(resolved.derived.manifest_validation.files.is_empty());
    assert!(resolved.derived.manifest_validation.incomplete.is_empty());
}

/// The row of this case.
pub(crate) const CASES: &[Case<(), (), ()>] = &[Case {
    name: CaseName("module_identity_only_preserves_inputs_without_projecting_or_validating"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];

/// Runs the body of the case.
pub(crate) fn subject((): ()) -> Result<(), ()> {
    module_identity_only_preserves_inputs_without_projecting_or_validating();
    Ok(())
}

/// A module context that counts the lookups it answers and checks the request it is given.
pub(super) struct ContextProbe {
    inner: Arc<dyn ModuleContextPort>,
    seen: Arc<AtomicUsize>,
}
/// How many lookups a probe has answered.
pub(super) struct Lookups(Arc<AtomicUsize>);
impl Lookups {
    pub(super) fn count(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}
impl ContextProbe {
    pub(super) fn around(inner: Arc<dyn ModuleContextPort>) -> (Self, Lookups) {
        let seen = Arc::new(AtomicUsize::new(0));
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
        assert_eq!(request.manifests.len(), 1);
        assert_eq!(
            request.manifests[0].text.0.as_ref(),
            "not TOML: projection must not be attempted"
        );
        self.seen.fetch_add(1, Ordering::Relaxed);
        self.inner.resolve(request)
    }
}
