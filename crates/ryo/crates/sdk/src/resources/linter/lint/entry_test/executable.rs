//! Actual SDK/cache and common absence-guard routes for edge-observed permissions.
use super::super::{
    entry::lint_with,
    types::{ExecutablePermission, Linter},
};
use super::*;
use crate::domain::capacity::types::WorkerPool;

fn request(root: &std::path::Path, observation: ExecutablePermission, query: &str) -> LintRequest {
    LintRequest {
        project_search_scope: Default::default(),
        given_project_inventory: Default::default(),
        source_language: None,
        root: root.to_path_buf(),
        rules: BTreeMap::from([(
            RuleId::try_from("permissions").unwrap(),
            Rule {
                observations: Vec::new(),
                query: crate::resources::query::RyoQlQuery::try_from(query).unwrap(),
                applicability: crate::resources::linter::RuleApplicability::Always,
                parameters: BTreeMap::new(),
                include: Vec::new(),
                exclude: Vec::new(),
            },
        )]),
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
fn linter(cache: Cache) -> Linter {
    Linter::new(
        Languages::default(),
        WorkerPool::from(Workers(NonZeroUsize::MIN)),
        MemoryBudget::Machine,
        cache,
    )
}

#[test]
fn unchanged_bytes_and_given_timestamp_do_not_reuse_old_mode_findings() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let shared = linter(Cache::At(cache.path().join("store")));
    let query = "let files = select path as file from \"filesystem.executable\" where executable = \"true\"; emit files subject file key(file);";
    for executable in [false, true, true, false, false, true] {
        let current = request(root.path(), if executable {
                ExecutablePermission::PosixExecutable
            } else {
                ExecutablePermission::PosixNotExecutable
            }, query);
        let retained = lint_with(&shared, current.clone()).unwrap();
        let fresh = lint_with(&linter(Cache::None), current).unwrap();
        assert_eq!(retained.findings, fresh.findings);
        assert_eq!(retained.findings.len(), usize::from(executable));
        assert!(retained.unknowns.is_empty());
    }
}

#[test]
fn unavailable_permission_cannot_be_used_as_false_or_as_proven_absence() {
    let root = tempfile::tempdir().unwrap();
    let query = "let files = select path as file from \"filesystem.file\"; let yes = select path as file from \"filesystem.executable\" where executable = \"true\"; let absent = antijoin(left=files,right=yes,left-on=[\"file\"],right-on=[\"file\"]); emit absent subject file key(file);";
    let run = |observation| {
        lint_with(
            &linter(Cache::None),
            request(root.path(), observation, query),
        )
        .unwrap()
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
}
