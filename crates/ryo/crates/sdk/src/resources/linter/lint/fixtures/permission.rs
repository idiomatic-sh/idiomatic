//! The test data of `linter lint` that fixes permission.

use super::outcome::{Outcome, summary};
use super::rules::{ordinary, rule_id};
use super::shared_calls::shared;
use crate::domain::capacity::types::MemoryBudget;
use crate::domain::capacity::types::WorkerPool;
use crate::domain::file::types::RelativePath;
use crate::domain::language::types::Languages;
use crate::resources::linter::lint::entry::lint_with;
use crate::resources::linter::lint::types::Cache;
use crate::resources::linter::lint::types::CaseSource;
use crate::resources::linter::lint::types::Corpus;
use crate::resources::linter::lint::types::LintRequest;
use crate::resources::linter::lint::types::Linter;
use crate::resources::linter::lint::types::SourceText;
use crate::resources::linter::lint::types::Workers;
use ryo_values::domain::files::types::ExecutablePermission;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;

/// Bytes that are unchanged and a timestamp that is given never reuse a finding of an old mode.
pub(crate) fn permission_bytes() -> Outcome {
    let root = tempfile::tempdir().expect("root");
    let cache = tempfile::tempdir().expect("cache");
    let shared = permission_linter(Cache::At(cache.path().join("store")));
    let mut last = (0, 0, true, 0);
    let query = "let files = select path as file from \"filesystem.executable\" where executable = \"true\"; emit files subject file key(file);";
    for executable in [false, true, true, false, false, true] {
        let current = permission_request(
            root.path(),
            if executable {
                ExecutablePermission::PosixExecutable
            } else {
                ExecutablePermission::PosixNotExecutable
            },
            query,
        );
        let retained = lint_with(&shared, current.clone()).expect("a report");
        let fresh = lint_with(&permission_linter(Cache::None), current).expect("a report");
        assert_eq!(retained.findings, fresh.findings);
        assert_eq!(retained.findings.len(), usize::from(executable));
        assert!(retained.unknowns.is_empty());
        last = summary(&retained);
    }
    last
}

/// A permission that cannot be known is neither false nor a proven absence.
pub(crate) fn permission_unavailable() -> Outcome {
    let root = tempfile::tempdir().expect("root");
    let query = "let files = select path as file from \"filesystem.file\"; let yes = select path as file from \"filesystem.executable\" where executable = \"true\"; let absent = antijoin(left=files,right=yes,left-on=[\"file\"],right-on=[\"file\"]); emit absent subject file key(file);";
    let run = |observation| {
        lint_with(
            &permission_linter(Cache::None),
            permission_request(root.path(), observation, query),
        )
        .expect("a report")
    };
    let unknown = run(ExecutablePermission::Unknown);
    assert!(unknown.findings.is_empty());
    assert!(!unknown.unknowns.is_empty());
    let known_false = run(ExecutablePermission::PosixNotExecutable);
    assert_eq!(known_false.findings.len(), 1);
    assert!(known_false.unknowns.is_empty());
    let known_true = run(ExecutablePermission::PosixExecutable);
    assert!(known_true.findings.is_empty());
    assert!(known_true.unknowns.is_empty());
    let unsupported = run(ExecutablePermission::Unsupported);
    assert_eq!(unsupported.findings.len(), 1);
    assert!(unsupported.unknowns.is_empty());
    summary(&unsupported)
}

/// A linter with no language, retaining into `cache`.
fn permission_linter(cache: Cache) -> Linter {
    Linter::new(
        Languages::default(),
        WorkerPool::from(Workers(NonZeroUsize::MIN)),
        MemoryBudget::Machine,
        cache,
    )
}

/// A request over the one given file `case.py`, whose permission is `observation`.
fn permission_request(
    root: &std::path::Path,
    observation: ExecutablePermission,
    query: &str,
) -> LintRequest {
    LintRequest {
        project_search_scope: Default::default(),
        given_project_inventory: Default::default(),
        source_language: crate::domain::language::types::SourceLanguage::Recognized,
        root: root.to_path_buf(),
        rules: BTreeMap::from([(rule_id("permissions"), ordinary(query))]),
        files: Vec::new(),
        hosted: Vec::new(),
        corpus: Corpus::Given(vec![CaseSource {
            executable: observation,
            path: RelativePath("case.py".into()),
            text: SourceText("print(1)\n".into()),
        }]),
        beside: Vec::new(),
    }
}
