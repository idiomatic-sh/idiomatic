//! A group of the contract's cases.
//!
//! The case `metadata_rejection_and_dependency_refusal_have_independent_applicability`.

use super::support::{ReaderOutcome, pep};
use crate::inbound::contracts::project::contract::ProjectContract;
use crate::outbound::notes::factory::postcard;
use ryo::languages;
use ryo::languages::Project as _;
use std::sync::Arc;

pub(crate) fn metadata_rejection_and_dependency_refusal_have_independent_applicability() -> String {
    let mut seen: Vec<String> = Vec::new();
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
    match contract.resolve(&request) {
        Ok(metadata) => {
            seen.push(format!("{:?}", metadata.refused.is_empty()));
            seen.push(format!(
                "{:?}",
                metadata.derived.packages.packages.is_empty()
            ));
            seen.push(format!(
                "{:?}",
                metadata.derived.packages.refused.is_empty()
            ));
            seen.push(format!(
                "{:?}",
                metadata.derived.manifest_validation.files[0]
                    .validation
                    .diagnostics
                    .len()
            ));
        }
        Err(error) => seen.push(format!("{error:?}")),
    }
    request.wanted.push(languages::Relation::PackageDependency);
    match contract.resolve(&request) {
        Ok(both) => {
            seen.push(format!("{:?}", both.refused.len()));
            seen.push(format!("{:?}", both.derived.packages.packages.is_empty()));
            seen.push(format!(
                "{:?}",
                both.derived.manifest_validation.files[0]
                    .validation
                    .diagnostics
                    .len()
            ));
            use languages::ManifestItem as _;
            let mut symbols = languages::Symbols::default();
            let input = languages::ManifestInput {
                request: &request,
                derived: &both.derived,
            };
            let mut dependencies = ReaderOutcome::default();
            (crate::domain::manifests::collections::manifests::package_dependency::item::PackageDependencyItem::READER.read)(&input, &mut symbols, &mut dependencies);
            seen.push(format!("{:?}", dependencies.rows));
            seen.push(format!("{:?}", !dependencies.partial.is_empty()));
            let mut diagnostics = ReaderOutcome::default();
            (crate::domain::manifests::collections::manifests::manifest_diagnostic::item::ManifestDiagnosticItem::READER.read)(&input, &mut symbols, &mut diagnostics);
            seen.push(format!("{:?}", diagnostics.rows));
            seen.push(format!("{:?}", diagnostics.partial.is_empty()));
        }
        Err(error) => seen.push(format!("{error:?}")),
    }
    request.wanted = vec![languages::Relation::SyntaxError];
    match contract.resolve(&request) {
        Ok(unrelated) => {
            seen.push(format!("{:?}", unrelated.refused.is_empty()));
        }
        Err(error) => seen.push(format!("{error:?}")),
    }
    seen.join("|")
}
