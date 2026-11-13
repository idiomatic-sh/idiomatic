//! The test data of `linter lint` that fixes refused and empty producers.

use super::capability_provider::provider_linter;
use super::directory::case_directory;
use super::observations::{column, incoming};
use super::outcome::{Outcome, summary};
use super::rules::{listed, ordinary, rule_id};
use super::shared_calls::shared;
use crate::domain::observation::types::ObservationKind;
use crate::resources::linter::lint::entry::lint_with;
use crate::resources::linter::lint::errors::LintError;
use crate::resources::linter::lint::types::Cache;
use ryo_relations::domain::catalog::types::OptionalCapability;
use std::collections::BTreeMap;

/// A producer whose population is refused and one whose rows are truly empty stay distinct on a cold and a retained replay.
pub(crate) fn refused_and_empty_producers() -> Result<Outcome, LintError> {
    let root = case_directory();
    let mut last = (0, 0, true, 0);
    for known in [true, false] {
        let store = tempfile::tempdir().expect("store");
        let capabilities: &'static [OptionalCapability] = if known {
            &[OptionalCapability::Identifier]
        } else {
            &[]
        };
        let engine = provider_linter(
            "readiness-fixture",
            "readiness-fixture",
            capabilities,
            "present",
            Cache::At(store.path().to_owned()),
        );
        let p = rule_id("producer");
        let c = rule_id("consumer");
        let mut consumer = ordinary(
            "let found = select id, file, span from incoming; emit found subject file key(id);",
        );
        consumer.observations = vec![incoming(
            vec![
                column("id", ObservationKind::Node),
                column("file", ObservationKind::File),
                column("span", ObservationKind::Span),
            ],
            p.clone(),
        )];
        let request = listed(
            root.path(),
            BTreeMap::from([
                (
                    p,
                    ordinary(
                        "let files = select path from \"filesystem.file\"; let empty = select f.path as path from files as f where f.path != f.path; let rows = select id, file, span from \"identifier\" where name = \"absent\"; observe rows as shared; emit empty subject path key(path);",
                    ),
                ),
                (c.clone(), consumer),
            ]),
        );
        for _ in 0..2 {
            let answer = lint_with(&engine, request.clone())?;
            assert!(answer.findings.is_empty());
            last = summary(&answer);
            assert_eq!(
                answer.unknowns.iter().any(|unknown| unknown.rule == c),
                !known,
                "filtered Unknown population cannot become a complete empty observation on warm replay"
            );
        }
    }
    Ok(last)
}
