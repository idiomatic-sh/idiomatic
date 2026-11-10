//! The test data of `linter lint` that fixes shared.

use super::outcome::Outcome;
use super::queries::HOSTED;
use crate::domain::capacity::types::MemoryBudget;
use crate::domain::cell::types::Cell;
use crate::domain::cell::types::HostText;
use crate::domain::file::types::RelativePath;
use crate::domain::host::types::HostColumn;
use crate::domain::host::types::HostColumnName;
use crate::domain::host::types::HostKind;
use crate::domain::host::types::HostRelationName;
use crate::domain::language::types::Languages;
use crate::resources::linter::lint::types::Cache;
use crate::resources::linter::lint::types::CaseSource;
use crate::resources::linter::lint::types::Corpus;
use crate::resources::linter::lint::types::HostRelation;
use crate::resources::linter::lint::types::HostRow;
use crate::resources::linter::lint::types::LintRequest;
use crate::resources::linter::lint::types::Rule;
use crate::resources::linter::lint::types::RuleId;
use crate::resources::linter::lint::types::SourceText;
use crate::resources::linter::lint::types::Workers;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;

/// The two calls the shared-linter row tells apart: one rule id, two queries, two declarations of the host, two sets of globs and
/// two sets of files given as text.
fn shared_calls(root: &std::path::Path) -> [LintRequest; 2] {
    let rule = |query: &str, include: &str| Rule {
        observations: Vec::new(),
        query: crate::resources::query::RyoQlQuery::try_from(query).expect("a query"),
        applicability: crate::resources::linter::RuleApplicability::Always,
        parameters: BTreeMap::new(),
        include: vec![
            crate::resources::linter::lint::types::Glob::try_from(include).expect("a glob"),
        ],
        exclude: Vec::new(),
    };
    let placed = |file: &str, spec: &str| HostRelation {
        name: HostRelationName("host.placed".into()),
        columns: vec![
            HostColumn {
                name: HostColumnName("path".into()),
                kind: HostKind::File,
            },
            HostColumn {
                name: HostColumnName("spec".into()),
                kind: HostKind::Text,
            },
        ],
        rows: vec![HostRow(vec![
            Cell::File(RelativePath(file.into())),
            Cell::Text(HostText(spec.into())),
        ])],
    };
    let given = |file: &str| {
        Corpus::Given(vec![CaseSource {
            executable: Default::default(),
            path: RelativePath(file.into()),
            text: SourceText("text".into()),
        }])
    };
    let request = |query: &str, include: &str, file: &str, spec: &str| LintRequest {
        project_search_scope: Default::default(),
        given_project_inventory: Default::default(),
        source_language: crate::domain::language::types::SourceLanguage::Recognized,
        root: root.to_path_buf(),
        rules: BTreeMap::from([(
            RuleId::try_from("rule").expect("a rule id"),
            rule(query, include),
        )]),
        files: Vec::new(),
        hosted: vec![placed(file, spec)],
        corpus: given(file),
        beside: Vec::new(),
    };
    let other = "let placed = select * from \"host.placed\"; emit placed subject \"path\" key(\"path\", \"spec\");";
    [
        request(HOSTED, "*.txt", "from-memory.txt", "docs"),
        request(other, "*.md", "other.md", "notes"),
    ]
}

/// Two different calls on one linter, one after the other, interleaved and in parallel, give exactly what the same calls give on a
/// fresh linter each: nothing a call leaves in the linter reaches another. The value is the first call's, over a directory holding
/// one file.
pub(crate) fn shared() -> Outcome {
    let root = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(root.path().join("notes.txt"), "text").expect("a file");
    let linter = || {
        crate::resources::linter::lint::types::Linter::new(
            Languages::default(),
            crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
            MemoryBudget::Machine,
            Cache::None,
        )
    };
    let fresh = |request: LintRequest| {
        crate::resources::linter::lint::entry::lint_with(&linter(), request).expect("a report")
    };
    let [first, second] = shared_calls(root.path());
    let expected = [fresh(first.clone()), fresh(second.clone())];
    assert_ne!(
        expected[0], expected[1],
        "the two calls tell different things"
    );
    let shared = linter();
    let call = |request: &LintRequest| {
        crate::resources::linter::lint::entry::lint_with(&shared, request.clone())
            .expect("a report")
    };
    for round in 0_u8..3 {
        assert_eq!(call(&first), expected[0], "round {round}, first");
        assert_eq!(call(&second), expected[1], "round {round}, second");
        assert_eq!(call(&second), expected[1], "round {round}, second again");
        assert_eq!(call(&first), expected[0], "round {round}, first again");
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0_usize..16)
            .map(|each| {
                let (request, want) = if each % 2_usize == 0 {
                    (&first, &expected[0])
                } else {
                    (&second, &expected[1])
                };
                let call = &call;
                scope.spawn(move || assert_eq!(&call(request), want, "parallel call {each}"))
            })
            .collect();
        for handle in handles {
            handle.join().expect("a call that did not panic");
        }
    });
    let untouched = std::fs::read_dir(root.path()).is_ok_and(|entries| entries.count() == 1);
    let columns = expected[0]
        .findings
        .first()
        .map_or(0, |finding| finding.columns.len());
    (
        expected[0].findings.len(),
        expected[0].unread.len(),
        untouched,
        columns,
    )
}
