//! Actual SDK projection distinguishes generic lazy validity from dedicated future codes.
use super::PreparedEngine as _;
use super::*;

#[test]
fn lazy_future_stays_visible_without_a_dedicated_selected_rule() -> Result<(), Error> {
    for selected in ["LIT", "ALT"] {
        for (source, generic) in [
            (
                "lazy from __future__ import annotations\nvalue=True\n",
                true,
            ),
            ("lazy import collections\nvalue=True\n", false),
            ("value=True\nfrom __future__ import annotations\n", false),
            ("from __future__ import not_a_feature\nvalue=True\n", false),
        ] {
            let root = tempfile::tempdir()?;
            let rules = root.path().join("rules");
            let directory = rules.join(selected);
            std::fs::create_dir_all(&directory)?;
            std::fs::write(
                directory.join("query.ryoql"),
                "let truth = select id, file, span from \"literal-properties\" where kind = \"boolean\" and boolean = \"true\"; emit truth subject file key(span);",
            )?;
            std::fs::write(root.path().join("input.py"), source)?;
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
                    ryo_benchmark::domain::suite::types::ProjectContextMode::default(),
                context_manifest: None,
                root: root.path().to_owned(),
                rules,
                query_modules: BTreeMap::default(),
                rule_applicability: BTreeMap::default(),
                parser_configuration: Some("python-target=3.15".to_owned()),
                source_language: None,
                files: vec![ryo::linter::RelativePath("input.py".into())],
                state: ryo_benchmark::domain::suite::types::WorkerState::Fresh,
            })?;
            assert_eq!(report.files, [ryo::linter::RelativePath("input.py".into())]);
            assert_eq!(report.diagnostics.len(), 1 + usize::from(generic));
            let literal = report
                .diagnostics
                .iter()
                .find(|finding| finding.rule.as_ref() == selected)
                .expect("live selected-rule positive");
            let start = source.find("True").unwrap();
            assert_eq!((literal.start.0, literal.end.0), (start, start + 4));
            let syntax: Vec<_> = report
                .diagnostics
                .iter()
                .filter(|finding| finding.rule.as_ref() == "invalid-syntax")
                .collect();
            assert_eq!(syntax.len(), usize::from(generic));
            if generic {
                assert_eq!((syntax[0].start.0, syntax[0].end.0), (0, 39));
            }
        }
    }
    Ok(())
}
