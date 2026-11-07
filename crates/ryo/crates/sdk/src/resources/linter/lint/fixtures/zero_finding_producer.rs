//! The test data of `linter lint` that fixes zero finding producer.

use crate::domain::observation::types::ObservationBindingName;
use crate::domain::observation::types::ObservationKind;
use crate::domain::observation::types::ObservationName;
use crate::resources::linter::lint::entry::lint_with;
use crate::resources::linter::lint::errors::LintError;
use crate::resources::linter::lint::types::Cache;
use crate::resources::linter::lint::types::Glob;
use crate::resources::linter::lint::types::ObservationBinding;
use crate::resources::linter::lint::types::ObservationProducer;
use crate::resources::query::RyoQlQuery;
use std::collections::BTreeMap;
use super::outcome::{Outcome, summary};
use super::capability_provider::provider_linter;
use super::rules::{listed, ordinary, rule_id};
use super::directory::case_directory;
use super::observations::column;

/// A later rule that produces an observation publishes it nonempty although it has no finding, and the include and exclude of a
/// producer govern its diagnostics, not the observation.
pub(crate) fn zero_finding_producer() -> Outcome {
    let root = case_directory();
    let store = tempfile::tempdir().expect("cache");
    let engine = provider_linter(
        "observation-fixture",
        "observation-fixture",
        &[],
        "unused",
        Cache::At(store.path().to_owned()),
    );
    let producer = rule_id("z-producer");
    let consumer = rule_id("a-consumer");
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
    let request = listed(
        root.path(),
        BTreeMap::from([
            (consumer.clone(), consuming),
            (
                producer,
                ordinary(
                    "let files = select path from \"filesystem.file\"; let empty = select f.path as path from files as f where f.path != f.path; observe files as used; emit empty subject path key(path);",
                ),
            ),
        ]),
    );
    let mut last = (0, 0, true, 0);
    for _ in 0..2 {
        let answer = lint_with(&engine, request.clone())
            .expect("named producer executes before consumer");
        last = summary(&answer);
        assert_eq!(
            answer.findings.len(),
            1,
            "observation rows are not diagnostics and the producer Emit is genuinely empty"
        );
        assert_eq!(answer.findings[0].rule, consumer);
        assert!(
            answer.unknowns.is_empty(),
            "a real complete publication satisfies the binding"
        );
    }
    // Diagnostic governance is independent of explicitly bound facts. Exercise
    // both include and exclude while retaining cold/warm public entry behavior.
    for include_only in [false, true] {
        let mut configured = request.clone();
        let producing = configured
            .rules
            .get_mut(&rule_id("z-producer"))
            .expect("the producer");
        producing.query = RyoQlQuery::try_from(
        "let files = select path from \"filesystem.file\"; observe files as used; emit files subject path key(path);",
    )
    .expect("a query");
        let pattern = Glob::try_from(if include_only {
            "other.txt"
        } else {
            "case.txt"
        })
        .expect("a glob");
        if include_only {
            producing.include.push(pattern);
        } else {
            producing.exclude.push(pattern);
        }
        for _ in 0..2 {
            let answer = lint_with(&engine, configured.clone()).expect("a report");
            assert!(
                answer.unknowns.is_empty(),
                "diagnostic exclusion is not missing observation data"
            );
            assert_eq!(answer.findings.len(), 1);
            assert_eq!(
                answer.findings[0].rule, consumer,
                "excluded producer Emit stays suppressed"
            );
        }
    }
    let mut undeclared = request.clone();
    undeclared
        .rules
        .get_mut(&consumer)
        .expect("the consumer")
        .observations
        .clear();
    assert!(
        matches!(lint_with(&engine, undeclared), Err(LintError::Rule { .. })),
        "removing the declaration must reject the source, not reuse a previously bound plan"
    );
    last
}
