//! The declared context cases and the check of each against the real SDK.
use super::prepared_engine_test::PreparedEngine as _;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use ryo_benchmark_contracts::domain::requests::types::WorkerState;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

/// One independently expected context; pinned competitor diagnostics are not rewritten.
#[derive(serde::Deserialize)]
pub(super) struct ContextCase {
    /// Stable identity for failure messages and isolated source directories.
    name: String,
    /// Complete valid source to analyze.
    source: String,
    /// Exact existing syntax occurrence selected by real byte locations.
    needle: String,
    /// Expected semantic node kind for that occurrence.
    kind: String,
    /// Explicit parser target.
    target: String,
    /// Source filename; a stub suffix must not invent a future feature.
    file: String,
    /// Whether the validated module future flag is enabled.
    feature: bool,
    /// Direct annotation evaluation context, or empty for a different context.
    evaluation: String,
}

/// Read the actual source assets, not a second implementation or a historical helper snapshot.
pub(super) fn stage_modules(root: &Path) -> Result<BTreeMap<String, PathBuf>, Error> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("fragments");
    std::fs::create_dir_all(root.join("fragments"))?;
    let mut modules = BTreeMap::new();
    for name in ["future-annotation-context", "annotation-evaluation-context"] {
        let relative = PathBuf::from(format!("fragments/{name}.ryoql"));
        let _copied = std::fs::copy(source.join(format!("{name}.ryoql")), root.join(&relative))?;
        let _previous = modules.insert(format!("python/ruff/{name}"), relative);
    }
    Ok(modules)
}

/// Select one real occurrence, with an independent INPUT finding preventing empty-input passes.
pub(super) fn write_queries(
    root: &Path,
    case: &ContextCase,
    start: usize,
    end: usize,
) -> Result<(), Error> {
    let input = format!(
        "let nodes = select id, kind, file, span from \"semantic-syntax\" where kind = {:?};\nlet locations = select id, \"start-byte\" as start, \"end-byte\" as end, file from \"source-location\";\nlet selected = select l.id as id, l.file as file, l.span as span from nodes as l join locations as r on l.id = r.id and l.file = r.file where r.start = {start} and r.end = {end};\nlet inputs = select id, file from selected;\n",
        case.kind
    );
    for (rule, evaluation) in [
        ("INPUT", ""),
        ("FLAG", ""),
        ("EAGER", "eager"),
        ("POSTPONED", "postponed"),
        ("DEFERRED", "deferred"),
        ("UNEVALUATED", "unevaluated"),
    ] {
        let directory = root.join("rules").join(rule);
        std::fs::create_dir_all(&directory)?;
        let query = if rule == "INPUT" {
            format!("{input}emit selected subject file key(span);\n")
        } else {
            let (import, matches) = if rule == "FLAG" {
                (
                    "import mode from \"python/ruff/future-annotation-context\";",
                    "let matched = mode(expressions=inputs);".to_owned(),
                )
            } else {
                (
                    "import mode from \"python/ruff/annotation-evaluation-context\";",
                    format!(
                        "let contexts = mode(expressions=inputs);\nlet matched = select id, file from contexts where evaluation = {evaluation:?};"
                    ),
                )
            };
            format!(
                "{import}\n{input}{matches}\nlet rows = select l.id as id, l.file as file, l.span as span from selected as l join matched as r on l.id = r.id and l.file = r.file;\nemit rows subject file key(span);\n"
            )
        };
        std::fs::write(directory.join("query.ryoql"), query)?;
    }
    Ok(())
}

/// The expected compiler mode and context are asserted through the actual SDK execution path.
pub(super) fn check_case(
    root: &Path,
    modules: &BTreeMap<String, PathBuf>,
    case: &ContextCase,
) -> Result<(), Error> {
    use ryo_python::settings::{PythonParserSettings, PythonTargetVersion};
    let target = match case.target.as_str() {
        "py313" => PythonTargetVersion::Py313,
        "py314" => PythonTargetVersion::Py314,
        "py315" => PythonTargetVersion::Py315,
        _ => return Err(Error::Invalid("unknown context-test parser target".into())),
    };
    let start = case
        .source
        .rfind(&case.needle)
        .ok_or_else(|| Error::Invalid("the test occurrence must exist".into()))?;
    let end = start + case.needle.len();
    write_queries(root, case, start, end)?;
    let source = root.join(&case.name);
    std::fs::create_dir_all(&source)?;
    std::fs::write(source.join(&case.file), &case.source)?;
    let report = factory::local(
        ryo::linter::Languages(vec![ryo_python::linked_with_settings(
            PythonParserSettings::new(target),
        )]),
        None,
    )
    .run_prepared(WorkerRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(
            super::prepared_engine_test::fixture_cohort(&[
                "INPUT",
                "FLAG",
                "EAGER",
                "POSTPONED",
                "DEFERRED",
                "UNEVALUATED",
            ]),
        ),
        rule_observations:
            ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context:
            ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots:
            ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        project_context_mode: ProjectContextMode::Isolated,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: source,
        rules: root.join("rules"),
        query_modules: super::prepared_engine_test::module_names(modules.clone()),
        rule_applicability: BTreeMap::default(),
        parser_configuration:
            ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language:
            ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        files: vec![ryo::linter::RelativePath(case.file.clone().into())],
        state: WorkerState::Fresh,
    })?;
    assert!(report.skipped.is_empty());
    let mut actual = report
        .diagnostics
        .iter()
        .map(|finding| {
            (
                finding.rule.as_ref().to_owned(),
                finding.start.0,
                finding.end.0,
            )
        })
        .collect::<Vec<_>>();
    let mut expected = vec![("INPUT".to_owned(), start, end)];
    if case.feature {
        expected.push(("FLAG".to_owned(), start, end));
    }
    if !case.evaluation.is_empty() {
        expected.push((case.evaluation.to_uppercase(), start, end));
    }
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected, "{}: {}", case.name, case.source);
    Ok(())
}
