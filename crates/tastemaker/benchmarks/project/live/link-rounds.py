"""Install the cooperative fixture driver in the pinned benchmark copy."""
from pathlib import Path


def replace(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new)


def link_rounds(core: Path, library: Path):
    interface = core / "src/outbound/analysis/interface.rs"
    s = interface.read_text()
    at = "    /// Every relation a query may read"
    s = replace(s, at, """    /// The benchmark's ordered batch, with one result per request.
    fn lint_benchmark_round(
        &self, requests: Vec<AnalysisRequest>,
    ) -> Vec<Result<AnalysisReport, AnalysisError>> {
        requests.into_iter().map(|request| self.lint(request)).collect()
    }

""" + at)
    interface.write_text(s)

    at = core / "src/inbound/use_cases/validate/use_case.rs"
    s = at.read_text()
    at.with_name("round_host.rs").write_bytes((library / "round-host.rs").read_bytes())
    # Keep module docs at the start of the source.
    s = replace(s, "impl ValidateUseCase for ValidateUseCaseImpl {",
                '#[path = "round_host.rs"]\nmod rounds;\n\nimpl ValidateUseCase for ValidateUseCaseImpl {')
    for method in (
        "prove", "template_proof", "template_proof_cases", "rule_proof", "rule_cases",
        "project_cases", "verdicts", "project_case", "spec_cases", "template_cases",
        "checked", "project_lint", "linted", "project_counted",
    ):
        s = replace(s, f"    fn {method}(", f"    async fn {method}(")
    for before, after in (
        ("self.rule_proof(&mut problems, &label, version, &version.release, &rule)?",
         "self.rule_proof(&mut problems, &label, version, &version.release, &rule).await?"),
        ("self.rule_proof(&mut problems, &owned, version, &release, &rule)?",
         "self.rule_proof(&mut problems, &owned, version, &release, &rule).await?"),
        ("self.template_proof(version, &owned, &release, &template)?", "self.template_proof(version, &owned, &release, &template).await?"),
        ("self.spec_cases(version, &spec)?", "self.spec_cases(version, &spec).await?"),
        ("                    &template,\n                )?", "                    &template,\n                ).await?"),
        ("self.template_proof_cases(version, label, release, template)?", "self.template_proof_cases(version, label, release, template).await?"),
        ("self.template_cases(version, release, template)?", "self.template_cases(version, release, template).await?"),
        ("self.rule_cases(version, release, rule)?", "self.rule_cases(version, release, rule).await?"),
        ("self.verdicts(&version.package, rows)?", "self.verdicts(&version.package, rows).await?"),
        ("self.linted(root, plan, &given, corpus, findings, refused)", "self.linted(root, plan, &given, corpus, findings, refused).await"),
    ):
        s = replace(s, before, after)
    assert s.count("            &scenarios,\n        )") == 3
    s = s.replace("            &scenarios,\n        )", "            &scenarios,\n        ).await")
    lo = s.index("                for VersionFile {", s.index("    async fn prove("))
    hi = s.index("            }\n            DefinitionKind::Template", lo)
    s = s[:lo] + """                let owned = rounds::ordered(files.iter().skip(1).map(|VersionFile { path, text }| async {
                    let owned = owned_label(&label, path);
                    let mut problems = Vec::new();
                    if path.0.ends_with(kind_file(DefinitionKind::Rule).0) {
                        match self.manifest.rule(&DefinitionText(text.0.clone())) {
                            Ok(rule) => {
                                let release = owned_release(&version.release, DefinitionKind::Rule, path);
                                self.rule_proof(&mut problems, &owned, version, &release, &rule).await?;
                            }
                            Err(reason) => problems.push(Problem(format!("`{}`: {reason}", path.0).into())),
                        }
                    }
                    if path.0.ends_with(kind_file(DefinitionKind::Template).0) {
                        match self.manifest.template(&DefinitionText(text.0.clone())) {
                            Ok(template) => {
                                let release = owned_release(&version.release, DefinitionKind::Template, path);
                                problems.extend(self.template_proof(version, &owned, &release, &template).await?);
                            }
                            Err(reason) => problems.push(Problem(format!("`{}`: {reason}", path.0).into())),
                        }
                    }
                    Ok::<_, ValidateError>(problems)
                }));
                let (owned, scenarios) = rounds::together(owned, self.spec_cases(version, &spec)).await;
                for each in owned {
                    problems.extend(each?);
                }
                covered(&mut problems, &label, &judged_spec(&spec));
                problems.extend(scenarios?);
""" + s[hi:]
    lo = s.index("        let completed = scenarios")
    hi = s.index("        // Every checked scenario",lo)
    s = s[:lo] + """        let completed = rounds::ordered(scenarios.iter().map(|scenario| async {
            self.project_case(
                &version.package, origin,
                Some(&FixturePlacing { version, place: place() }), scenario,
            ).await
        })).await;
""" + s[hi:]
    s = replace(s, "let outcome = (|| {", "let outcome = async {")
    s = replace(s, """                placing.map(|placing| &placing.version.release),
            )
        })();""", """                placing.map(|placing| &placing.version.release),
            ).await
        }.await;""")
    s = replace(s, "let report = self.analysis.lint(AnalysisRequest {", "let report = rounds::lint(self.analysis.as_ref(), AnalysisRequest {")
    s = replace(s, "            hosted: Hosted::Verdict(rows),\n        })?;", "            hosted: Hosted::Verdict(rows),\n        }).await?;")
    s = replace(s, "match self.analysis.lint(AnalysisRequest {", "match rounds::lint(self.analysis.as_ref(), AnalysisRequest {")
    s = replace(s, "                hosted: Hosted::Nothing,\n            }) {", "                hosted: Hosted::Nothing,\n            }).await {")
    s = replace(s, """        let report =
            self.project_lint(root, written, &plan, &corpus, &mut findings, &mut refused)?;
        findings.extend(self.project_counted(root, &plan, &report, &refused)?);""", """        let independent = rounds::enabled().then(|| {
            crate::domain::judging::service::host(
                &plan.counts, &plan.pairs, &BTreeMap::new(), &plan.closings,
            )
        }).filter(|rows| !rows.conditions.iter().any(|row| {
            plan.purposes.values().any(|purpose| {
                matches!(purpose, crate::domain::applications::types::Purpose::Condition { scope, name, .. }
                    if *scope == row.scope && *name == row.condition)
            })
        }));
        let lint = self.project_lint(root, written, &plan, &corpus, &mut findings, &mut refused);
        let (report, counted) = if let Some(rows) = independent {
            let (report, counted) = rounds::together(lint, self.project_counted(root, &plan, rows)).await;
            (report?, counted?)
        } else {
            let report = lint.await?;
            let holds = crate::domain::judging::service::holds(&plan, &refused, &report.findings);
            let rows = crate::domain::judging::service::host(&plan.counts, &plan.pairs, &holds, &plan.closings);
            (report, self.project_counted(root, &plan, rows).await?)
        };
        findings.extend(counted);""")
    lo = s.index("        report: &AnalysisReport,",s.index("    async fn project_counted("))
    hi = s.index("        if rows.entries.is_empty()",lo)
    s = s[:lo] + """        rows: crate::domain::judging::types::CountRows,
    ) -> Result<Vec<crate::domain::findings::types::Finding>, Problem> {
""" + s[hi:]
    s = replace(s, "        let counted = self\n            .analysis\n            .lint(AnalysisRequest {",
                "        let counted = rounds::lint(self.analysis.as_ref(), AnalysisRequest {")
    s = replace(s, "                hosted: Hosted::Counts(rows),\n            })\n            .map_err",
                "                hosted: Hosted::Counts(rows),\n            }).await\n            .map_err")
    lo = s.index("            made = Some(",s.index("    fn proved("))
    hi = s.index("        });",lo)
    s = s[:lo] + """            made = Some(rounds::drive(
                self.analysis.as_ref(),
                rounds::ordered(asked.iter().map(|each| async {
                    match &each.held {
                        Some(proof) => Ok(proof.clone()),
                        None => self.prove(&each.version).await,
                    }
                })),
            ));
""" + s[hi:]
    at.write_text(s)
