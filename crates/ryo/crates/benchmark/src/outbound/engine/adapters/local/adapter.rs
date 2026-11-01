//! Query loading, compilation and SDK startup occur inside the measured child.
use super::constants;
use crate::domain::diagnostics::{
    errors::DiagnosticsError as Error,
    types::{BenchmarkFinding, RunReport},
};
use crate::domain::suite::service::relative_path;
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::engine::factory::FrontendDiagnosticProjection;
use ryo::linter::{
    self, Cache, CaseSource, Cell, Corpus, Languages, LintRequest, MemoryBudget, Rule,
    RuleApplicability, RuleId, Slot, SourceText, WorkerPool, Workers,
};
use ryo::query::RyoQlQuery;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use crate::outbound::engine::interface::EnginePort;

/// Runs authored queries through the SDK with the injected language registry.
#[derive(Debug)]
pub(crate) struct EngineLocalAdapter {
    /// Frontends selected by the outer language composition.
    languages: Languages,
    /// Optional trace destination for a separately labelled profiling invocation.
    trace: Option<std::path::PathBuf>,
    /// Optional explicit query mapping provider diagnostics to benchmark findings.
    frontend_diagnostic: Option<FrontendDiagnosticProjection>,
}
impl EngineLocalAdapter {
    /// Bind language support and optional profiling output once at composition.
    pub(crate) fn new(
        languages: Languages,
        trace: Option<std::path::PathBuf>,
        frontend_diagnostic: Option<FrontendDiagnosticProjection>,
    ) -> Self {
        Self {
            languages,
            trace,
            frontend_diagnostic,
        }
    }
    /// Load all selected query files, refusing duplicate or empty inventories.
    fn load_rules(
        directory: &std::path::Path,
        registry: &BTreeMap<String, std::path::PathBuf>,
        applicability_by_rule: &BTreeMap<String, String>,
        frontend_diagnostic: Option<FrontendDiagnosticProjection>,
    ) -> Result<BTreeMap<RuleId, Rule>, Error> {
        let profile = directory
            .parent()
            .ok_or_else(|| Error::Invalid("rules directory has no profile root".into()))?;
        let root = profile.canonicalize()?;
        let mut modules = Vec::with_capacity(registry.len());
        for (id, relative) in registry {
            let _module = ryo::query::QueryModuleSource::try_new(id, "module source")
                .map_err(|error| Error::Invalid(error.to_string()))?;
            relative_path(relative)?;
            let mut current = root.clone();
            for component in relative.components() {
                current.push(component);
                let metadata = std::fs::symlink_metadata(&current)?;
                if metadata.file_type().is_symlink() {
                    return Err(Error::Invalid(format!(
                        "query module is a symlink: {}",
                        relative.display()
                    )));
                }
            }
            let canonical = current.canonicalize()?;
            if !canonical.starts_with(&root) || !canonical.is_file() {
                return Err(Error::Invalid(format!(
                    "query module is outside profile or not a file: {}",
                    relative.display()
                )));
            }
            let source = std::fs::read_to_string(canonical)?;
            modules.push(
                ryo::query::QueryModuleSource::try_new(id, &source)
                    .map_err(|error| Error::Invalid(error.to_string()))?,
            );
        }
        let mut rules = BTreeMap::new();
        let mut mapped_rules = std::collections::BTreeSet::new();
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let id = entry
                .file_name()
                .into_string()
                .map_err(|_name| Error::Invalid(constants::NON_UTF8_RULE.into()))?;
            let query = std::fs::read_to_string(entry.path().join(constants::QUERY_FILE))?;
            let applicability = match applicability_by_rule.get(&id).map(String::as_str) {
                Some("always") => RuleApplicability::Always,
                Some("accepted-syntax") => RuleApplicability::AcceptedSyntax,
                Some(value) => {
                    return Err(Error::Invalid(format!(
                        "unsupported applicability for {id}: {value}"
                    )));
                }
                None if applicability_by_rule.is_empty() => RuleApplicability::Always,
                None => {
                    return Err(Error::Invalid(format!(
                        "selected rule lacks explicit applicability: {id}"
                    )));
                }
            };
            let _new_mapping = mapped_rules.insert(id.clone());
            let id =
                RuleId::try_from(id.as_str()).map_err(|error| Error::Invalid(error.to_string()))?;
            let query = if modules.is_empty() {
                RyoQlQuery::try_from(query.as_str())
                    .map_err(|error| Error::Invalid(error.to_string()))?
            } else {
                RyoQlQuery::with_modules(&format!("rules/{id}"), &query, modules.clone())
                    .map_err(|error| Error::Invalid(error.to_string()))?
            };
            if rules
                .insert(
                    id,
                    Rule {
                        query,
                        applicability,
                        parameters: BTreeMap::new(),
                        include: vec![],
                        exclude: vec![],
                    },
                )
                .is_some()
            {
                return Err(Error::Invalid(constants::DUPLICATE_RULE.into()));
            }
        }
        if rules.is_empty() {
            return Err(Error::Invalid(constants::EMPTY_RULES.into()));
        }
        if !applicability_by_rule.is_empty()
            && mapped_rules != applicability_by_rule.keys().cloned().collect()
        {
            return Err(Error::Invalid(
                "applicability map does not exactly match selected candidate rules".into(),
            ));
        }
        if let Some(projection) = frontend_diagnostic
            && rules
                .insert(
                    projection.rule,
                    Rule {
                        query: projection.query,
                        applicability: RuleApplicability::Always,
                        parameters: BTreeMap::new(),
                        include: vec![],
                        exclude: vec![],
                    },
                )
                .is_some()
        {
            return Err(Error::Invalid(constants::DUPLICATE_RULE.into()));
        }
        Ok(rules)
    }

    /// Refuse skips that do not follow the sealed syntax-applicability policy.
    fn validate_skips(
        skipped: &[linter::Skipped],
        selected_rules: &std::collections::BTreeSet<RuleId>,
        applicability_by_rule: &BTreeMap<String, String>,
        files: &std::collections::BTreeSet<linter::RelativePath>,
        frontend_diagnostic_enabled: bool,
    ) -> Result<(), Error> {
        let mut seen = std::collections::BTreeSet::new();
        for skip in skipped {
            if skip.reason != linter::SkipReason::SyntaxRejected
                || !files.contains(&skip.path)
                || !selected_rules.contains(&skip.rule)
                || applicability_by_rule
                    .get(skip.rule.as_ref())
                    .map(String::as_str)
                    != Some("accepted-syntax")
                || !seen.insert((skip.rule.clone(), skip.path.clone()))
            {
                return Err(Error::Invalid(format!(
                    "invalid or unexplained applicability skip: {skip:?}"
                )));
            }
        }
        if !skipped.is_empty() && !frontend_diagnostic_enabled {
            return Err(Error::Invalid(
                "applicability skips require frontend diagnostic projection".into(),
            ));
        }
        Ok(())
    }
}
impl EnginePort for EngineLocalAdapter {
    fn run(&self, request: WorkerRequest) -> Result<RunReport, Error> {
        use tracing_subscriber::prelude::*;
        let _trace = if let Some(path) = &self.trace {
            let (layer, guard) = tracing_chrome::ChromeLayerBuilder::new().file(path).build();
            tracing::subscriber::set_global_default(
                tracing_subscriber::registry()
                    .with(tracing_subscriber::filter::LevelFilter::INFO)
                    .with(layer),
            )
            .map_err(|error| Error::Invalid(error.to_string()))?;
            Some(guard)
        } else {
            None
        };
        let rules = Self::load_rules(
            &request.rules,
            &request.query_modules,
            &request.rule_applicability,
            self.frontend_diagnostic.clone(),
        )?;
        let selected_rules = rules.keys().cloned().collect();
        let selected_rule_count = rules
            .len()
            .saturating_sub(usize::from(self.frontend_diagnostic.is_some()));
        let selected_pairs = selected_rule_count.saturating_mul(request.files.len());
        let mut sources = Vec::new();
        let mut accepted = std::collections::BTreeSet::new();
        for file in &request.files {
            relative_path(file.0.as_ref())?;
            if !accepted.insert(file.clone()) {
                return Err(Error::Invalid(
                    crate::domain::diagnostics::constants::DUPLICATE_INPUT.into(),
                ));
            }
            let source = std::fs::read_to_string(request.root.join(file.0.as_ref()))?;
            sources.push(CaseSource {
                path: file.clone(),
                text: SourceText(source.into_boxed_str()),
            });
        }
        if sources.is_empty() {
            return Err(Error::Invalid(constants::EMPTY_CORPUS.into()));
        }
        let pool = WorkerPool::from(Workers(NonZeroUsize::MIN));
        let cache = match request.state {
            crate::domain::suite::types::WorkerState::Fresh => Cache::None,
            crate::domain::suite::types::WorkerState::Retained(path) => Cache::At(path),
        };
        let report = linter::lint(
            self.languages.clone(),
            &pool,
            MemoryBudget::Machine,
            cache,
            LintRequest {
                root: request.root,
                rules,
                files: vec![],
                hosted: vec![],
                corpus: Corpus::Given(sources),
                beside: vec![],
            },
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        if !report.unknowns.is_empty() || !report.unread.is_empty() {
            return Err(Error::Invalid(format!(
                "incomplete candidate: selected_rule_file_pairs={selected_pairs}; unknown_count={}; unread_count={}; unknowns={:?}; unread={:?}",
                report.unknowns.len(),
                report.unread.len(),
                report.unknowns,
                report.unread
            )));
        }
        let input_files: std::collections::BTreeSet<_> = accepted.iter().cloned().collect();
        Self::validate_skips(
            &report.skipped,
            &selected_rules,
            &request.rule_applicability,
            &input_files,
            self.frontend_diagnostic.is_some(),
        )?;
        let skips_count = report.skipped.len();
        let mut diagnostics = Vec::new();
        for finding in report.findings {
            let Slot::Is(Cell::File(file)) = finding.subject else {
                return Err(Error::Invalid(constants::INVALID_SUBJECT.into()));
            };
            let spans: Vec<_> = finding
                .columns
                .iter()
                .filter_map(|column| {
                    if column.name.0.as_ref() == constants::SPAN_COLUMN
                        && let Slot::Is(Cell::Span(span)) = &column.value
                    {
                        return Some(*span);
                    }
                    None
                })
                .collect();
            let [span] = spans.as_slice() else {
                return Err(Error::Invalid(constants::INVALID_SPAN.into()));
            };
            diagnostics.push(BenchmarkFinding {
                rule: finding.rule,
                file,
                start: crate::domain::diagnostics::types::SourceByteOffset(
                    usize::try_from(span.start.0)
                        .map_err(|error| Error::Invalid(error.to_string()))?,
                ),
                end: crate::domain::diagnostics::types::SourceByteOffset(
                    usize::try_from(span.end.0)
                        .map_err(|error| Error::Invalid(error.to_string()))?,
                ),
            });
        }
        Ok(RunReport {
            diagnostics,
            skipped: report.skipped,
            coverage: Some(crate::domain::diagnostics::types::RuleCoverage {
                selected: selected_pairs,
                evaluated: selected_pairs.saturating_sub(skips_count),
                skipped: skips_count,
                unknown_or_unread: 0,
            }),
            files: accepted.into_iter().collect(),
        })
    }
}

#[cfg(test)]
#[expect(
    clippy::inline_modules,
    reason = "these worker registry tests stay next to the local adapter contract"
)]
mod query_module_tests {
    use super::{EngineLocalAdapter, RuleApplicability, RuleId, linter};
    use std::{collections::BTreeMap, path::PathBuf};

    fn profile() -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().expect("profile root");
        let rules = root.path().join("rules");
        std::fs::create_dir_all(rules.join("F401")).expect("rule directory");
        std::fs::write(
            rules.join("F401/query.ryoql"),
            "import shared from \"python/ruff/suppress\";",
        )
        .expect("query source");
        (root, rules)
    }

    #[test]
    fn worker_reads_explicit_module_text_from_the_frozen_profile() {
        let (root, rules) = profile();
        let module = root.path().join("fragments/suppress.ryoql");
        std::fs::create_dir_all(module.parent().expect("module parent")).expect("module folder");
        let text = "input source: relation<id: node>;\n";
        std::fs::write(&module, text).expect("module source");
        let registry = BTreeMap::from([(
            "python/ruff/suppress".to_owned(),
            PathBuf::from("fragments/suppress.ryoql"),
        )]);
        let queries = EngineLocalAdapter::load_rules(&rules, &registry, &BTreeMap::default(), None)
            .expect("module bundle is loaded in the worker");
        assert_eq!(queries.len(), 1);
        assert_eq!(
            queries.values().next().expect("rule").query.root_text(),
            "import shared from \"python/ruff/suppress\";"
        );
        assert_eq!(
            queries.values().next().expect("rule").applicability,
            RuleApplicability::Always
        );
    }

    #[test]
    fn selected_rules_receive_the_pinned_syntax_applicability() {
        let (_root, rules) = profile();
        let applicability = BTreeMap::from([("F401".to_owned(), "accepted-syntax".to_owned())]);
        let queries =
            EngineLocalAdapter::load_rules(&rules, &BTreeMap::default(), &applicability, None)
                .expect("complete applicability map loads");
        assert_eq!(
            queries.values().next().expect("rule").applicability,
            RuleApplicability::AcceptedSyntax
        );
    }

    #[test]
    fn explicit_applicability_must_match_every_loaded_rule() {
        let (_root, rules) = profile();
        let applicability = BTreeMap::from([
            ("F401".to_owned(), "accepted-syntax".to_owned()),
            ("F402".to_owned(), "always".to_owned()),
        ]);
        let error =
            EngineLocalAdapter::load_rules(&rules, &BTreeMap::default(), &applicability, None)
                .expect_err("extra applicability entry is refused");
        assert!(error.to_string().contains("does not exactly match"));
    }

    #[test]
    fn explicit_applicability_rejects_unknown_policy_values() {
        let (_root, rules) = profile();
        let applicability = BTreeMap::from([("F401".to_owned(), "parse-only".to_owned())]);
        let error =
            EngineLocalAdapter::load_rules(&rules, &BTreeMap::default(), &applicability, None)
                .expect_err("unknown applicability is refused");
        assert!(error.to_string().contains("unsupported applicability"));
    }

    #[test]
    fn typed_syntax_skips_require_matching_policy_input_and_projection() -> Result<(), String> {
        let (_root, _directory) = profile();
        let path = linter::RelativePath("input.py".into());
        let skipped = linter::Skipped {
            rule: RuleId::try_from("F401").map_err(|error| error.to_string())?,
            path: path.clone(),
            reason: linter::SkipReason::SyntaxRejected,
        };
        let files = std::collections::BTreeSet::from([path]);
        let policy = BTreeMap::from([("F401".to_owned(), "accepted-syntax".to_owned())]);
        let selected = std::collections::BTreeSet::from([
            RuleId::try_from("F401").map_err(|error| error.to_string())?
        ]);
        assert!(
            EngineLocalAdapter::validate_skips(
                &[skipped.clone()],
                &selected,
                &policy,
                &files,
                true
            )
            .is_ok()
        );
        assert!(
            EngineLocalAdapter::validate_skips(
                &[skipped.clone()],
                &selected,
                &policy,
                &files,
                false
            )
            .is_err()
        );
        assert!(
            EngineLocalAdapter::validate_skips(
                &[skipped.clone(), skipped.clone()],
                &selected,
                &policy,
                &files,
                true
            )
            .is_err()
        );

        assert!(
            EngineLocalAdapter::validate_skips(
                &[skipped.clone()],
                &selected,
                &BTreeMap::new(),
                &files,
                true
            )
            .is_err()
        );
        let other_file = std::collections::BTreeSet::from([linter::RelativePath("else.py".into())]);
        assert!(
            EngineLocalAdapter::validate_skips(&[skipped], &selected, &policy, &other_file, true)
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn worker_refuses_module_paths_outside_the_profile() {
        let (_root, rules) = profile();
        let registry = BTreeMap::from([(
            "python/ruff/suppress".to_owned(),
            PathBuf::from("../outside.ryoql"),
        )]);
        let error = EngineLocalAdapter::load_rules(&rules, &registry, &BTreeMap::default(), None)
            .expect_err("traversal is refused");
        assert!(error.to_string().contains("unsafe corpus path"));
    }

    #[cfg(unix)]
    #[test]
    fn worker_refuses_symlinked_module_paths() {
        let (root, rules) = profile();
        std::fs::create_dir_all(root.path().join("fragments")).expect("module folder");
        let target = root.path().join("outside.ryoql");
        std::fs::write(&target, "module bytes").expect("module source");
        std::os::unix::fs::symlink(&target, root.path().join("fragments/suppress.ryoql"))
            .expect("symlink fixture");
        let registry = BTreeMap::from([(
            "python/ruff/suppress".to_owned(),
            PathBuf::from("fragments/suppress.ryoql"),
        )]);
        let error = EngineLocalAdapter::load_rules(&rules, &registry, &BTreeMap::default(), None)
            .expect_err("symlink is refused");
        assert!(error.to_string().contains("query module is a symlink"));
    }
}
