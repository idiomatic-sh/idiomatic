//! javascript's `manifest_reading` workload.
//!
//! The `manifest_reading` hex's adapters back to back: the javascript language with each manifest reading adapter alone resolves the
//! workspace fixture's manifests, and beside them a pnpm workspace, so the fastest is chosen by measurement.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot read its own input has nothing to measure: the panic is the failure"
)]

use ryo::languages::{self, contract};
use ryo_javascript::fixtures::contracts::{bun, npm, pnpm};

/// The fixture project the manifests are read from, below the crate's directory.
const WORKSPACE: &str = "tests/fixtures/manifests/workspace";

/// The manifests the fixture project lacks, each with its path and its text: one for each adapter that reads none of its files.
const EXTRAS: &[(&str, &str)] = &[(
    "pnpm-workspace.yaml",
    "packages:\n  - \"packages/*\"\ncatalog:\n  lodash: ^4.17.21\n",
)];

/// The manifest reading adapters the factory builds.
#[derive(Clone, Copy, Debug)]
enum Variant {
    /// npm, and Yarn.
    Npm,
    /// pnpm workspaces.
    Pnpm,
    /// Bun catalogs.
    Bun,
}

impl Variant {
    /// Every adapter, run on the same manifests.
    const ALL: [Self; 3] = [Self::Npm, Self::Pnpm, Self::Bun];
}

/// Every file below `directory`, with its path relative to `root`, into `manifests`.
fn walked(
    root: &std::path::Path,
    directory: &std::path::Path,
    manifests: &mut Vec<languages::ManifestFile>,
) {
    for entry in std::fs::read_dir(directory).expect("the fixture directory is read") {
        let path = entry.expect("the fixture entry is read").path();
        if path.is_dir() {
            walked(root, &path, manifests);
        } else {
            let relative = path
                .strip_prefix(root)
                .expect("the file is below the fixture")
                .display()
                .to_string();
            manifests.push(languages::ManifestFile {
                path: languages::FilePath(relative.into()),
                text: languages::ManifestText(
                    std::fs::read_to_string(&path)
                        .expect("the fixture file is read")
                        .into(),
                ),
            });
        }
    }
}

/// The request every variant resolves: the fixture project's files and the extra manifests, given, every relation asked for.
fn request() -> languages::ProjectRequest {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(WORKSPACE);
    let mut manifests = Vec::new();
    walked(&root, &root, &mut manifests);
    manifests.extend(EXTRAS.iter().map(|(path, text)| languages::ManifestFile {
        path: languages::FilePath((*path).into()),
        text: languages::ManifestText((*text).into()),
    }));
    languages::ProjectRequest {
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Vec::new(),
        root: languages::RootPath(root.display().to_string().into()),
        repository: languages::Repository::Given,
        manifests,
        files: Vec::new(),
        notes: Vec::new(),
        wanted: languages::all().to_vec(),
    }
}

/// Resolves `request` through `subject` on each iteration, writing the rows of the manifest relations.
fn resolve(
    bencher: divan::Bencher<'_, '_>,
    subject: &impl languages::Project,
    request: &languages::ProjectRequest,
) {
    let (mut symbols, mut out) = (languages::Symbols::default(), contract::Collect::default());
    bencher.bench_local(|| {
        out.clear();
        contract::read_project(subject, request, &mut symbols, &mut out)
            .expect("the project is resolved")
    });
}

/// Resolves the manifests with the language whose manifests only the variant's adapter reads.
#[divan::bench(args = Variant::ALL)]
fn workspace_manifests(bencher: divan::Bencher<'_, '_>, variant: Variant) {
    let request = request();
    match variant {
        Variant::Npm => resolve(bencher, &npm(), &request),
        Variant::Pnpm => resolve(bencher, &pnpm(), &request),
        Variant::Bun => resolve(bencher, &bun(), &request),
    }
}
