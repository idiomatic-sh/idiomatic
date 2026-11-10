//! The I001 namespace observation across cold and warm replay.
use super::prepared_engine_test::python_310;
use std::collections::BTreeMap;

pub(super) fn actual_i001_mixed_namespace_observation_stays_unknown_on_cold_and_warm_replay() {
    use ryo::linter::{
        Cache, CaseSource, Corpus, ExecutablePermission, GivenProjectInventory, Glob, LintRequest,
        Linter, MemoryBudget, ObservationBinding, ObservationBindingName, ObservationColumn,
        ObservationColumnName, ObservationKind, ObservationName, ObservationProducer,
        ProjectEntryObservation, ProjectEntryState, ProjectInventoryCompleteness,
        ProjectSearchScope, RelativePath, Rule, RuleApplicability, RuleId, SourceText, WorkerPool,
        Workers, lint_with,
    };
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let authored = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("rules/I001/query.ryoql"),
    )
    .unwrap();
    let emission = authored.rfind("emit ").expect("actual I001 emission");
    let producer_query = format!(
        "{}let known_imports = select r.id as id, r.file as file, r.span as span from first_party_aliases as l join import_nodes as r on l.file = r.file; observe import_nodes as independent_imports; observe known_imports as namespace_imports; {}",
        &authored[..emission],
        &authored[emission..]
    );
    let profile: toml::Value = toml::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmark.toml"),
        )
        .unwrap(),
    )
    .unwrap();
    let modules = profile["query-modules"]
        .as_table()
        .unwrap()
        .iter()
        .map(|(name, path)| {
            ryo::query::QueryModuleSource::try_new(
                name,
                &std::fs::read_to_string(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path.as_str().unwrap()),
                )
                .unwrap(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let producer = RuleId::try_from("I001").unwrap();
    let independent = RuleId::try_from("INDEPENDENT").unwrap();
    let dependent = RuleId::try_from("DEPENDENT").unwrap();
    let modules = ryo::query::QueryModuleCatalog::try_new(modules).unwrap();
    let rule = |query: &str, observations| Rule {
        query: ryo::query::RyoQlQuery::with_modules("test/readiness", query, &modules).unwrap(),
        observations,
        applicability: RuleApplicability::Always,
        parameters: BTreeMap::new(),
        include: Vec::new(),
        exclude: Vec::new(),
    };
    let consumer = |output: &str| {
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
                    rule: producer.clone(),
                    output: ObservationName(output.into()),
                }],
            }],
        )
    };
    let mut producing = rule(&producer_query, Vec::new());
    producing.exclude.push(Glob::try_from("**").unwrap());
    let mut request = LintRequest {
        source_language: ryo::linter::SourceLanguage::Recognized,
        root: root.path().into(),
        project_search_scope: ProjectSearchScope::Declared(vec![RelativePath("".into())]),
        given_project_inventory: GivenProjectInventory {
            entries: vec![ProjectEntryObservation {
                path: RelativePath("local.py".into()),
                state: ProjectEntryState::File,
            }],
            completeness: ProjectInventoryCompleteness::Partial,
        },
        corpus: Corpus::Given(vec![CaseSource {
            path: RelativePath("case.py".into()),
            text: SourceText("import local\nimport unresolved\n".into()),
            executable: ExecutablePermission::Unknown,
        }]),
        rules: BTreeMap::from([
            (independent.clone(), consumer("independent_imports")),
            (dependent.clone(), consumer("namespace_imports")),
            (producer.clone(), producing),
        ]),
        files: Vec::new(),
        beside: Vec::new(),
        hosted: Vec::new(),
    };
    let engine = |cache| {
        Linter::new(
            python_310(),
            WorkerPool::from(Workers(std::num::NonZeroUsize::MIN)),
            MemoryBudget::Machine,
            cache,
        )
    };
    let cached = engine(Cache::At(store.path().into()));
    for complete in [true, true, false, false, true] {
        request.given_project_inventory.completeness = if complete {
            ProjectInventoryCompleteness::CompleteWithinDeclaredScope
        } else {
            ProjectInventoryCompleteness::Partial
        };
        let fresh = lint_with(&engine(Cache::None), request.clone()).unwrap();
        let actual = lint_with(&cached, request.clone()).unwrap();
        assert_eq!(
            actual, fresh,
            "persisted readiness must agree with a fresh evaluation"
        );
        assert_eq!(
            actual
                .findings
                .iter()
                .filter(|finding| finding.rule == independent)
                .count(),
            2,
            "independent import root remains known"
        );
        assert!(
            !actual
                .unknowns
                .iter()
                .any(|unknown| unknown.rule == independent)
        );
        assert_eq!(
            actual
                .findings
                .iter()
                .filter(|finding| finding.rule == dependent)
                .count(),
            if complete { 2 } else { 0 }
        );
        assert_eq!(
            actual
                .unknowns
                .iter()
                .any(|unknown| unknown.rule == dependent),
            !complete,
            "filtered positive namespace rows do not erase the missing physical population"
        );
        assert!(
            !actual
                .findings
                .iter()
                .any(|finding| finding.rule == producer),
            "excluded Emit does not suppress demanded roots"
        );
    }
}
