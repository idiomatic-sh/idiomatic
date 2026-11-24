//! The inputs the contract's rows start from.
use benchmark_contracts::domain::context::types::CaseContext;
use benchmark_contracts::domain::context::types::ContextBytes;
use benchmark_contracts::domain::context::types::ContextManifest;
use std::collections::BTreeMap;

/// The context every row starts from: one auxiliary file and one declared absence.
pub fn context(auxiliary: &str) -> CaseContext {
    CaseContext {
        logical_root: "pkg".into(),
        auxiliary_files: BTreeMap::from([(auxiliary.into(), ContextBytes(Vec::new()))]),
        missing: vec!["__init__.py".into()],
    }
}

/// The one lint source every row materializes.
pub fn sources() -> BTreeMap<ryo::linter::RelativePath, ryo::linter::SourceText> {
    BTreeMap::from([(
        ryo::linter::RelativePath("mod.py".into()),
        ryo::linter::SourceText("import pkg.mod\n".into()),
    )])
}

/// A dependency observed present, with the fingerprint of empty bytes.
pub fn present(path: &str) -> ryo::linter::ProjectDependency {
    ryo::linter::ProjectDependency {
        path: ryo::linter::RelativePath(path.into()),
        state: ryo::linter::ProjectDependencyState::Present(
            ryo::linter::DependencyFingerprint::from(ryo::linter::DependencyBytes(&[])),
        ),
    }
}

/// A dependency observed missing.
pub fn missing(path: &str) -> ryo::linter::ProjectDependency {
    ryo::linter::ProjectDependency {
        path: ryo::linter::RelativePath(path.into()),
        state: ryo::linter::ProjectDependencyState::Missing,
    }
}

/// A sealed manifest of nothing, for a context double that never reads it.
pub fn sealed_manifest() -> ContextManifest {
    ContextManifest {
        boundary: "/fixture".into(),
        logical_root: ".".into(),
        auxiliary_files: BTreeMap::new(),
        source_files: BTreeMap::new(),
        missing: Vec::new(),
        links: BTreeMap::new(),
    }
}

/// The manifest with these sealed links added, each a path and its exact target text.
#[must_use]
pub fn linked(manifest: &ContextManifest, links: &[(&str, &str)]) -> ContextManifest {
    let mut linked = manifest.clone();
    linked.links.extend(links.iter().map(|(path, target)| {
        (
            (*path).into(),
            benchmark_contracts::domain::context::types::LinkTarget((*target).into()),
        )
    }));
    linked
}

/// The sealed link every link row starts from: `pkg/alias.py` to its sealed `__init__.py`.
pub const ALIAS: (&str, &str) = ("pkg/alias.py", "__init__.py");

/// Manifests that differ from [`ALIAS`] by one part of a link's identity: its path, its target text, an equivalent
/// spelling of the same target, and one more link.
pub const LINK_VARIANTS: [&[(&str, &str)]; 4] = [
    &[("pkg/other.py", "__init__.py")],
    &[("pkg/alias.py", "mod.py")],
    &[("pkg/alias.py", "./__init__.py")],
    &[ALIAS, ("pkg/second.py", "__init__.py")],
];
