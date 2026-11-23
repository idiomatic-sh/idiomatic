//! The sparse-traversal guards: each rooted query answers the global closure's counted multiset, by membership, across
//! unrelated width and depth growth, and the actual rule queries keep their rooted axis.
use super::constants::{
    A_PY, ACCEPTED_SYNTAX, B_PY, CASE_PY, PYTHON_TARGET_3_10, QUERY_RYOQL, ROOTED001, RULES,
    SPARSE_NONEMPTY_AWAIT, SPARSE_PIE794_RELEVANT, SPARSE_PLR1708_RELEVANT, SPARSE_UNRELATED_AWAIT,
};
use super::prepared_engine_test::{
    PreparedEngine as _, default_frontend_projection, fixture_cohort, python_310,
};
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{
    CoverageAccounting, ExecutionMetadata, PairCount, RuleCoverage,
};
use ryo_benchmark_contracts::domain::requests::types::{
    ApplicabilityPolicy, CohortSelection, ContextSeal, InputDeclaration, ParserConfigurationName,
    ParserSelection, ProviderSelection, RuleContextPreparation, RuleLabel, SearchRoots,
    WorkerRequest, WorkerState,
};
use std::collections::BTreeMap;

/// The PIE794 equivalence: the rooted axis against the global closure over the same roots.
const PIE794_EQUIVALENCE: &str =
    include_str!("../../../../tests/fixtures/ryoql-fragments/pie794-rooted/equivalence.ryoql");
/// The B023 and C419 consumers' equivalence.
const SPARSE_DESCENDANT_EQUIVALENCE: &str = include_str!(
    "../../../../tests/fixtures/ryoql-fragments/sparse-descendant-consumers/equivalence.ryoql"
);
/// The PLR1708 equivalence.
const PLR1708_EQUIVALENCE: &str =
    include_str!("../../../../tests/fixtures/ryoql-fragments/plr1708-rooted/equivalence.ryoql");
/// The actual rule queries whose roots stay sparse.
const PIE794_QUERY: &str = include_str!("../rules/PIE794/query.ryoql");
const PLR1708_QUERY: &str = include_str!("../rules/PLR1708/query.ryoql");
const B023_QUERY: &str = include_str!("../rules/B023/query.ryoql");
const C419_QUERY: &str = include_str!("../rules/C419/query.ryoql");

/// One corpus: its name and its files.
type Corpus = (String, Vec<(&'static str, String)>);

/// The relevant program alone, the same program in two files, then three widths and three depths of unrelated growth.
fn grown(relevant: &str, extra: Vec<Corpus>) -> Vec<Corpus> {
    let mut corpora = vec![
        (
            "branch-order-and-two-contexts".to_owned(),
            vec![(CASE_PY, relevant.to_owned())],
        ),
        (
            "two-files-same-ids".to_owned(),
            vec![(A_PY, relevant.to_owned()), (B_PY, relevant.to_owned())],
        ),
    ];
    corpora.extend(extra);
    for width in [16, 256, 1024] {
        corpora.push((
            format!("unrelated-width-{width}"),
            vec![(
                CASE_PY,
                format!("{relevant}unrelated = [{}]\n", vec!["0"; width].join(",")),
            )],
        ));
    }
    for depth in [4, 16, 48] {
        corpora.push((
            format!("unrelated-depth-{depth}"),
            vec![(
                CASE_PY,
                format!(
                    "{relevant}unrelated = {}0{}\n",
                    "[".repeat(depth),
                    "]".repeat(depth)
                ),
            )],
        ));
    }
    corpora
}

/// Run the equivalence query over every corpus; each must report nothing and account for every rule/file pair.
fn equivalent(query: &str, corpora: &[Corpus]) -> Result<usize, Error> {
    for (name, files) in corpora {
        let root = tempfile::tempdir()?;
        let rules = root.path().join(RULES);
        let rule = rules.join(ROOTED001);
        std::fs::create_dir_all(&rule)?;
        std::fs::write(rule.join(QUERY_RYOQL), query)?;
        for (file, source) in files {
            std::fs::write(root.path().join(file), source)?;
        }
        let paths = files
            .iter()
            .map(|(file, _)| ryo::linter::RelativePath((*file).into()))
            .collect::<Vec<_>>();
        let report = factory::local_with_frontend_diagnostic(
            python_310(),
            None,
            default_frontend_projection()?,
        )
        .run_prepared(WorkerRequest {
            performance: Default::default(),
            selected_rules: CohortSelection::Explicit(fixture_cohort(&[ROOTED001])),
            rule_observations: InputDeclaration::Undeclared,
            rule_context: RuleContextPreparation::Unconfigured,
            project_search_roots: SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            project_context_mode: ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
            context_manifest: ContextSeal::Unsealed,
            root: root.path().to_owned(),
            rules,
            query_modules: BTreeMap::default(),
            rule_applicability: BTreeMap::from([(
                RuleLabel(ROOTED001.into()),
                ApplicabilityPolicy(ACCEPTED_SYNTAX.into()),
            )]),
            parser_configuration: ParserSelection::Named(ParserConfigurationName(
                PYTHON_TARGET_3_10.into(),
            )),
            source_language: ProviderSelection::Unspecified,
            files: paths.clone(),
            state: WorkerState::Fresh,
        })?;
        let mut sorted = paths;
        sorted.sort();
        assert_eq!(
            report.execution_metadata,
            ExecutionMetadata::Complete,
            "{name}"
        );
        assert_eq!(report.files, sorted, "{name}");
        assert_eq!(
            report.coverage,
            CoverageAccounting::Accounted(RuleCoverage {
                selected: PairCount(files.len()),
                evaluated: PairCount(files.len()),
                skipped: PairCount(0),
                unknown_or_unread: PairCount(0),
            }),
            "{name}"
        );
        assert!(report.skipped.is_empty(), "{name}");
        assert!(
            report.diagnostics.is_empty(),
            "{name}: {:?}",
            report.diagnostics
        );
    }
    Ok(corpora.len())
}

/// The PIE794 rooted axis answers the global closure on 8 corpora of unrelated growth.
pub(super) fn pie794_rooted_equivalence_reports_nothing_across_unrelated_growth()
-> Result<(), Error> {
    let corpora = grown(SPARSE_PIE794_RELEVANT, Vec::new());
    assert_eq!(equivalent(PIE794_EQUIVALENCE, &corpora)?, 8);
    Ok(())
}

/// The B023 and C419 consumers' rooted axes answer the global closure on 10 corpora, two with awaits.
pub(super) fn sparse_descendant_consumers_equivalence_reports_nothing_across_unrelated_growth()
-> Result<(), Error> {
    let corpora = grown(
        SPARSE_PIE794_RELEVANT,
        vec![
            (
                "nonempty-await-domain".to_owned(),
                vec![(CASE_PY, SPARSE_NONEMPTY_AWAIT.to_owned())],
            ),
            (
                "unrelated-await-outside-roots".to_owned(),
                vec![(
                    CASE_PY,
                    format!("{SPARSE_PIE794_RELEVANT}{SPARSE_UNRELATED_AWAIT}"),
                )],
            ),
        ],
    );
    assert_eq!(equivalent(SPARSE_DESCENDANT_EQUIVALENCE, &corpora)?, 10);
    Ok(())
}

/// The PLR1708 rooted walk answers the global closure on 8 corpora, its scope boundaries filtered.
pub(super) fn plr1708_rooted_equivalence_reports_nothing_across_unrelated_growth()
-> Result<(), Error> {
    let mut corpora = grown(SPARSE_PLR1708_RELEVANT, Vec::new());
    corpora[0].0 = "filtered-boundaries".to_owned();
    corpora[1].0 = "two-real-file-identities".to_owned();
    assert_eq!(equivalent(PLR1708_EQUIVALENCE, &corpora)?, 8);
    Ok(())
}

/// The actual PIE794, PLR1708, B023 and C419 queries keep a rooted axis, and never a closure. PIE794 and C419 walk
/// down from their candidates; PLR1708 and B023 start from their rare side (#64) and walk up: PLR1708 from each
/// `StopIteration` raise to its nearest member mark, B023 from each function, and each used binding, to its loops.
pub(super) fn actual_rooted_queries_keep_their_sparse_axis() -> Result<(), Error> {
    assert!(PIE794_QUERY.contains(
        r#"axis(from=candidate_values, relation="semantic-child", direction=descendant, domain=names)"#
    ));
    assert!(!PIE794_QUERY.contains(r#"let expression_paths = closure(edges="semantic-child""#));
    assert!(PLR1708_QUERY.contains(
        r#"axis(from=stop_raises, relation="semantic-child", direction=nearest-ancestor, domain=member_marks)"#
    ));
    assert!(!PLR1708_QUERY.contains("closure("));
    assert!(C419_QUERY.contains(
        r#"axis(from=candidate_elements, relation="semantic-child", direction=descendant, domain=awaits)"#
    ));
    for (from, domain) in [
        ("functions", "loops"),
        ("used_binding_ids", "target_root_ids"),
        ("used_binding_ids", "body_root_ids"),
    ] {
        assert!(B023_QUERY.contains(&format!(
            r#"axis(from={from}, relation="semantic-child", direction=ancestor, domain={domain})"#
        )));
    }
    assert!(!B023_QUERY.contains("closure("));
    Ok(())
}
