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
    project_context_mode: crate::outbound::context::adapters::local::schemas::ModeSchema,
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
            let _module = ryo::query::QueryModuleSource::try_new(&id, "module source")
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
                || !matches!(applicability.as_str(), "always" | "accepted-syntax")
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
                None => None,
                Some("posix-executable") => Some(types::FileObservation::PosixExecutable),
                Some(_) => {
                    return Err(Error::Invalid(
                        "unsupported file observation requirement".into(),
                    ));
                }
            },
            project_context_mode: wire.project_context_mode.into(),
            tool: wire.tool.into(),
            version: wire.version.into(),
            sha256: wire.sha256.into_iter().map(Into::into).collect(),
            defaults,
            supporting_rules,
            rule_context: wire
                .rule_context
                .map(TryInto::try_into)
                .transpose()
                .map_err(Error::Invalid)?,
            rule_observation_selection: match wire.rule_observation_selection.as_deref() {
                None | Some("declared") => types::ObservationSelectionPolicy::Declared,
                Some("selected") => types::ObservationSelectionPolicy::Selected,
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
            query_modules,
            rule_applicability: wire.rule_applicability,
            parser_configuration: wire.parser_configuration,
            source_language: wire.source_language,
            project_search_roots: project_search_roots(wire.project_search_roots)?,
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
