//! Actual I001 suppression output and RUF100 readiness across project inventory changes.
use super::constants::{
    BENCHMARK_TOML, BENCHMARKS_LINTERS_RUFF, BROKEN_PY, CASE_PY, CODE, CONFIG_DISABLED,
    CONFIG_EXTERNAL, CONFIG_SELECTED, DEF_BROKEN_NOQA_I001, DIRECTIVE, EMPTY, FILE, HIDDEN, I001,
    IMPORT_A_IMPORT_B, IMPORT_B_IMPORT_A, IMPORT_LOCAL_IMPORT_NUMPY_NOQA_I00,
    IMPORT_LOCAL_NOQA_I001_IMPORT_NUMP, INCOMING, LET_LOCATED_SELECT_L_DIRECTIVE_AS, LOCAL_PY,
    NOQA_MATCHES, OTHER_PY, PREFIX, QUERY_MODULES, RUF100, RULES_I001_QUERY_RYOQL,
    RULES_RUF100_QUERY_RYOQL, SUPPRESSION_MATCHES, TEST_HIDDEN, TEST_I001, TEST_RUF100, TEXT,
};
use super::*;
use ryo::linter::{
    Cache, CaseSource, Cell, Corpus, ExecutablePermission, GivenProjectInventory, Glob, HostColumn,
    HostColumnName, HostKind, HostRelation, HostRelationName, HostRow, HostText, LintRequest,
    Linter, MemoryBudget, ObservationBinding, ObservationBindingName, ObservationColumn,
    ObservationColumnName, ObservationKind, ObservationName, ObservationProducer,
    ProjectEntryObservation, ProjectEntryState, ProjectInventoryCompleteness, ProjectSearchScope,
    RelativePath, Rule, RuleApplicability, RuleId, SourceText, WorkerPool, Workers, lint_with,
};

#[test]
fn i001_hidden_match_and_ruf100_output_follow_first_import_and_unknown_inventory() {
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let authored = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(BENCHMARKS_LINTERS_RUFF);
    let profile: toml::Value =
        toml::from_str(&std::fs::read_to_string(authored.join(BENCHMARK_TOML)).unwrap()).unwrap();
    let modules = profile[QUERY_MODULES]
        .as_table()
        .unwrap()
        .iter()
        .map(|(name, path)| {
            ryo::query::QueryModuleSource::try_new(
                name,
                &std::fs::read_to_string(authored.join(path.as_str().unwrap())).unwrap(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let modules = ryo::query::QueryModuleCatalog::try_new(modules).unwrap();
    let i001 = RuleId::try_from(I001).unwrap();
    let ruf100 = RuleId::try_from(RUF100).unwrap();
    let hidden = RuleId::try_from(HIDDEN).unwrap();
    let columns = vec![
        ObservationColumn {
            name: ObservationColumnName(DIRECTIVE.into()),
            kind: ObservationKind::Node,
        },
        ObservationColumn {
            name: ObservationColumnName(FILE.into()),
            kind: ObservationKind::File,
        },
        ObservationColumn {
            name: ObservationColumnName(CODE.into()),
            kind: ObservationKind::Symbol,
        },
    ];
    let binding = |name: &str| {
        vec![ObservationBinding {
            name: ObservationBindingName(name.into()),
            columns: columns.clone(),
            producers: vec![ObservationProducer::Output {
                rule: i001.clone(),
                output: ObservationName(NOQA_MATCHES.into()),
            }],
        }]
    };
    let rule = |name: &str, query: &str, observations| Rule {
        query: ryo::query::RyoQlQuery::with_modules(name, query, &modules).unwrap(),
        observations,
        applicability: RuleApplicability::Always,
        parameters: BTreeMap::new(),
        include: Vec::new(),
        exclude: Vec::new(),
    };
    let mut producer = rule(
        TEST_I001,
        &std::fs::read_to_string(authored.join(RULES_I001_QUERY_RYOQL)).unwrap(),
        Vec::new(),
    );
    producer.exclude.push(Glob::try_from(TEXT).unwrap());
    let consumer = rule(
        TEST_RUF100,
        &std::fs::read_to_string(authored.join(RULES_RUF100_QUERY_RYOQL)).unwrap(),
        binding(SUPPRESSION_MATCHES),
    );
    let hidden_query = LET_LOCATED_SELECT_L_DIRECTIVE_AS;
    let hidden_consumer = rule(TEST_HIDDEN, hidden_query, binding(INCOMING));
    let hosted = vec![
        HostRelation {
            name: HostRelationName(CONFIG_SELECTED.into()),
            columns: vec![HostColumn {
                name: HostColumnName(CODE.into()),
                kind: HostKind::Text,
            }],
            rows: [I001, RUF100, HIDDEN]
                .into_iter()
                .map(|code| HostRow(vec![Cell::Text(HostText(code.into()))]))
                .collect(),
        },
        HostRelation {
            name: HostRelationName(CONFIG_DISABLED.into()),
            columns: vec![
                HostColumn {
                    name: HostColumnName(FILE.into()),
                    kind: HostKind::File,
                },
                HostColumn {
                    name: HostColumnName(CODE.into()),
                    kind: HostKind::Text,
                },
            ],
            rows: Vec::new(),
        },
        HostRelation {
            name: HostRelationName(CONFIG_EXTERNAL.into()),
            columns: vec![HostColumn {
                name: HostColumnName(PREFIX.into()),
                kind: HostKind::Text,
            }],
            rows: Vec::new(),
        },
    ];
    let engine = |cache| {
        Linter::new(
            python_310(),
            WorkerPool::from(Workers(std::num::NonZeroUsize::MIN)),
            MemoryBudget::Machine,
            cache,
        )
    };
    let cached = engine(Cache::At(store.path().into()));
    for (source, first_line) in [
        (IMPORT_LOCAL_NOQA_I001_IMPORT_NUMP, true),
        (IMPORT_LOCAL_IMPORT_NUMPY_NOQA_I00, false),
    ] {
        for (phase, complete) in [true, true, false, false, true].into_iter().enumerate() {
            let request = LintRequest {
                source_language: None,
                root: root.path().into(),
                project_search_scope: ProjectSearchScope::Declared(vec![RelativePath(
                    EMPTY.into(),
                )]),
                given_project_inventory: GivenProjectInventory {
                    entries: vec![ProjectEntryObservation {
                        path: RelativePath(LOCAL_PY.into()),
                        state: ProjectEntryState::File,
                    }],
                    completeness: if complete {
                        ProjectInventoryCompleteness::CompleteWithinDeclaredScope
                    } else {
                        ProjectInventoryCompleteness::Partial
                    },
                },
                corpus: Corpus::Given(vec![CaseSource {
                    path: RelativePath(CASE_PY.into()),
                    text: SourceText(source.into()),
                    executable: ExecutablePermission::Unknown,
                }]),
                rules: BTreeMap::from([
                    (i001.clone(), producer.clone()),
                    (ruf100.clone(), consumer.clone()),
                    (hidden.clone(), hidden_consumer.clone()),
                ]),
                files: Vec::new(),
                beside: Vec::new(),
                hosted: hosted.clone(),
            };
            let fresh = lint_with(&engine(Cache::None), request.clone()).unwrap();
            let actual = lint_with(&cached, request.clone()).unwrap();
            assert_eq!(actual, fresh, "warm result must match fresh result");
            let count = |rule: &RuleId| {
                actual
                    .findings
                    .iter()
                    .filter(|finding| &finding.rule == rule)
                    .count()
            };
            assert_eq!(count(&i001), 0, "excluded producer Emit stays hidden");
            assert_eq!(
                count(&hidden),
                usize::from(complete && first_line),
                "actual hidden I001 match"
            );
            assert_eq!(
                count(&ruf100),
                usize::from(complete && !first_line),
                "actual RUF100 used/unused result"
            );
            for rule in [&hidden, &ruf100] {
                assert_eq!(
                    actual.unknowns.iter().any(|unknown| &unknown.rule == rule),
                    !complete,
                    "Unknown source population must not become a complete empty observation",
                );
            }
            if phase == 0 {
                let mut mixed_syntax = request.clone();
                let Corpus::Given(files) = &mut mixed_syntax.corpus else {
                    unreachable!()
                };
                files.push(CaseSource {
                    path: RelativePath(BROKEN_PY.into()),
                    text: SourceText(DEF_BROKEN_NOQA_I001.into()),
                    executable: ExecutablePermission::Unknown,
                });
                let fresh_mixed = lint_with(&engine(Cache::None), mixed_syntax.clone()).unwrap();
                let warm_mixed = lint_with(&cached, mixed_syntax.clone()).unwrap();
                assert_eq!(warm_mixed, fresh_mixed);
                assert_eq!(lint_with(&cached, mixed_syntax).unwrap(), fresh_mixed);
                for (rule, expected) in [
                    (&hidden, usize::from(first_line)),
                    (&ruf100, usize::from(!first_line)),
                ] {
                    assert_eq!(
                        warm_mixed
                            .findings
                            .iter()
                            .filter(|finding| &finding.rule == rule
                                && finding.subject
                                    == ryo::linter::Slot::Is(Cell::File(RelativePath(
                                        CASE_PY.into()
                                    ))))
                            .count(),
                        expected,
                        "the valid file keeps its actual I001-derived answer"
                    );
                    assert!(
                        !warm_mixed
                            .unknowns
                            .iter()
                            .any(|unknown| &unknown.rule == rule
                                && unknown.path == RelativePath(CASE_PY.into())),
                        "one broken file cannot poison the valid named partition"
                    );
                    assert!(
                        warm_mixed
                            .unknowns
                            .iter()
                            .any(|unknown| &unknown.rule == rule
                                && unknown.path == RelativePath(BROKEN_PY.into())),
                        "the incomplete partition remains explicitly Unknown"
                    );
                }
            }
            if first_line && phase == 0 {
                // The producer is hidden only on case.py. Its independent
                // observation still serves that file while another file's
                // diagnostic Emit remains visible under the same rule.
                let mut mixed = request;
                mixed.rules.get_mut(&i001).unwrap().exclude =
                    vec![Glob::try_from(CASE_PY).unwrap()];
                let Corpus::Given(files) = &mut mixed.corpus else {
                    unreachable!()
                };
                files.push(CaseSource {
                    path: RelativePath(OTHER_PY.into()),
                    text: SourceText(IMPORT_B_IMPORT_A.into()),
                    executable: ExecutablePermission::Unknown,
                });
                let fresh_mixed = lint_with(&engine(Cache::None), mixed.clone()).unwrap();
                let warm_mixed = lint_with(&cached, mixed.clone()).unwrap();
                assert_eq!(warm_mixed, fresh_mixed);
                assert_eq!(
                    warm_mixed
                        .findings
                        .iter()
                        .filter(|finding| finding.rule == i001)
                        .count(),
                    1,
                    "other.py alone remains governed by I001"
                );
                assert_eq!(
                    warm_mixed
                        .findings
                        .iter()
                        .filter(|finding| finding.rule == hidden)
                        .count(),
                    1,
                    "excluded case.py still publishes its named output"
                );
                assert_eq!(lint_with(&cached, mixed.clone()).unwrap(), fresh_mixed);

                // Change an included file after both producers have been
                // retained. Its diagnostic must change while the hidden
                // observation from case.py remains available.
                {
                    let Corpus::Given(files) = &mut mixed.corpus else {
                        unreachable!()
                    };
                    files[1].text = SourceText(IMPORT_A_IMPORT_B.into());
                }
                let fresh_other_edit = lint_with(&engine(Cache::None), mixed.clone()).unwrap();
                let warm_other_edit = lint_with(&cached, mixed.clone()).unwrap();
                assert_eq!(warm_other_edit, fresh_other_edit);
                assert_eq!(
                    warm_other_edit
                        .findings
                        .iter()
                        .filter(|finding| finding.rule == i001)
                        .count(),
                    0,
                    "the included file is now sorted"
                );
                assert_eq!(
                    warm_other_edit
                        .findings
                        .iter()
                        .filter(|finding| finding.rule == hidden)
                        .count(),
                    1,
                    "the excluded producer still serves its named output"
                );

                // Move the directive off the first import. The hidden
                // observation disappears, and the consumer reports it as
                // unused. This must invalidate a retained producer frontier.
                {
                    let Corpus::Given(files) = &mut mixed.corpus else {
                        unreachable!()
                    };
                    files[0].text = SourceText(IMPORT_LOCAL_IMPORT_NUMPY_NOQA_I00.into());
                }
                let fresh_producer_edit = lint_with(&engine(Cache::None), mixed.clone()).unwrap();
                let warm_producer_edit = lint_with(&cached, mixed).unwrap();
                assert_eq!(warm_producer_edit, fresh_producer_edit);
                let count = |rule: &RuleId| {
                    warm_producer_edit
                        .findings
                        .iter()
                        .filter(|finding| &finding.rule == rule)
                        .count()
                };
                assert_eq!(count(&i001), 0);
                assert_eq!(count(&hidden), 0);
                assert_eq!(count(&ruf100), 1);
            }
        }
    }
}
