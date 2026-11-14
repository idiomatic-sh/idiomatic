//! The inputs the contract's rows start from.
use ryo_benchmark_contracts::domain::context::types::CaseContext;
use ryo_benchmark_contracts::domain::context::types::ContextBytes;
use ryo_benchmark_contracts::domain::context::types::ContextManifest;
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
    }
}
