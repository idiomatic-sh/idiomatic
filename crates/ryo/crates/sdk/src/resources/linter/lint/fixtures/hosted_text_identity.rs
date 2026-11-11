//! The test data of `linter lint` that fixes hosted text identity.

use super::capability_provider::provider_linter;
use super::directory::case_directory;
use super::outcome::{Outcome, summary};
use super::rules::{listed, ordinary, rule_id};
use super::shared_calls::shared;
use crate::domain::cell::types::Cell;
use crate::domain::cell::types::HostText;
use crate::domain::file::types::RelativePath;
use crate::domain::host::types::HostColumn;
use crate::domain::host::types::HostColumnName;
use crate::domain::host::types::HostKind;
use crate::domain::host::types::HostRelationName;
use crate::resources::linter::lint::entry::lint_with;
use crate::resources::linter::lint::errors::LintError;
use crate::resources::linter::lint::types::Cache;
use crate::resources::linter::lint::types::HostRelation;
use crate::resources::linter::lint::types::HostRow;
use std::collections::BTreeMap;

/// Hosted text that collides in the readable wire form invalidates the retained cross-file findings when it changes.
pub(crate) fn hosted_text_identity() -> Result<Outcome, LintError> {
    let root = case_directory();
    let store = tempfile::tempdir().expect("store");
    let source = root.path().join("case.txt");
    let modified = std::fs::metadata(&source)
        .expect("metadata")
        .modified()
        .expect("modified");
    let engine = |cache| provider_linter("same-build", "host-identity", &[], "unused", cache);
    let request = |first: &str, second: &str| {
        let text = |name: &str, kind| HostColumn {
            name: HostColumnName(name.into()),
            kind,
        };
        let mut request = listed(
            root.path(),
            BTreeMap::from([(
                rule_id("host-check"),
                ordinary(
                    "let found = select file from \"host.values\" where first = \"x\ttext:y\"; emit found subject file key(file);",
                ),
            )]),
        );
        request.hosted = vec![HostRelation {
            name: HostRelationName("host.values".into()),
            columns: vec![
                text("file", HostKind::File),
                text("first", HostKind::Text),
                text("second", HostKind::Text),
            ],
            rows: vec![HostRow(vec![
                Cell::File(RelativePath("case.txt".into())),
                Cell::Text(HostText(first.into())),
                Cell::Text(HostText(second.into())),
            ])],
        }];
        request
    };
    let left = request("x\ttext:y", "z");
    let right = request("x", "y\ttext:z");
    let fresh_left = lint_with(&engine(Cache::None), left.clone())?;
    let fresh_right = lint_with(&engine(Cache::None), right.clone())?;
    assert_eq!(fresh_left.findings.len(), 1);
    assert!(fresh_right.findings.is_empty());
    let shared = engine(Cache::At(store.path().to_owned()));
    for expected in [
        (&left, &fresh_left),
        (&left, &fresh_left),
        (&right, &fresh_right),
        (&right, &fresh_right),
        (&left, &fresh_left),
    ] {
        assert_eq!(&lint_with(&shared, expected.0.clone())?, expected.1);
        assert_eq!(
            &lint_with(
                &engine(Cache::At(store.path().to_owned())),
                expected.0.clone()
            )?,
            expected.1,
            "fresh engine must also validate the persisted identity"
        );
    }
    assert_eq!(
        std::fs::metadata(source)
            .expect("metadata")
            .modified()
            .expect("modified"),
        modified
    );
    Ok(summary(&fresh_left))
}
