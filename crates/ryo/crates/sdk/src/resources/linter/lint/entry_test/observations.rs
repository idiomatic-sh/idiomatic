//! Real rule-order and zero-diagnostic producer controls through the public SDK route.
use super::super::types::{Glob, ObservationBinding, ObservationProducer};
use super::*;
use crate::domain::observation::types::ObservationKind;
use crate::domain::observation::types::{
    ObservationBindingName, ObservationColumn, ObservationColumnName, ObservationName,
};

#[test]
fn later_original_rule_publishes_nonempty_observation_despite_zero_findings() {
    let root = tempfile::tempdir().expect("corpus");
    let cache = tempfile::tempdir().expect("cache");
    std::fs::write(root.path().join("case.txt"), "x").expect("source");
    let engine = super::super::types::Linter::new(
        Languages(vec![crate::domain::language::types::Language::from_parts(
            crate::domain::language::types::LanguageName("fixture".into()),
            crate::domain::language::types::LanguageBuild("observation-fixture".into()),
            std::sync::Arc::new(CapabilityProvider(&[], "observation-fixture", "unused")),
            std::sync::Arc::new(
                ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl,
            ),
        )]),
        crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
        MemoryBudget::Machine,
        Cache::At(cache.path().to_owned()),
    );
    let producer = RuleId::try_from("z-producer").expect("ID");
    let consumer = RuleId::try_from("a-consumer").expect("ID");
    let ordinary = |query: &str| Rule {
        observations: Vec::new(),
        query: crate::resources::query::RyoQlQuery::try_from(query).expect("query"),
        applicability: crate::resources::linter::RuleApplicability::Always,
        parameters: BTreeMap::new(),
        include: Vec::new(),
        exclude: Vec::new(),
    };
    let mut consuming =
        ordinary("let found = select path from published; emit found subject path key(path);");
    consuming.observations.push(ObservationBinding {
        name: ObservationBindingName("published".into()),
        columns: vec![ObservationColumn {
            name: ObservationColumnName("path".into()),
            kind: ObservationKind::File,
        }],
        producers: vec![ObservationProducer::Output {
            rule: producer.clone(),
            output: ObservationName("used".into()),
        }],
    });
    let request = LintRequest {
        source_language: None,
        root: root.path().to_owned(),
        files: Vec::new(),
        beside: Vec::new(),
        hosted: Vec::new(),
        corpus: Corpus::Listed(vec![RelativePath("case.txt".into())]),
        rules: BTreeMap::from([
            (consumer.clone(), consuming),
            (
                producer,
                ordinary(
                    "let files = select path from \"filesystem.file\"; let empty = select f.path as path from files as f where f.path != f.path; observe files as used; emit empty subject path key(path);",
                ),
            ),
        ]),
    };
    for _ in 0..2 {
        let answer = super::super::entry::lint_with(&engine, request.clone())
            .expect("named producer executes before consumer");
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
            .get_mut(&RuleId::try_from("z-producer").unwrap())
            .unwrap();
        producing.query = crate::resources::query::RyoQlQuery::try_from(
            "let files = select path from \"filesystem.file\"; observe files as used; emit files subject path key(path);"
        ).unwrap();
        let pattern = Glob::try_from(if include_only {
            "other.txt"
        } else {
            "case.txt"
        })
        .unwrap();
        if include_only {
            producing.include.push(pattern);
        } else {
            producing.exclude.push(pattern);
        }
        for _ in 0..2 {
            let answer = super::super::entry::lint_with(&engine, configured.clone()).unwrap();
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
        .unwrap()
        .observations
        .clear();
    assert!(
        matches!(
            super::super::entry::lint_with(&engine, undeclared),
            Err(super::super::errors::LintError::Rule { .. })
        ),
        "removing the declaration must reject the source, not reuse a previously bound plan"
    );
}

#[test]
fn cross_publication_reexport_survives_cold_warm_consumer_replay_and_fresh_engine() {
    use super::super::types::{ParameterName, ParameterText, ParameterValue};
    use crate::domain::cell::types::Slot;
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("case.txt"), "x").unwrap();
    let engine = |cache| {
        super::super::types::Linter::new(
            Languages(vec![crate::domain::language::types::Language::from_parts(
                crate::domain::language::types::LanguageName("fixture".into()),
                crate::domain::language::types::LanguageBuild("cross-observation-v1".into()),
                std::sync::Arc::new(CapabilityProvider(&[], "cross-observation-v1", "unused")),
                std::sync::Arc::new(
                    ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl,
                ),
            )]),
            crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
            MemoryBudget::Machine,
            cache,
        )
    };
    let p = RuleId::try_from("z-producer").unwrap();
    let q = RuleId::try_from("m-reexport").unwrap();
    let c = RuleId::try_from("a-consumer").unwrap();
    let columns = vec![
        ObservationColumn {
            name: ObservationColumnName("count".into()),
            kind: ObservationKind::Number,
        },
        ObservationColumn {
            name: ObservationColumnName("label".into()),
            kind: ObservationKind::Symbol,
        },
    ];
    let binding = |producer: RuleId| ObservationBinding {
        name: ObservationBindingName("incoming".into()),
        columns: columns.clone(),
        producers: vec![ObservationProducer::Output {
            rule: producer,
            output: ObservationName("shared".into()),
        }],
    };
    let rule = |query: &str, observations| Rule {
        observations,
        query: crate::resources::query::RyoQlQuery::try_from(query).unwrap(),
        applicability: crate::resources::linter::RuleApplicability::Always,
        parameters: BTreeMap::new(),
        include: Vec::new(),
        exclude: Vec::new(),
    };
    let request = |marker: &str| {
        let producer = rule(
            r#"let files = select path from "filesystem.file";
let empty = select f.path as path from files as f where f.path != f.path;
let counted = group-count(from=files, keys=[], count="count");
let published = extend(from=counted, values={label: "escaped λ	\tvalue"});
observe published as shared;
emit empty subject path key(path);"#,
            Vec::new(),
        );
        let reexport = rule(
            "let copied = select count, label from incoming; let empty = select l.count as count from copied as l where l.count != l.count; observe copied as shared; emit empty subject count key(count);",
            vec![binding(p.clone())],
        );
        let mut consumer = rule(
            "param marker: string; let found = select count, label from incoming where label != $marker; emit found subject label key(count);",
            vec![binding(q.clone())],
        );
        consumer.parameters.insert(
            ParameterName("marker".into()),
            ParameterValue::Text(ParameterText(marker.into())),
        );
        LintRequest {
            source_language: None,
            root: root.path().to_owned(),
            files: Vec::new(),
            beside: Vec::new(),
            hosted: Vec::new(),
            corpus: Corpus::Listed(vec![RelativePath("case.txt".into())]),
            rules: BTreeMap::from([
                (c.clone(), consumer),
                (q.clone(), reexport),
                (p.clone(), producer),
            ]),
        }
    };
    let cached = engine(Cache::At(store.path().to_owned()));
    // RyoQL strings preserve their bytes (winnow::string): a literal tab is
    // authored directly, while backslash-t remains a distinct pair of bytes.
    let label = "escaped λ\t\\tvalue";
    for marker in ["never-a", "never-a", label, label, "never-b", "never-a"] {
        let requested = request(marker);
        let fresh =
            super::super::entry::lint_with(&engine(Cache::None), requested.clone()).unwrap();
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
        assert_eq!(
            super::super::entry::lint_with(&cached, requested.clone()).unwrap(),
            fresh
        );
        assert_eq!(
            super::super::entry::lint_with(&engine(Cache::At(store.path().to_owned())), requested)
                .unwrap(),
            fresh
        );
    }
}

#[test]
fn refused_and_true_empty_producers_remain_distinct_on_cold_and_retained_replay() {
    use ryo_relations::domain::relations::types::OptionalCapability;
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("case.txt"), "x").unwrap();
    for known in [true, false] {
        let store = tempfile::tempdir().unwrap();
        let capabilities: &'static [OptionalCapability] = if known {
            &[OptionalCapability::Identifier]
        } else {
            &[]
        };
        let engine = super::super::types::Linter::new(
            Languages(vec![crate::domain::language::types::Language::from_parts(
                crate::domain::language::types::LanguageName("fixture".into()),
                crate::domain::language::types::LanguageBuild("readiness-fixture".into()),
                std::sync::Arc::new(CapabilityProvider(
                    capabilities,
                    "readiness-fixture",
                    "present",
                )),
                std::sync::Arc::new(
                    ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl,
                ),
            )]),
            crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
            MemoryBudget::Machine,
            Cache::At(store.path().to_owned()),
        );
        let p = RuleId::try_from("producer").unwrap();
        let c = RuleId::try_from("consumer").unwrap();
        let rule = |query: &str, observations| Rule {
            observations,
            query: crate::resources::query::RyoQlQuery::try_from(query).unwrap(),
            applicability: crate::resources::linter::RuleApplicability::Always,
            parameters: BTreeMap::new(),
            include: Vec::new(),
            exclude: Vec::new(),
        };
        let request = LintRequest {
            source_language: None,
            root: root.path().to_owned(),
            files: Vec::new(),
            beside: Vec::new(),
            hosted: Vec::new(),
            corpus: Corpus::Listed(vec![RelativePath("case.txt".into())]),
            rules: BTreeMap::from([
                (
                    p.clone(),
                    rule(
                        "let files = select path from \"filesystem.file\"; let empty = select f.path as path from files as f where f.path != f.path; let rows = select id, file, span from \"identifier\" where name = \"absent\"; observe rows as shared; emit empty subject path key(path);",
                        Vec::new(),
                    ),
                ),
                (
                    c.clone(),
                    rule(
                        "let found = select id, file, span from incoming; emit found subject file key(id);",
                        vec![ObservationBinding {
                            name: ObservationBindingName("incoming".into()),
                            columns: vec![
                                ObservationColumn {
                                    name: ObservationColumnName("id".into()),
                                    kind: ObservationKind::Node,
                                },
                                ObservationColumn {
                                    name: ObservationColumnName("file".into()),
                                    kind: ObservationKind::File,
                                },
                                ObservationColumn {
                                    name: ObservationColumnName("span".into()),
                                    kind: ObservationKind::Span,
                                },
                            ],
                            producers: vec![ObservationProducer::Output {
                                rule: p,
                                output: ObservationName("shared".into()),
                            }],
                        }],
                    ),
                ),
            ]),
        };
        for _ in 0..2 {
            let answer = super::super::entry::lint_with(&engine, request.clone()).unwrap();
            assert!(answer.findings.is_empty());
            assert_eq!(
                answer.unknowns.iter().any(|unknown| unknown.rule == c),
                !known,
                "filtered Unknown population cannot become a complete empty observation on warm replay"
            );
        }
    }
}

#[test]
fn selected_no_query_producer_never_becomes_a_complete_empty_cross_input() {
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("case.txt"), "x").unwrap();
    let engine = super::super::types::Linter::new(
        Languages(vec![crate::domain::language::types::Language::from_parts(
            crate::domain::language::types::LanguageName("fixture".into()),
            crate::domain::language::types::LanguageBuild("no-query-fixture".into()),
            std::sync::Arc::new(CapabilityProvider(&[], "no-query-fixture", "unused")),
            std::sync::Arc::new(
                ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl,
            ),
        )]),
        crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
        MemoryBudget::Machine,
        Cache::At(store.path().to_owned()),
    );
    let consumer = RuleId::try_from("consumer").unwrap();
    let request = LintRequest {
        source_language: None,
        root: root.path().to_owned(),
        files: Vec::new(),
        beside: Vec::new(),
        hosted: Vec::new(),
        corpus: Corpus::Listed(vec![RelativePath("case.txt".into())]),
        rules: BTreeMap::from([(
            consumer.clone(),
            Rule {
                observations: vec![ObservationBinding {
                    name: ObservationBindingName("incoming".into()),
                    columns: vec![ObservationColumn {
                        name: ObservationColumnName("path".into()),
                        kind: ObservationKind::File,
                    }],
                    producers: vec![ObservationProducer::NoQuery {
                        rule: RuleId::try_from("selected-unimplemented").unwrap(),
                    }],
                }],
                query: crate::resources::query::RyoQlQuery::try_from(
                    "let found = select path from incoming; emit found subject path key(path);",
                )
                .unwrap(),
                applicability: crate::resources::linter::RuleApplicability::Always,
                parameters: BTreeMap::new(),
                include: Vec::new(),
                exclude: Vec::new(),
            },
        )]),
    };
    for _ in 0..2 {
        let answer = super::super::entry::lint_with(&engine, request.clone()).unwrap();
        assert!(answer.findings.is_empty());
        assert!(
            answer
                .unknowns
                .iter()
                .any(|unknown| unknown.rule == consumer)
        );
    }
}

#[test]
fn unbound_export_does_not_demand_a_reader_or_change_ordinary_results() {
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("case.txt"), "x").unwrap();
    // This fixture reports Missing::Any if Identifier is demanded. An unused
    // export must be pruned before request wants, not merely ignored at rendering.
    let engine = super::super::types::Linter::new(
        Languages(vec![crate::domain::language::types::Language::from_parts(
            crate::domain::language::types::LanguageName("fixture".into()),
            crate::domain::language::types::LanguageBuild("ordinary-unbound-fixture".into()),
            std::sync::Arc::new(CapabilityProvider(
                &[],
                "ordinary-unbound-fixture",
                "unused",
            )),
            std::sync::Arc::new(
                ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl,
            ),
        )]),
        crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
        MemoryBudget::Machine,
        Cache::At(store.path().to_owned()),
    );
    let request = |query: &str| LintRequest {
        source_language: None,
        root: root.path().to_owned(),
        files: Vec::new(),
        beside: Vec::new(),
        hosted: Vec::new(),
        corpus: Corpus::Listed(vec![RelativePath("case.txt".into())]),
        rules: BTreeMap::from([(
            RuleId::try_from("ordinary").unwrap(),
            Rule {
                observations: Vec::new(),
                query: crate::resources::query::RyoQlQuery::try_from(query).unwrap(),
                applicability: crate::resources::linter::RuleApplicability::Always,
                parameters: BTreeMap::new(),
                include: Vec::new(),
                exclude: Vec::new(),
            },
        )]),
    };
    let ordinary = request(
        "let found = select path from \"filesystem.file\"; emit found subject path key(path);",
    );
    let unbound = request(
        "let found = select path from \"filesystem.file\"; let unavailable = select id from \"identifier\"; observe unavailable as unused; emit found subject path key(path);",
    );
    let expected = super::super::entry::lint_with(&engine, ordinary).unwrap();
    assert_eq!(expected.findings.len(), 1);
    assert!(expected.unknowns.is_empty());
    for _ in 0..2 {
        assert_eq!(
            super::super::entry::lint_with(&engine, unbound.clone()).unwrap(),
            expected
        );
    }
}
