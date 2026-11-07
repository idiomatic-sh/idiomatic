//! Pinned reference profile of the tool under measurement.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types;
use serde::Deserialize;
use std::collections::BTreeMap;
use crate::outbound::suite::adapters::local::declarations::schemas::{
    ObservationBindingSchema, RuleContextDeclarationSchema, observation_declarations,
    project_search_roots,
};

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Pinned reference profile and explicit default-rule accounting.
struct ToolSchema {
    #[serde(default, rename = "rule-observation-selection")]
    rule_observation_selection: Option<String>,
    #[serde(default, rename = "supporting-rules")]
    supporting_rules: Vec<String>,
    #[serde(default, rename = "rule-observations")]
    rule_observations: BTreeMap<String, Vec<ObservationBindingSchema>>,
    #[serde(default, rename = "rule-context")]
    rule_context: Option<RuleContextDeclarationSchema>,
    /// Generic whole-profile input observation requirement.
    #[serde(default, rename = "file-observation")]
    file_observation: Option<String>,
    #[serde(default, rename = "project-context-mode")]
    /// Explicit project lookup mode for this invocation.
    project_context_mode: ModeSchema,
    /// Reference tool name.
    tool: String,
    /// Exact expected version output.
    version: String,
    /// Trusted platform-specific SHA256 digests of the executable bytes.
    sha256: Vec<String>,
    /// Resolved default rule inventory for the pinned tool.
    defaults: Vec<String>,
    #[serde(default)]
    /// Capability groups whose rules remain blocked by documented limitations.
    deferred: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    /// Capability groups whose rules remain implementation work.
    pending: BTreeMap<String, Vec<String>>,
    /// Explicit logical query-module IDs mapped to profile-relative source paths.
    #[serde(default, rename = "query-modules")]
    query_modules: BTreeMap<String, String>,
    /// Explicit syntax applicability for each selected rule.
    #[serde(default, rename = "rule-applicability")]
    rule_applicability: BTreeMap<String, String>,
    #[serde(default, rename = "parser-configuration")]
    /// Language parser configuration shared by both contestants.
    parser_configuration: Option<String>,
    #[serde(default, rename = "source-language")]
    source_language: Option<String>,
    /// Explicit ordered import roots; absent and empty have distinct meanings.
    #[serde(default, rename = "project-search-roots")]
    project_search_roots: Option<Vec<String>>,
}
/// Decode and validate a pinned reference profile.
pub(in super::super) fn tool(text: &str) -> Result<types::Tool, Error> {
    toml::from_str::<ToolSchema>(text)?.try_into()
}
impl TryFrom<ToolSchema> for types::Tool {
    type Error = Error;
    fn try_from(wire: ToolSchema) -> Result<Self, Error> {
        if wire.sha256.is_empty()
            || wire
                .sha256
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != wire.sha256.len()
        {
            return Err(Error::Invalid(
                "executable pins must be nonempty and unique".into(),
            ));
        }
        let mut query_modules = BTreeMap::new();
        for (id, path) in wire.query_modules {
            let _module = ryo::query::QueryModuleSource::try_new(&id, crate::outbound::suite::adapters::local::constants::MODULE_SOURCE)
                .map_err(|error| Error::Invalid(error.to_string()))?;
            let path = std::path::PathBuf::from(path);
            ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(&path))?;
            if query_modules.insert(id, path).is_some() {
                return Err(Error::Invalid("duplicate query module ID".into()));
            }
        }
        let defaults = wire
            .defaults
            .iter()
            .map(|value| rule(value))
            .collect::<Result<Vec<_>, _>>()?;
        let supporting_rules = wire
            .supporting_rules
            .iter()
            .map(|id| rule(id))
            .collect::<Result<Vec<_>, _>>()?;
        if supporting_rules
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != supporting_rules.len()
            || supporting_rules.iter().any(|id| defaults.contains(id))
        {
            return Err(Error::Invalid(
                "supporting rules must be unique and distinct from defaults".into(),
            ));
        }
        for (id, applicability) in &wire.rule_applicability {
            let parsed_id = rule(id)?;
            if (!defaults.contains(&parsed_id) && !supporting_rules.contains(&parsed_id))
                || !matches!(
                    applicability.as_str(),
                    crate::outbound::suite::adapters::local::constants::ALWAYS
                        | crate::outbound::suite::adapters::local::constants::ACCEPTED_SYNTAX
                )
            {
                return Err(Error::Invalid(format!(
                    "invalid rule applicability policy: {id} -> {applicability}"
                )));
            }
        }
        if !wire.rule_applicability.is_empty() {
            for id in defaults.iter().chain(&supporting_rules) {
                if !wire.rule_applicability.contains_key(id.as_ref()) {
                    return Err(Error::Invalid(format!(
                        "invalid rule applicability policy: missing declared rule {id}"
                    )));
                }
            }
        }
        if wire
            .parser_configuration
            .as_ref()
            .is_some_and(|configuration| configuration.trim().is_empty())
        {
            return Err(Error::Invalid(
                "parser configuration must not be blank".into(),
            ));
        }
        if wire
            .source_language
            .as_ref()
            .is_some_and(|name| name.is_empty() || name.trim() != name)
        {
            return Err(Error::Invalid(
                "source language must be an exact nonblank provider name".into(),
            ));
        }
        Ok(Self {
            file_observation: match wire.file_observation.as_deref() {
                None => ryo_benchmark_contracts::domain::workloads::types::FileObservation::Undeclared,
                Some("posix-executable") => ryo_benchmark_contracts::domain::workloads::types::FileObservation::PosixExecutable,
                Some(_) => {
                    return Err(Error::Invalid(
                        "unsupported file observation requirement".into(),
                    ));
                }
            },
            project_context_mode: wire.project_context_mode.into(),
            tool: wire.tool.into(),
            version: wire.version.into(),
            sha256: wire
                .sha256
                .into_iter()
                .map(types::tool_digest::value::ToolDigest)
                .collect(),
            defaults,
            supporting_rules,
            rule_context: match wire.rule_context {
                Some(declaration) => ryo_benchmark_contracts::domain::workloads::types::RuleContextProfile::Declared(
                    declaration.try_into().map_err(Error::Invalid)?,
                ),
                None => ryo_benchmark_contracts::domain::workloads::types::RuleContextProfile::Undeclared,
            },
            rule_observation_selection: match wire.rule_observation_selection.as_deref() {
                None | Some("declared") => ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Declared,
                Some("selected") => ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Selected,
                Some(_) => {
                    return Err(Error::Invalid(
                        "unknown profile observation-selection policy".into(),
                    ));
                }
            },
            rule_observations: observation_declarations(wire.rule_observations)
                .map_err(Error::Invalid)?,
            deferred: dispositions(wire.deferred)?,
            pending: dispositions(wire.pending)?,
            query_modules: query_modules
                .into_iter()
                .map(|(name, path)| (ryo_benchmark_contracts::domain::requests::types::QueryModuleName(name), path))
                .collect(),
            rule_applicability: wire
                .rule_applicability
                .into_iter()
                .map(|(rule, policy)| (ryo_benchmark_contracts::domain::requests::types::RuleLabel(rule), ryo_benchmark_contracts::domain::requests::types::ApplicabilityPolicy(policy)))
                .collect(),
            parser_configuration: match wire.parser_configuration {
                Some(name) => ryo_benchmark_contracts::domain::requests::types::ParserSelection::Named(ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName(name)),
                None => ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
            },
            source_language: match wire.source_language {
                Some(name) => ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Selected(ryo_benchmark_contracts::domain::requests::types::SourceLanguageName(name)),
                None => ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
            },
            project_search_roots: match project_search_roots(wire.project_search_roots)? {
                Some(roots) => ryo_benchmark_contracts::domain::requests::types::SearchRoots::Declared(roots),
                None => ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
            },
        })
    }
}
/// Expand grouped capabilities while rejecting duplicate rule membership.
fn dispositions(
    groups: BTreeMap<String, Vec<String>>,
) -> Result<
    BTreeMap<
        ryo::linter::RuleId,
        ryo_benchmark_contracts::domain::workloads::types::capability_name::value::CapabilityName,
    >,
    Error,
> {
    let mut rules = BTreeMap::new();
    for (capability, members) in groups {
        if capability.is_empty()
            || !capability
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
            || members.is_empty()
        {
            return Err(Error::Invalid(
                "capability groups require a stable name and rules".into(),
            ));
        }
        for value in members {
            if rules
                .insert(rule(&value)?, ryo_benchmark_contracts::domain::workloads::types::capability_name::value::CapabilityName(capability.clone()))
                .is_some()
            {
                return Err(Error::Invalid(
                    "rule appears in multiple capability groups".into(),
                ));
            }
        }
    }
    Ok(rules)
}
/// Validate a rule identifier before it enters the benchmark domain.
fn rule(value: &str) -> Result<ryo::linter::RuleId, Error> {
    ryo::linter::RuleId::try_from(value).map_err(|error| Error::Invalid(error.to_string()))
}

/// Rewrite a profile document so it selects exactly the authored rules and keeps the definitions the others need.
pub(in super::super) fn narrowed(
    original: &str,
    authored: &[ryo::linter::RuleId],
    definitions: &std::collections::BTreeSet<ryo::linter::RuleId>,
) -> Result<String, Error> {
    use crate::outbound::suite::adapters::local::constants;
    let mut value: toml::Value = toml::from_str(original)?;
    let table = value
        .as_table_mut()
        .ok_or_else(|| Error::Invalid(constants::PROFILE_MUST_BE_A_TABLE.into()))?;
    table.insert(
        constants::DEFAULTS.into(),
        toml::Value::Array(
            authored
                .iter()
                .map(|id| toml::Value::String(id.to_string()))
                .collect(),
        ),
    );
    table.insert(
        constants::SUPPORTING_RULES.into(),
        toml::Value::Array(
            definitions
                .iter()
                .filter(|id| !authored.contains(id))
                .map(|id| toml::Value::String(id.to_string()))
                .collect(),
        ),
    );
    table.remove(constants::PENDING);
    table.remove(constants::DEFERRED);
    for key in [constants::RULE_OBSERVATIONS, constants::RULE_APPLICABILITY] {
        if let Some(entries) = table.get_mut(key).and_then(toml::Value::as_table_mut) {
            entries.retain(|id, _| definitions.iter().any(|rule| rule.as_ref() == id));
        }
    }
    toml::to_string(&value).map_err(|error| Error::Invalid(error.to_string()))
}

/// Explicit project lookup mode a profile declares.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ModeSchema {
    /// Source-only execution with no external project lookup.
    #[default]
    Isolated,
    /// Disk lookup constrained by the sealed context manifest.
    ListedDisk,
}
impl From<ModeSchema>
    for ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode
{
    fn from(mode: ModeSchema) -> Self {
        match mode {
            ModeSchema::Isolated => Self::Isolated,
            ModeSchema::ListedDisk => Self::ListedDisk,
        }
    }
}
