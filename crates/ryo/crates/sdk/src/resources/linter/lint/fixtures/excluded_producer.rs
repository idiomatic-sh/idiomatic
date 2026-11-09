//! The test data of `linter lint` that fixes an excluded producer.

use super::capability_provider::provider_linter;
use super::directory::case_directory;
use super::observations::column;
use super::outcome::{Outcome, summary};
use super::rules::{listed, ordinary, rule_id};
use crate::domain::observation::types::ObservationBindingName;
use crate::domain::observation::types::ObservationKind;
use crate::domain::observation::types::ObservationName;
use crate::resources::linter::lint::entry::lint_with;
use crate::resources::linter::lint::errors::LintError;
use crate::resources::linter::lint::types::Cache;
use crate::resources::linter::lint::types::Glob;
use crate::resources::linter::lint::types::ObservationBinding;
use crate::resources::linter::lint::types::ObservationProducer;
use std::collections::BTreeMap;

/// A producer excluded from every file still publishes its observation to the consumer that binds it, reports none of its own
/// findings, and answers the same on a retained store; with the producer gone from the request the binding is refused, never read
/// as an empty input.
pub(crate) fn excluded_producer() -> Outcome {
    let root = case_directory();
    let store = tempfile::tempdir().expect("store");
    let engine = provider_linter(
        "observation-fixture",
        "observation-fixture",
        &[],
        "unused",
        Cache::At(store.path().to_owned()),
    );
    let producer = rule_id("producer");
    let consumer = rule_id("consumer");
    let mut consuming =
        ordinary("let found = select path from published; emit found subject path key(path);");
    consuming.observations.push(ObservationBinding {
        name: ObservationBindingName("published".into()),
        columns: vec![column("path", ObservationKind::File)],
        producers: vec![ObservationProducer::Output {
            rule: producer.clone(),
            output: ObservationName("used".into()),
        }],
    });
    let mut producing = ordinary(
        "let files = select path from \"filesystem.file\"; observe files as used; emit files subject path key(path);",
    );
    producing
        .exclude
        .push(Glob::try_from("**").expect("a glob"));
    let request = listed(
        root.path(),
        BTreeMap::from([(consumer.clone(), consuming), (producer.clone(), producing)]),
    );
    let mut last = (0, 0, true, 0);
    for _ in 0..2 {
        let answer = lint_with(&engine, request.clone()).expect("a report");
        last = summary(&answer);
        assert_eq!(answer.findings.len(), 1);
        assert_eq!(answer.findings[0].rule, consumer);
        assert!(answer.unknowns.is_empty());
    }
    let mut missing = request;
    let _removed = missing.rules.remove(&producer);
    let refused = lint_with(&engine, missing);
    assert!(
        matches!(refused, Err(LintError::ObservationBindings { .. })),
        "a bound producer that is gone is refused, not an empty input: {refused:?}"
    );
    last
}
