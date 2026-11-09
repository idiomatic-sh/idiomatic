//! The test data of `linter lint` that fixes configured scope.

use super::capability_provider::provider_linter;
use super::outcome::{Outcome, summary};
use super::rules::{listed, ordinary, rule_id};
use crate::domain::cell::types::Cell;
use crate::domain::cell::types::HostText;
use crate::domain::cell::types::Slot;
use crate::domain::file::types::RelativePath;
use crate::domain::host::types::HostColumn;
use crate::domain::host::types::HostColumnName;
use crate::domain::host::types::HostKind;
use crate::domain::host::types::HostRelationName;
use crate::resources::linter::lint::entry::lint_with;
use crate::resources::linter::lint::types::Cache;
use crate::resources::linter::lint::types::Corpus;
use crate::resources::linter::lint::types::Glob;
use crate::resources::linter::lint::types::HostRelation;
use crate::resources::linter::lint::types::HostRow;
use std::collections::BTreeMap;

/// A rule's per-file exclusion and the hosted rows it joins change what a retained store answers, run after run, as they change what
/// a fresh linter answers, and the run leaves the repository's files as they were.
pub(crate) fn configured_scope() -> Outcome {
    let root = tempfile::tempdir().expect("corpus");
    std::fs::create_dir_all(root.path().join("pkg")).expect("package");
    let files = ["case.txt", "pkg/case.txt"];
    for file in files {
        std::fs::write(root.path().join(file), "x").expect("source");
    }
    let source = root.path().join(files[0]);
    let modified = std::fs::metadata(&source)
        .expect("metadata")
        .modified()
        .expect("modified");
    let store = tempfile::tempdir().expect("store");
    let engine = |cache| provider_linter("same-build", "configured-scope", &[], "unused", cache);
    let request = |prefix: &str, exclude: bool| {
        let mut rule = ordinary(
            "let files = select path from \"filesystem.file\"; let external = select prefix from \"config.external\" where prefix = \"V\"; let found = join(left=files, right=external, left-on=[], right-on=[], output={path:[left,path]}); emit found subject path key(path);",
        );
        if exclude {
            rule.exclude.push(Glob::try_from(files[0]).expect("a glob"));
        }
        let mut request = listed(root.path(), BTreeMap::from([(rule_id("configured"), rule)]));
        request.corpus = Corpus::Listed(
            files
                .iter()
                .map(|file| RelativePath((*file).into()))
                .collect(),
        );
        request.hosted = vec![HostRelation {
            name: HostRelationName("config.external".into()),
            columns: vec![HostColumn {
                name: HostColumnName("prefix".into()),
                kind: HostKind::Text,
            }],
            rows: vec![HostRow(vec![Cell::Text(HostText(prefix.into()))])],
        }];
        request
    };
    let retained = engine(Cache::At(store.path().to_owned()));
    let mut first = None;
    for (prefix, exclude, found) in [
        ("V", false, 2),
        ("V", true, 1),
        ("V", true, 1),
        ("W", false, 0),
        ("V", false, 2),
    ] {
        let fresh = lint_with(&engine(Cache::None), request(prefix, exclude)).expect("a report");
        let answer = lint_with(&retained, request(prefix, exclude)).expect("a report");
        assert_eq!(answer, fresh, "prefix={prefix}, exclude={exclude}");
        assert_eq!(answer.findings.len(), found, "prefix={prefix}, exclude={exclude}");
        if exclude {
            assert_eq!(
                answer.findings[0].subject,
                Slot::Is(Cell::File(RelativePath(files[1].into())))
            );
        }
        let _kept = first.get_or_insert(answer);
    }
    assert_eq!(
        std::fs::metadata(source)
            .expect("metadata")
            .modified()
            .expect("modified"),
        modified
    );
    summary(&first.expect("a first run"))
}
