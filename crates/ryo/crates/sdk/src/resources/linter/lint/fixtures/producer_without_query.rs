//! The test data of `linter lint` that fixes producer without query.

use super::capability_provider::provider_linter;
use super::directory::case_directory;
use super::observations::{column, incoming};
use super::outcome::{Outcome, summary};
use super::rules::{listed, ordinary, rule_id};
use crate::domain::observation::types::ObservationBindingName;
use crate::domain::observation::types::ObservationKind;
use crate::resources::linter::lint::entry::lint_with;
use crate::resources::linter::lint::types::Cache;
use crate::resources::linter::lint::types::ObservationBinding;
use crate::resources::linter::lint::types::ObservationProducer;
use std::collections::BTreeMap;

/// A selected producer that has no query never becomes a complete empty input of its consumer.
pub(crate) fn producer_without_query() -> Outcome {
    let root = case_directory();
    let store = tempfile::tempdir().expect("store");
    let engine = provider_linter(
        "no-query-fixture",
        "no-query-fixture",
        &[],
        "unused",
        Cache::At(store.path().to_owned()),
    );
    let consumer = rule_id("consumer");
    let mut rule =
        ordinary("let found = select path from incoming; emit found subject path key(path);");
    rule.observations = vec![ObservationBinding {
        name: ObservationBindingName("incoming".into()),
        columns: vec![column("path", ObservationKind::File)],
        producers: vec![ObservationProducer::NoQuery {
            rule: rule_id("selected-unimplemented"),
        }],
    }];
    let request = listed(root.path(), BTreeMap::from([(consumer.clone(), rule)]));
    let mut last = (0, 0, true, 0);
    for _ in 0..2 {
        let answer = lint_with(&engine, request.clone()).expect("a report");
        assert!(answer.findings.is_empty());
        assert!(
            answer
                .unknowns
                .iter()
                .any(|unknown| unknown.rule == consumer)
        );
        last = summary(&answer);
    }
    last
}
