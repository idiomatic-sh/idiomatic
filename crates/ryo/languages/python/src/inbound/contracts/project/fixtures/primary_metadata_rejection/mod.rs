//! A group of the contract's cases.
//!
//! The case `primary_metadata_rejection_keeps_real_dependency_projection`.

use super::support::pep;
use crate::cases::{Case, CaseName, Expect, Word};
use crate::inbound::contracts::project::contract::ProjectContract;
use crate::outbound::notes::factory::postcard;
use ryo::languages;
use ryo::languages::Project as _;
use std::sync::Arc;

fn primary_metadata_rejection_keeps_real_dependency_projection() {
    let contract = ProjectContract::new(
        pep,
        Arc::new(postcard()),
        Arc::new(crate::outbound::module_context::factory::filesystem()),
    );
    let text = "[project]\nname = 'example'\nversion = '1.0'\nauthors = [{email = 1}]\ndependencies = ['requests>=2']\n";
    let request = languages::ProjectRequest {
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Default::default(),
        repository: languages::Repository::Given,
        root: languages::RootPath("".into()),
        files: vec![languages::FilePath("pyproject.toml".into())],
        manifests: vec![languages::ManifestFile {
            path: languages::FilePath("pyproject.toml".into()),
            text: languages::ManifestText(text.into()),
        }],
        notes: vec![],
        wanted: vec![
            languages::Relation::ManifestDiagnostic,
            languages::Relation::PackageDependency,
        ],
    };
    let resolved = contract
        .resolve(&request)
        .expect("pure manifest processing");
    assert!(resolved.refused.is_empty());
    assert_eq!(resolved.derived.packages.packages.len(), 1);
    assert_eq!(
        resolved.derived.packages.packages[0].declared.requirements[0]
            .written
            .0
            .as_ref(),
        "requests>=2"
    );
    assert_eq!(
        resolved.derived.manifest_validation.files[0].file,
        languages::FileId(0)
    );
    assert_eq!(
        resolved.derived.manifest_validation.files[0]
            .validation
            .diagnostics
            .len(),
        1
    );
    assert!(resolved.derived.manifest_validation.incomplete.is_empty());
}

/// The row of this case.
pub(crate) const CASES: &[Case<(), (), ()>] = &[Case {
    name: CaseName("primary_metadata_rejection_keeps_real_dependency_projection"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];

/// Runs the body of the case.
pub(crate) fn subject((): ()) -> Result<(), ()> {
    primary_metadata_rejection_keeps_real_dependency_projection();
    Ok(())
}
