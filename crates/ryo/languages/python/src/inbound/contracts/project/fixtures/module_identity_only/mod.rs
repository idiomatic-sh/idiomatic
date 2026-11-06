//! A group of the contract's cases.
//!
//! The case `module_identity_only_preserves_inputs_without_projecting_or_validating`.

use crate::cases::{Case, CaseName, Expect, Word};
use crate::inbound::contracts::project::contract::ProjectContract;
use crate::inbound::contracts::project::contract_test::ContextProbe;
use crate::outbound::manifest_reading::fixtures::fake::ManifestReadingFake;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::notes::factory::postcard;
use ryo::languages;
use ryo::languages::Project as _;
use std::sync::Arc;

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
