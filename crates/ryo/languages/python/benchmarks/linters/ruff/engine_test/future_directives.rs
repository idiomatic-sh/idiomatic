//! Actual SDK projection distinguishes generic lazy validity from dedicated future codes.
use super::PreparedEngine as _;
use super::constants::{
    ALT, FROM_FUTURE_IMPORT_NOT_A_FEATURE_V, INPUT_PY, INVALID_SYNTAX,
    LAZY_FROM_FUTURE_IMPORT_ANNOTATION, LAZY_IMPORT_COLLECTIONS_VALUE_TRUE,
    LET_TRUTH_SELECT_ID_FILE_SPAN_FROM, LIT, LIVE_SELECTED_RULE_POSITIVE, PYTHON_TARGET_3_15,
    QUERY_RYOQL, RULES, TRUE, VALUE_TRUE_FROM_FUTURE_IMPORT_ANNO,
};
use super::*;

#[test]
fn lazy_future_stays_visible_without_a_dedicated_selected_rule() -> Result<(), Error> {
    for selected in [LIT, ALT] {
        for (source, generic) in [
            (LAZY_FROM_FUTURE_IMPORT_ANNOTATION, true),
            (LAZY_IMPORT_COLLECTIONS_VALUE_TRUE, false),
            (VALUE_TRUE_FROM_FUTURE_IMPORT_ANNO, false),
            (FROM_FUTURE_IMPORT_NOT_A_FEATURE_V, false),
        ] {
            let root = tempfile::tempdir()?;
            let rules = root.path().join(RULES);
            let directory = rules.join(selected);
            std::fs::create_dir_all(&directory)?;
            std::fs::write(
                directory.join(QUERY_RYOQL),
                LET_TRUTH_SELECT_ID_FILE_SPAN_FROM,
            )?;
            std::fs::write(root.path().join(INPUT_PY), source)?;
            let languages = ryo::linter::Languages(vec![ryo_python::linked_with_settings(
                ryo_python::settings::PythonParserSettings::new(
                    ryo_python::settings::PythonTargetVersion::Py315,
                ),
            )]);
            let report = factory::local_with_frontend_diagnostic(
                languages,
                None,
                frontend_projection(EngineScenario::Finding)?,
            )
            .run_prepared(WorkerRequest {
                performance: Default::default(),
                selected_rules: Some(fixture_cohort(&[selected])),
                rule_observations: None,
                rule_context: None,
                project_search_roots: None,
                given_project_inventory: Default::default(),
                project_context_mode:
                    ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
                context_manifest: None,
                root: root.path().to_owned(),
                rules,
                query_modules: BTreeMap::default(),
                rule_applicability: BTreeMap::default(),
                parser_configuration: Some(PYTHON_TARGET_3_15.to_owned()),
                source_language: None,
                files: vec![ryo::linter::RelativePath(INPUT_PY.into())],
                state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
            })?;
            assert_eq!(report.files, [ryo::linter::RelativePath(INPUT_PY.into())]);
            assert_eq!(report.diagnostics.len(), 1 + usize::from(generic));
            let literal = report
                .diagnostics
                .iter()
                .find(|finding| finding.rule.as_ref() == selected)
                .expect(LIVE_SELECTED_RULE_POSITIVE);
            let start = source.find(TRUE).unwrap();
            assert_eq!((literal.start.0, literal.end.0), (start, start + 4));
            let syntax: Vec<_> = report
                .diagnostics
                .iter()
                .filter(|finding| finding.rule.as_ref() == INVALID_SYNTAX)
                .collect();
            assert_eq!(syntax.len(), usize::from(generic));
            if generic {
                assert_eq!((syntax[0].start.0, syntax[0].end.0), (0, 39));
            }
        }
    }
    Ok(())
}
