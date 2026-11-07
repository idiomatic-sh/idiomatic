//! The test data of `linter lint` that fixes unbound export.

use crate::resources::linter::lint::entry::lint_with;
use crate::resources::linter::lint::types::Cache;
use std::collections::BTreeMap;
use super::outcome::{Outcome, summary};
use super::capability_provider::provider_linter;
use super::rules::{listed, ordinary, rule_id};
use super::directory::case_directory;

/// An export nothing binds demands no reader and changes no ordinary result.
pub(crate) fn unbound_export() -> Outcome {
    let root = case_directory();
    // This fixture reports Missing::Any if Identifier is demanded. An unused
    // export must be pruned before request wants, not merely ignored at rendering.
    let store = tempfile::tempdir().expect("store");
    let engine = provider_linter(
        "ordinary-unbound-fixture",
        "ordinary-unbound-fixture",
        &[],
        "unused",
        Cache::At(store.path().to_owned()),
    );
    let request = |query: &str| {
        listed(
            root.path(),
            BTreeMap::from([(rule_id("ordinary"), ordinary(query))]),
        )
    };
    let plain = request(
        "let found = select path from \"filesystem.file\"; emit found subject path key(path);",
    );
    let unbound = request(
        "let found = select path from \"filesystem.file\"; let unavailable = select id from \"identifier\"; observe unavailable as unused; emit found subject path key(path);",
    );
    let expected = lint_with(&engine, plain).expect("a report");
    assert_eq!(expected.findings.len(), 1);
    assert!(expected.unknowns.is_empty());
    for _ in 0..2 {
        assert_eq!(
            lint_with(&engine, unbound.clone()).expect("a report"),
            expected
        );
    }
    summary(&expected)
}
