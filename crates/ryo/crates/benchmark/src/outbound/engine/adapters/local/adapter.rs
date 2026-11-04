//! Query loading, compilation and SDK startup occur inside the measured child.
use super::constants;
use crate::domain::diagnostics::{
    errors::DiagnosticsError as Error,
    types::{BenchmarkFinding, RunReport},
};
use crate::domain::suite::service::relative_path;
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::engine::factory::FrontendDiagnosticProjection;
use crate::outbound::engine::types::PreparedMissingInputs;
use ryo::linter::{
    self, Cache, CaseSource, Cell, Corpus, Languages, LintRequest, MemoryBudget, Rule,
    RuleApplicability, RuleId, Slot, SourceText, WorkerPool, Workers,
};
use ryo::query::RyoQlQuery;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::path::Path;
use tracing_subscriber::prelude::*;

use crate::outbound::engine::interface::EnginePort;

/// Runs authored queries through the SDK with the injected language registry.
#[derive(Debug)]
pub(crate) struct EngineLocalAdapter {
    /// Frontends selected by the outer language composition.
    languages: Languages,
    /// Optional trace destination for a separately labelled profiling invocation.
    trace: Option<std::path::PathBuf>,
    /// Optional flushed JSONL event destination for a separately labelled diagnostic invocation.
    diagnostic_events: Option<std::path::PathBuf>,
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
            diagnostic_events: None,
            frontend_diagnostic,
        }
    }
    /// Bind an explicit live JSONL diagnostic sink without changing the timing worker.
    pub(crate) fn with_diagnostic_events(
        languages: Languages,
        path: std::path::PathBuf,
        frontend_diagnostic: Option<FrontendDiagnosticProjection>,
    ) -> Self {
        Self {
            languages,
            trace: None,
            diagnostic_events: Some(path),
            frontend_diagnostic,
        }
    }
    /// Load all selected query files, refusing duplicate or empty inventories.
    fn load_rules(
        directory: &std::path::Path,
        registry: &BTreeMap<String, std::path::PathBuf>,
        applicability_by_rule: &BTreeMap<String, String>,
        frontend_diagnostic: Option<FrontendDiagnosticProjection>,
        declarations: Option<&crate::domain::suite::types::RuleObservationDeclarations>,
    ) -> Result<BTreeMap<RuleId, Rule>, Error> {
        let modules = query_modules(directory, registry)?;
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
            let observations = declarations
                .and_then(|values| values.get(&id))
                .cloned()
                .unwrap_or_default();
            if rules
                .insert(
                    id,
                    Rule {
                        observations,
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
                        observations: Vec::new(),
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
        if let Some(declarations) = declarations {
            for consumer in declarations.keys() {
                if !rules.contains_key(consumer) {
                    return Err(Error::Invalid(format!(
                        "observation declarations name an unregistered consumer: {consumer}"
                    )));
                }
            }
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
    fn run(
        &self,
        request: WorkerRequest,
        prepared: PreparedMissingInputs,
    ) -> Result<RunReport, Error> {
        let selected_missing = prepared.selected_missing;
        validate_context(&request)?;
        let _trace = if let Some(path) = &self.trace {
            let (layer, guard) = tracing_chrome::ChromeLayerBuilder::new().file(path).build();
            tracing::subscriber::set_global_default(
                tracing_subscriber::registry()
                    .with(tracing_subscriber::filter::LevelFilter::INFO)
                    .with(layer),
            )
            .map_err(|error| Error::Invalid(error.to_string()))?;
            Some(guard)
        } else if let Some(path) = &self.diagnostic_events {
            tracing::subscriber::set_global_default(super::diagnostic_events::subscriber(
                path,
                request.performance.diagnostics,
            )?)
            .map_err(|error| Error::Invalid(error.to_string()))?;
            None
        } else {
            None
        };
        let mut rules = Self::load_rules(
            &request.rules,
            &request.query_modules,
            &request.rule_applicability,
            self.frontend_diagnostic.clone(),
            request.rule_observations.as_ref(),
        )?;
        if let Some(projection) = &self.frontend_diagnostic {
            let frontend = rules
                .get_mut(&projection.rule)
                .ok_or_else(|| Error::Invalid("frontend projection was not registered".into()))?;
            frontend.exclude = frontend_exclusions(&selected_missing)?;
            frontend
                .exclude
                .extend(project_input_exclusions(&request, &self.languages)?);
        }
        let selected_rules = rules.keys().cloned().collect();
        let selected_rule_count = rules
            .len()
            .saturating_sub(usize::from(self.frontend_diagnostic.is_some()));
        let selected_pairs = selected_rule_count.saturating_mul(request.files.len());
        let configured = rule_context::project(&request, &mut rules, selected_pairs)?;
        let selected_pairs = configured.selected_pairs;
        let (sources, accepted) = source_inputs(&request, &selected_missing)?;
        let pool = WorkerPool::from(Workers(NonZeroUsize::MIN));
        let cache = match request.state {
            crate::domain::suite::types::WorkerState::Fresh => Cache::None,
            crate::domain::suite::types::WorkerState::Retained(path) => Cache::At(path),
        };
        let (corpus, beside) = match request.project_context_mode {
            crate::domain::suite::types::ProjectContextMode::Isolated => {
                (Corpus::Given(sources), vec![])
            }
            crate::domain::suite::types::ProjectContextMode::ListedDisk => {
                (Corpus::Listed(request.files.clone()), sources)
            }
        };
        let engine =
            linter::Linter::new(self.languages.clone(), pool, MemoryBudget::Machine, cache)
                .with_execution(request.performance.execution);
        let report = linter::lint_with(
            &engine,
            LintRequest {
                source_language: request
                    .source_language
                    .clone()
                    .map(|name| ryo::languages::LanguageName(name.into())),
                root: request.root.clone(),
                rules,
                files: vec![],
                hosted: configured.hosted,
                corpus,
                beside,
            },
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        let unexpected_unread = report.unread.iter().any(|unread| {
            !matches!(&unread.reason, linter::Unreadable::Failed(_))
                || !selected_missing.contains(&unread.path)
        });
        if !report.unknowns.is_empty() || unexpected_unread {
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
        let project_context = request
            .context_manifest
            .as_ref()
            .map(|manifest| {
                crate::outbound::context::adapters::local::adapter::receipt(
                    manifest,
                    &request.root,
                    report.dependencies,
                )
            })
            .transpose()?;
        let diagnostics = diagnostic_findings(report.findings)?;
        Ok(RunReport {
            execution_metadata: crate::domain::diagnostics::types::ExecutionMetadata::Complete,
            project_context,
            diagnostics,
            skipped: report.skipped,
            coverage: Some(crate::domain::diagnostics::types::RuleCoverage {
                selected: selected_pairs,
                evaluated: selected_pairs.checked_sub(skips_count).ok_or_else(|| {
                    Error::Invalid("skipped rule pairs exceed the configured selection".into())
                })?,
                skipped: skips_count,
                unknown_or_unread: 0,
            }),
            files: accepted.into_iter().collect(),
        })
    }
}

/// Load and validate every declared module inside the same sealed profile boundary.
fn query_modules(
    directory: &std::path::Path,
    registry: &BTreeMap<String, std::path::PathBuf>,
) -> Result<Vec<ryo::query::QueryModuleSource>, Error> {
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
    Ok(modules)
}

/// Require the declared context mode and exact source inventory before initializing the SDK.
fn validate_context(request: &WorkerRequest) -> Result<(), Error> {
    match (request.project_context_mode, &request.context_manifest) {
        (crate::domain::suite::types::ProjectContextMode::Isolated, None) => {}
        (crate::domain::suite::types::ProjectContextMode::ListedDisk, Some(manifest)) => {
            crate::outbound::context::adapters::local::adapter::validate(manifest, &request.root)?;
        }
        _ => {
            return Err(Error::Invalid(
                "worker context mode requires a matching explicit manifest".into(),
            ));
        }
    }
    Ok(())
}

/// Read each explicitly requested source once, retaining its exact ordered inventory.
fn source_inputs(
    request: &WorkerRequest,
    selected_missing: &std::collections::BTreeSet<linter::RelativePath>,
) -> Result<
    (
        Vec<CaseSource>,
        std::collections::BTreeSet<linter::RelativePath>,
    ),
    Error,
> {
    let mut sources = Vec::new();
    let mut accepted = std::collections::BTreeSet::new();
    for file in &request.files {
        relative_path(file.0.as_ref())?;
        if !accepted.insert(file.clone()) {
            return Err(Error::Invalid(
                crate::domain::diagnostics::constants::DUPLICATE_INPUT.into(),
            ));
        }
        if selected_missing.contains(file) {
            // Context validation already observed this absence. Keep the requested path,
            // but supply neither bytes nor metadata overlays to the engine.
            continue;
        }
        let source = std::fs::read_to_string(request.root.join(file.0.as_ref()))?;
        sources.push(CaseSource {
            executable: executable(&request.root.join(file.0.as_ref()))?,
            path: file.clone(),
            text: SourceText(source.into_boxed_str()),
        });
    }
    if accepted.is_empty() {
        return Err(Error::Invalid(constants::EMPTY_CORPUS.into()));
    }
    Ok((sources, accepted))
}

/// Decode the explicit file/span projection, refusing malformed candidate rows.
fn diagnostic_findings(findings: Vec<linter::Finding>) -> Result<Vec<BenchmarkFinding>, Error> {
    let mut diagnostics = Vec::new();
    for finding in findings {
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
                usize::try_from(span.start.0).map_err(|error| Error::Invalid(error.to_string()))?,
            ),
            end: crate::domain::diagnostics::types::SourceByteOffset(
                usize::try_from(span.end.0).map_err(|error| Error::Invalid(error.to_string()))?,
            ),
        });
    }
    Ok(diagnostics)
}

#[cfg(test)]
#[path = "query_module_tests.rs"]
mod query_module_tests;

/// Observe the actual staged file before transporting an Isolated input into memory.
fn executable(path: &Path) -> Result<ryo::linter::ExecutablePermission, Error> {
    let metadata = std::fs::metadata(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        Ok(ryo::linter::ExecutablePermission::Posix(
            metadata.permissions().mode() & 0o111 != 0,
        ))
    }
    #[cfg(not(unix))]
    {
        let _metadata = metadata;
        Ok(ryo::linter::ExecutablePermission::Unsupported)
    }
}

/// The auto frontend projection governs only declared readable inputs. Authored rules still
/// govern the complete primary inventory; an absence seal is not a statement about syntax.
pub(super) fn frontend_exclusions(
    selected_missing: &std::collections::BTreeSet<linter::RelativePath>,
) -> Result<Vec<linter::Glob>, Error> {
    selected_missing
        .iter()
        .map(|file| {
            #[cfg(unix)]
            let literal = file.0.replace('\\', "\\\\");
            #[cfg(not(unix))]
            let literal = file.0.to_string();
            let pattern = globset::escape(&literal);
            linter::Glob::try_from(pattern.as_str())
                .map_err(|error| Error::Invalid(error.to_string()))
        })
        .collect()
}

/// Syntax diagnostics apply to source inputs; provider-declared project inputs have their own reader.
fn project_input_exclusions(
    request: &WorkerRequest,
    languages: &Languages,
) -> Result<Vec<linter::Glob>, Error> {
    let matching: Vec<_> = match request.source_language.as_deref() {
        Some(selected) => languages
            .0
            .iter()
            .filter(|language| language.name().0.as_ref() == selected)
            .collect(),
        None if languages.0.len() == 1 => languages.0.iter().collect(),
        // Automatic multi-provider parsing chooses the actual accepting source provider. Without an
        // explicit unique owner, do not infer a configuration exclusion from one provider alone.
        None => return Ok(Vec::new()),
    };
    let [language] = matching.as_slice() else {
        return Err(Error::Invalid(
            "explicit primary-role provider is missing or ambiguous".into(),
        ));
    };
    request
        .files
        .iter()
        .filter(|file| {
            language.primary_role(ryo::languages::SourcePath(&file.0))
                == ryo::languages::PrimaryFileRole::Configuration
        })
        .map(|file| {
            #[cfg(unix)]
            let literal = file.0.replace('\\', "\\\\");
            #[cfg(not(unix))]
            let literal = file.0.to_string();
            linter::Glob::try_from(globset::escape(&literal).as_str())
                .map_err(|error| Error::Invalid(error.to_string()))
        })
        .collect()
}
#[path = "rule_context.rs"]
mod rule_context;
