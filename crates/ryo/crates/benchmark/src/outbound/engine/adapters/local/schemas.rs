//! The candidate rule directory and the project sources are documents this adapter reads: their checked loading into the SDK's inputs.
use super::constants;
use crate::outbound::engine::types::FrontendDiagnosticProjection;
use ryo::linter::{self, CaseSource, Rule, RuleApplicability, RuleId, SourceText};
use ryo::query::RyoQlQuery;
use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    ApplicabilityPolicy, QueryModuleName, RuleLabel, WorkerRequest,
};
use std::collections::BTreeMap;
use std::path::Path;

/// Load all selected query files, refusing duplicate or empty inventories.
pub(super) fn load_rules(
    directory: &std::path::Path,
    registry: &BTreeMap<QueryModuleName, std::path::PathBuf>,
    applicability_by_rule: &BTreeMap<RuleLabel, ApplicabilityPolicy>,
    frontend_diagnostic: Option<FrontendDiagnosticProjection>,
    declarations: Option<
        &ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations,
    >,
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
        let applicability = match applicability_by_rule
                .get(&RuleLabel(id.clone()))
                .map(|policy| policy.0.as_str()) {
            Some(constants::ALWAYS) => {
                RuleApplicability::Always
            }
            Some(constants::ACCEPTED_SYNTAX) => {
                RuleApplicability::AcceptedSyntax
            }
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
        let _new_mapping = mapped_rules.insert(RuleLabel(id.clone()));
        let id =
            RuleId::try_from(id.as_str()).map_err(|error| Error::Invalid(error.to_string()))?;
        let query = if modules.content() == ryo::query::CatalogContent::Empty {
            RyoQlQuery::try_from(query.as_str())
                .map_err(|error| Error::Invalid(error.to_string()))?
        } else {
            RyoQlQuery::with_modules(&format!("rules/{id}"), &query, &modules)
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
            constants::APPLICABILITY_MAP_DOES_NOT_EXACTLY_MATCH.into(),
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
pub(super) fn validate_skips(
    skipped: &[linter::Skipped],
    selected_rules: &std::collections::BTreeSet<RuleId>,
    applicability_by_rule: &BTreeMap<RuleLabel, ApplicabilityPolicy>,
    files: &std::collections::BTreeSet<linter::RelativePath>,
    frontend_diagnostic_enabled: bool,
) -> Result<(), Error> {
    let mut seen = std::collections::BTreeSet::new();
    for skip in skipped {
        if skip.reason != linter::SkipReason::SyntaxRejected
            || !files.contains(&skip.path)
            || !selected_rules.contains(&skip.rule)
            || applicability_by_rule
                .get(&RuleLabel(skip.rule.as_ref().to_owned()))
                .map(|policy| policy.0.as_str())
                != Some(constants::ACCEPTED_SYNTAX)
            || !seen.insert((skip.rule.clone(), skip.path.clone()))
        {
            return Err(Error::Invalid(format!(
                "invalid or unexplained applicability skip: {skip:?}"
            )));
        }
    }
    if !skipped.is_empty() && !frontend_diagnostic_enabled {
        return Err(Error::Invalid(
            constants::APPLICABILITY_SKIPS_REQUIRE_FRONTEND_DIAGNOS.into(),
        ));
    }
    Ok(())
}

/// Load and validate every declared module inside the same sealed profile boundary.
fn query_modules(
    directory: &std::path::Path,
    registry: &BTreeMap<String, std::path::PathBuf>,
) -> Result<ryo::query::QueryModuleCatalog, Error> {
    let profile = directory
        .parent()
        .ok_or_else(|| Error::Invalid(constants::RULES_DIRECTORY_HAS_NO_PROFILE_ROOT.into()))?;
    let root = profile.canonicalize()?;
    let mut modules = Vec::with_capacity(registry.len());
    for (id, relative) in registry {
        let _module = ryo::query::QueryModuleSource::try_new(
            &id.0,
            constants::MODULE_SOURCE,
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        RelativeArtifactPath::try_from(relative.as_path())?;
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
            ryo::query::QueryModuleSource::try_new(&id.0, &source)
                .map_err(|error| Error::Invalid(error.to_string()))?,
        );
    }
    ryo::query::QueryModuleCatalog::try_new(modules)
        .map_err(|error| Error::Invalid(error.to_string()))
}


/// Read each explicitly requested source once, retaining its exact ordered inventory.
pub(super) fn source_inputs(
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
        RelativeArtifactPath::try_from(Path::new(file.0.as_ref()))?;
        if !accepted.insert(file.clone()) {
            return Err(Error::Invalid(
                constants::DUPLICATE_SOURCE.into(),
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


/// Observe the actual staged file before transporting an Isolated input into memory.
fn executable(path: &Path) -> Result<ryo::linter::ExecutablePermission, Error> {
    let metadata = std::fs::metadata(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        Ok(if metadata.permissions().mode() & 0o111 != 0 {
            ryo::linter::ExecutablePermission::PosixExecutable
        } else {
            ryo::linter::ExecutablePermission::PosixNotExecutable
        })
    }
    #[cfg(not(unix))]
    {
        let _metadata = metadata;
        Ok(ryo::linter::ExecutablePermission::Unsupported)
    }
}


#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
