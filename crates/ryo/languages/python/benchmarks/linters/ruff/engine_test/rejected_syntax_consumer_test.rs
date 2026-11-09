//! A rejected file beside a valid one, with the PT rules feeding RUF100 through the configuration tables: every rule answers the
//! valid file as Ruff does and skips the rejected one.
use super::prepared_engine_test::python_310;
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use std::collections::BTreeMap;

/// What a row of the cases runs.
#[derive(Clone, Copy)]
enum Scenario {
    /// The PT rules and RUF100 over a valid file beside a rejected one.
    PtRulesAndRuf100BesideARejectedFile,
}

type Input = Scenario;
type Value = bool;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_rejected_file_leaves_pt020_pt025_pt026_and_ruf100_answering_the_valid_file"),
    word: Word::Succeeds,
    input: || Scenario::PtRulesAndRuf100BesideARejectedFile,
    expect: Expect::Value(true),
}];

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::PtRulesAndRuf100BesideARejectedFile => {
            a_rejected_file_leaves_pt020_pt025_pt026_and_ruf100_answering_the_valid_file();
            Ok(true)
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// Full Django's shape at checkpoint 323 (`django/__init__.py` beside `tests_syntax_error.py`): RUF100 reads the PT rules'
/// suppression matches through its joins with `config.selected` and `config.external`, which hold no file; those joins keep the
/// matches' rows in their file, so the rejected file leaves every rule answering the valid one.
fn a_rejected_file_leaves_pt020_pt025_pt026_and_ruf100_answering_the_valid_file() {
    use super::constants::{
        BENCHMARK_TOML, BENCHMARKS_LINTERS_RUFF, CODE, CONFIG_DISABLED, CONFIG_EXTERNAL,
        CONFIG_SELECTED, DIRECTIVE, EMPTY, FILE, NOQA_MATCHES, PREFIX, QUERY_MODULES,
        RULES_RUF100_QUERY_RYOQL, SUPPRESSION_MATCHES,
    };
    use ryo::linter::{
        Cache, CaseSource, Cell, Column, Corpus, ExecutablePermission, GivenProjectInventory,
        HostColumn, HostColumnName, HostKind, HostRelation, HostRelationName, HostRow, HostText,
        Line, LintRequest, Linter, MemoryBudget, ObservationBinding, ObservationBindingName,
        ObservationColumn, ObservationColumnName, ObservationKind, ObservationName,
        ObservationProducer, Place, ProjectEntryObservation, ProjectEntryState,
        ProjectInventoryCompleteness, ProjectSearchScope, RelativePath, Rule, RuleApplicability,
        RuleId, SourceText, WorkerPool, Workers, lint_with,
    };
    const CASE: &str = "case.py";
    const BROKEN: &str = "broken.py";
    let root = tempfile::tempdir().unwrap();
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
    let pt020 = RuleId::try_from("PT020").unwrap();
    let pt025 = RuleId::try_from("PT025").unwrap();
    let pt026 = RuleId::try_from("PT026").unwrap();
    let ruf100 = RuleId::try_from("RUF100").unwrap();
    let rule = |name: &str, query: &str, observations| Rule {
        query: ryo::query::RyoQlQuery::with_modules(name, query, &modules).unwrap(),
        observations,
        applicability: RuleApplicability::AcceptedSyntax,
        parameters: BTreeMap::new(),
        include: Vec::new(),
        exclude: Vec::new(),
    };
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
    let consumed = vec![ObservationBinding {
        name: ObservationBindingName(SUPPRESSION_MATCHES.into()),
        columns,
        producers: [&pt020, &pt025, &pt026]
            .into_iter()
            .map(|producer| ObservationProducer::Output {
                rule: producer.clone(),
                output: ObservationName(NOQA_MATCHES.into()),
            })
            .collect(),
    }];
    let text = |code: &str| HostRow(vec![Cell::Text(HostText(code.into()))]);
    let hosted = vec![
        HostRelation {
            name: HostRelationName(CONFIG_SELECTED.into()),
            columns: vec![HostColumn {
                name: HostColumnName(CODE.into()),
                kind: HostKind::Text,
            }],
            rows: ["PT020", "PT025", "PT026", "RUF100"]
                .into_iter()
                .map(text)
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
    let source = |path: &str, text: &str| CaseSource {
        path: RelativePath(path.into()),
        text: SourceText(text.into()),
        executable: ExecutablePermission::Unknown,
    };
    let request = LintRequest {
        source_language: ryo::linter::SourceLanguage::Recognized,
        root: root.path().into(),
        project_search_scope: ProjectSearchScope::Declared(vec![RelativePath(EMPTY.into())]),
        given_project_inventory: GivenProjectInventory {
            entries: [CASE, BROKEN]
                .into_iter()
                .map(|path| ProjectEntryObservation {
                    path: RelativePath(path.into()),
                    state: ProjectEntryState::File,
                })
                .collect(),
            completeness: ProjectInventoryCompleteness::CompleteWithinDeclaredScope,
        },
        corpus: Corpus::Given(vec![
            source(BROKEN, "def broken(:\n    pass\n"),
            source(
                CASE,
                "import pytest\n@pytest.yield_fixture  # noqa: PT020\ndef resource():\n    yield 1\n@pytest.mark.usefixtures(\"resource\")\n@pytest.fixture\ndef combined():\n    return 1\n@pytest.mark.usefixtures()\ndef test_case():\n    pass\nvalue = 1  # noqa\n",
            ),
        ]),
        rules: BTreeMap::from([
            (
                pt020.clone(),
                rule(
                    "test/PT020",
                    include_str!("../rules/PT020/query.ryoql"),
                    Vec::new(),
                ),
            ),
            (
                pt025.clone(),
                rule(
                    "test/PT025",
                    include_str!("../rules/PT025/query.ryoql"),
                    Vec::new(),
                ),
            ),
            (
                pt026.clone(),
                rule(
                    "test/PT026",
                    include_str!("../rules/PT026/query.ryoql"),
                    Vec::new(),
                ),
            ),
            (
                ruf100.clone(),
                rule(
                    "test/RUF100",
                    &std::fs::read_to_string(authored.join(RULES_RUF100_QUERY_RYOQL)).unwrap(),
                    consumed,
                ),
            ),
        ]),
        files: Vec::new(),
        beside: Vec::new(),
        hosted,
    };
    let linter = Linter::new(
        python_310(),
        WorkerPool::from(Workers(std::num::NonZeroUsize::MIN)),
        MemoryBudget::Machine,
        Cache::None,
    );
    let report = lint_with(&linter, request).unwrap();
    let mut found = report
        .findings
        .iter()
        .filter(|finding| {
            finding.subject == ryo::linter::Slot::Is(Cell::File(RelativePath(CASE.into())))
        })
        .map(|finding| (finding.rule.clone(), finding.place))
        .collect::<Vec<_>>();
    found.sort_by(|left, right| left.0.cmp(&right.0));
    let at = |line, column| Place::At {
        line: Line(line),
        column: Column(column),
    };
    // Ruff 0.16.5 (--select PT020,PT025,PT026,RUF100 --target-version py310) on the same files: PT025 at 5:1, PT026 at 9:1,
    // RUF100 at 12:12 of case.py; PT020 at 2:1 is suppressed by its used noqa; broken.py has only invalid-syntax.
    assert_eq!(
        found,
        vec![
            (pt025.clone(), at(5, 1)),
            (pt026.clone(), at(9, 1)),
            (ruf100.clone(), at(12, 12))
        ],
        "the valid file is answered as Ruff answers it; what was not decided: {:?}",
        report.unknowns
    );
    assert!(
        report.unknowns.is_empty(),
        "the rejected file leaves no rule undecided anywhere: {:?}",
        report.unknowns
    );
    for rule in [&pt020, &pt025, &pt026, &ruf100] {
        assert!(
            report
                .skipped
                .iter()
                .any(|skip| &skip.rule == rule && skip.path == RelativePath(BROKEN.into())),
            "{rule:?} skips the rejected file"
        );
    }
}
