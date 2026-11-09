//! The test data of `linter lint` that fixes re exported publication.

use super::capability_provider::provider_linter;
use super::directory::case_directory;
use super::observations::{column, incoming};
use super::outcome::{Outcome, summary};
use super::rules::{listed, ordinary, rule_id};
use super::shared_calls::shared;
use crate::domain::cell::types::Cell;
use crate::domain::cell::types::HostText;
use crate::domain::cell::types::Slot;
use crate::domain::observation::types::ObservationKind;
use crate::domain::parameter::types::ParameterName;
use crate::domain::parameter::types::ParameterText;
use crate::domain::parameter::types::ParameterValue;
use crate::resources::linter::lint::entry::lint_with;
use crate::resources::linter::lint::types::Cache;
use std::collections::BTreeMap;

/// A publication re-exported by a second rule survives a cold run, a warm run, a replay of the consumer and a fresh engine.
pub(crate) fn re_exported_publication() -> Outcome {
    let root = case_directory();
    let store = tempfile::tempdir().expect("store");
    let engine = |cache| {
        provider_linter(
            "cross-observation-v1",
            "cross-observation-v1",
            &[],
            "unused",
            cache,
        )
    };
    let p = rule_id("z-producer");
    let q = rule_id("m-reexport");
    let c = rule_id("a-consumer");
    let columns = vec![
        column("count", ObservationKind::Number),
        column("label", ObservationKind::Symbol),
    ];
    let request = |marker: &str| {
        let producer = ordinary(
            r#"let files = select path from "filesystem.file";
let empty = select f.path as path from files as f where f.path != f.path;
let counted = group-count(from=files, keys=[], count="count");
let published = extend(from=counted, values={label: "escaped λ	\tvalue"});
observe published as shared;
emit empty subject path key(path);"#,
        );
        let mut reexport = ordinary(
            "let copied = select count, label from incoming; let empty = select l.count as count from copied as l where l.count != l.count; observe copied as shared; emit empty subject count key(count);",
        );
        reexport.observations = vec![incoming(columns.clone(), p.clone())];
        let mut consumer = ordinary(
            "param marker: string; let found = select count, label from incoming where label != $marker; emit found subject label key(count);",
        );
        consumer.observations = vec![incoming(columns.clone(), q.clone())];
        consumer.parameters.insert(
            ParameterName("marker".into()),
            ParameterValue::Text(ParameterText(marker.into())),
        );
        listed(
            root.path(),
            BTreeMap::from([
                (c.clone(), consumer),
                (q.clone(), reexport),
                (p.clone(), producer),
            ]),
        )
    };
    let cached = engine(Cache::At(store.path().to_owned()));
    // RyoQL strings preserve their bytes (winnow::string): a literal tab is
    // authored directly, while backslash-t remains a distinct pair of bytes.
    let label = "escaped λ\t\\tvalue";
    let mut last = (0, 0, true, 0);
    for marker in ["never-a", "never-a", label, label, "never-b", "never-a"] {
        let requested = request(marker);
        let fresh = lint_with(&engine(Cache::None), requested.clone()).expect("a report");
        let expected = usize::from(marker != label);
        assert_eq!(fresh.findings.len(), expected);
        if expected != 0 {
            assert_eq!(fresh.findings[0].rule, c);
            assert_eq!(
                fresh.findings[0].subject,
                Slot::Is(Cell::Text(HostText(label.into())))
            );
        }
        assert!(fresh.unknowns.is_empty());
        last = summary(&fresh);
        assert_eq!(
            lint_with(&cached, requested.clone()).expect("a report"),
            fresh
        );
        assert_eq!(
            lint_with(&engine(Cache::At(store.path().to_owned())), requested).expect("a report"),
            fresh
        );
    }
    last
}
