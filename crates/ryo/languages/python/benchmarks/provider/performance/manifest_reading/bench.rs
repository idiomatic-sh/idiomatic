//! python's `manifest_reading` workload.
//!
//! The `manifest_reading` hex's adapters back to back: the python language with each manifest reading adapter alone resolves the
//! workspace fixture's manifests, and beside them a Pipfile and a Poetry project, so the fastest is chosen by measurement.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot read its own input has nothing to measure: the panic is the failure"
)]

use ryo::languages::{self, contract};
use ryo_python::fixtures::contracts::{pep621, pip, pipenv, poetry, uv};

/// The fixture project the manifests are read from, below the crate's directory.
const WORKSPACE: &str = "tests/fixtures/manifests/workspace";

/// The manifests the fixture project lacks, each with its path and its text: one for each adapter that reads none of its files.
const EXTRAS: &[(&str, &str)] = &[
    (
        "Pipfile",
        "[packages]\nrequests = \"*\"\n\n[dev-packages]\npytest = \">=7\"\n",
    ),
    (
        "poetry/pyproject.toml",
        "[tool.poetry]\nname = \"poetry-app\"\n\n[tool.poetry.dependencies]\npython = \"^3.9\"\nrequests = \"^2.32\"\n\n[tool.poetry.group.dev.dependencies]\npytest = \"^8\"\n",
    ),
];

/// The manifest reading adapters the factory builds.
#[derive(Clone, Copy, Debug)]
enum Variant {
    /// `[project]` and the dependency groups of `pyproject.toml`.
    Pep621,
    /// The workspace and the sources uv reads of `pyproject.toml`.
    Uv,
    /// The tables of `pyproject.toml` Poetry reads.
    Poetry,
    /// Requirements files.
    Pip,
    /// The `Pipfile`.
    Pipenv,
}

impl Variant {
    /// Every adapter, run on the same manifests.
    const ALL: [Self; 5] = [
        Self::Pep621,
        Self::Uv,
        Self::Poetry,
        Self::Pip,
        Self::Pipenv,
    ];
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
        Variant::Pep621 => resolve(bencher, &pep621(), &request),
        Variant::Uv => resolve(bencher, &uv(), &request),
        Variant::Poetry => resolve(bencher, &poetry(), &request),
        Variant::Pip => resolve(bencher, &pip(), &request),
        Variant::Pipenv => resolve(bencher, &pipenv(), &request),
    }
}
