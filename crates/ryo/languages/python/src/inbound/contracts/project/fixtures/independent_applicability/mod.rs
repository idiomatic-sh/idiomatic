//! A group of the contract's cases.
//!
//! The case `metadata_rejection_and_dependency_refusal_have_independent_applicability`.

use super::support::{ReaderOutcome, pep};
use crate::cases::{Case, CaseName, Expect, Word};
use crate::inbound::contracts::project::contract::ProjectContract;
use crate::outbound::notes::factory::postcard;
use ryo::languages;
use ryo::languages::Project as _;
use std::sync::Arc;

fn metadata_rejection_and_dependency_refusal_have_independent_applicability() {
    let contract = ProjectContract::new(
        pep,
        Arc::new(postcard()),
        Arc::new(crate::outbound::module_context::factory::filesystem()),
    );
    let mut request = languages::ProjectRequest {
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Default::default(),
        repository: languages::Repository::Given,
        root: languages::RootPath("".into()),
        files: vec![languages::FilePath("pyproject.toml".into())],
        manifests: vec![languages::ManifestFile {
            path: languages::FilePath("pyproject.toml".into()),
            text: languages::ManifestText(
                "[project]\nname = 'example'\nversion = '1.0'\ndependencies = [1]\n".into(),
            ),
        }],
        notes: vec![],
        wanted: vec![
            languages::Relation::ManifestDiagnostic,
            languages::Relation::SyntaxError,
        ],
    };
    let metadata = contract
        .resolve(&request)
        .expect("metadata diagnostic is a completed observation");
    assert!(metadata.refused.is_empty());
    assert!(metadata.derived.packages.packages.is_empty());
    assert!(
        metadata.derived.packages.refused.is_empty(),
        "undemanded dependencies are not read"
    );
    assert_eq!(
        metadata.derived.manifest_validation.files[0]
            .validation
            .diagnostics
            .len(),
        1
    );
    request.wanted.push(languages::Relation::PackageDependency);
    let both = contract
        .resolve(&request)
        .expect("dependency rejection remains partial");
    assert_eq!(both.refused.len(), 1);
    assert!(both.derived.packages.packages.is_empty());
    assert_eq!(
        both.derived.manifest_validation.files[0]
            .validation
            .diagnostics
            .len(),
        1
    );
    use languages::ManifestItem as _;
    let mut symbols = languages::Symbols::default();
    let input = languages::ManifestInput {
        request: &request,
        derived: &both.derived,
    };
    let mut dependencies = ReaderOutcome::default();
    (crate::domain::manifests::collections::manifests::package_dependency::item::PackageDependencyItem::READER.read)(&input, &mut symbols, &mut dependencies);
    assert_eq!(dependencies.rows, 0);
    assert!(
        !dependencies.partial.is_empty(),
        "rejected dependency output stays unknown"
    );
    let mut diagnostics = ReaderOutcome::default();
    (crate::domain::manifests::collections::manifests::manifest_diagnostic::item::ManifestDiagnosticItem::READER.read)(&input, &mut symbols, &mut diagnostics);
    assert_eq!(diagnostics.rows, 1);
    assert!(
        diagnostics.partial.is_empty(),
        "actual schema diagnostic is completely observed"
    );
    request.wanted = vec![languages::Relation::SyntaxError];
    let unrelated = contract
        .resolve(&request)
        .expect("unrequested projection cannot create a read failure");
    assert!(unrelated.refused.is_empty());
}

/// The row of this case.
pub(super) const CASES: &[Case<(), (), ()>] = &[Case {
    name: CaseName("metadata_rejection_and_dependency_refusal_have_independent_applicability"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];

/// Runs the body of the case.
pub(super) fn subject((): ()) -> Result<(), ()> {
    metadata_rejection_and_dependency_refusal_have_independent_applicability();
    Ok(())
}
