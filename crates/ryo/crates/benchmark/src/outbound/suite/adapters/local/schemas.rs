//! Private TOML DTOs map to canonical benchmark values at this boundary.
use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Pinned reference profile and explicit default-rule accounting.
struct ToolSchema {
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
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Shared language corpus manifest.
struct SuiteSchema {
    /// Language identity for the corpus.
    language: String,
    /// Pinned repositories included in the workload.
    projects: Vec<ProjectSchema>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// One reproducible repository workload.
struct ProjectSchema {
    /// Stable project identity.
    name: String,
    /// Repository URL used to obtain the source.
    repository: String,
    /// Exact source revision to check out.
    revision: String,
    /// Manifest-relative edit patch for incremental trials.
    patch: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Independent rule examples and expected findings.
struct CasesSchema {
    /// Ordered set of executable examples.
    cases: Vec<CaseSchema>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// One independent source example.
struct CaseSchema {
    #[serde(default)]
    performance: crate::outbound::worker::schemas::OptionsSchema,
    #[serde(default, rename = "selected-rules")]
    selected_rules: Option<Vec<String>>,
    #[serde(default, rename = "rule-observations")]
    rule_observations: Option<BTreeMap<String, Vec<ObservationBindingSchema>>>,
    #[serde(default, rename = "rule-context")]
    rule_context: Option<RuleConfigurationSchema>,
    /// Optional explicit primary inventory; absent paths require the shared context absence seal.
    #[serde(default, rename = "requested-files")]
    requested_files: Option<Vec<String>>,
    /// Explicit POSIX executable observations for declared source files.
    #[serde(default, rename = "file-metadata")]
    file_metadata: Option<BTreeMap<String, bool>>,
    #[serde(default)]
    /// Optional explicit per-case project context; isolated cases omit it.
    context: Option<CaseContextSchema>,
    /// Explicit diagnostic scope; existing cases assert their enclosing rule.
    #[serde(default)]
    comparison: ComparisonSchema,
    /// Human-readable case identity.
    name: String,
    /// Project-relative source paths mapped to complete source text.
    files: BTreeMap<String, String>,
    /// Expected diagnostic ranges, preserving multiplicity.
    expected: Vec<ExpectedSchema>,
    /// Optional separately pinned reference behavior and independent evidence.
    #[serde(default, rename = "reference-discrepancy")]
    reference_discrepancy:
        Option<crate::outbound::suite::types::discrepancy::ReferenceDiscrepancyWire>,
    /// A documented difference from an already public tool selection contract.
    #[serde(default, rename = "tool-contract-difference")]
    tool_contract_difference:
        Option<crate::outbound::suite::types::discrepancy::ReferenceDiscrepancyWire>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Expected finding in one authored case.
struct ExpectedSchema {
    /// Explicit diagnostic identity, required in full-report mode.
    #[serde(default)]
    rule: Option<String>,
    /// Project-relative source path.
    file: String,
    /// Inclusive zero-based UTF-8 byte offset.
    start: usize,
    /// Exclusive zero-based UTF-8 byte offset.
    end: usize,
}

/// Serialized fixture comparison scope.
#[derive(Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ComparisonSchema {
    /// Compare the enclosing rule only.
    #[default]
    Rule,
    /// Compare all reported diagnostics.
    Full,
}

/// Decode and validate a pinned reference profile.
pub(super) fn tool(text: &str) -> Result<types::Tool, Error> {
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
            crate::domain::suite::service::relative_path(&path)?;
            if query_modules.insert(id, path).is_some() {
                return Err(Error::Invalid("duplicate query module ID".into()));
            }
        }
        let defaults = wire
            .defaults
            .iter()
            .map(|value| rule(value))
            .collect::<Result<Vec<_>, _>>()?;
        for (id, applicability) in &wire.rule_applicability {
            let parsed_id = rule(id)?;
            if !defaults.contains(&parsed_id)
                || !matches!(applicability.as_str(), "always" | "accepted-syntax")
            {
                return Err(Error::Invalid(format!(
                    "invalid rule applicability policy: {id} -> {applicability}"
                )));
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
            rule_observations: observation_declarations(wire.rule_observations)
                .map_err(Error::Invalid)?,
            deferred: dispositions(wire.deferred)?,
            pending: dispositions(wire.pending)?,
            query_modules,
            rule_applicability: wire.rule_applicability,
            parser_configuration: wire.parser_configuration,
            source_language: wire.source_language,
        })
    }
}
/// Expand grouped capabilities while rejecting duplicate rule membership.
fn dispositions(
    groups: BTreeMap<String, Vec<String>>,
) -> Result<BTreeMap<ryo::linter::RuleId, types::CapabilityName>, Error> {
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
                .insert(rule(&value)?, types::CapabilityName(capability.clone()))
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
pub(super) fn rule(value: &str) -> Result<ryo::linter::RuleId, Error> {
    ryo::linter::RuleId::try_from(value).map_err(|error| Error::Invalid(error.to_string()))
}
/// Decode the shared language corpus manifest.
pub(super) fn suite(text: &str) -> Result<types::Suite, Error> {
    let wire: SuiteSchema = toml::from_str(text)?;
    if wire.language.trim().is_empty() || wire.projects.is_empty() {
        return Err(Error::Invalid(
            "corpus must name a language and at least one project".into(),
        ));
    }
    {
        let mut names = std::collections::BTreeSet::new();
        for project in &wire.projects {
            if project.name.trim().is_empty() || !names.insert(project.name.as_str()) {
                return Err(Error::Invalid(
                    "corpus project names must be nonempty and unique".into(),
                ));
            }
        }
    }
    Ok(wire.into())
}
impl From<SuiteSchema> for types::Suite {
    fn from(wire: SuiteSchema) -> Self {
        Self {
            language: wire.language.into(),
            projects: wire.projects.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<ProjectSchema> for types::Project {
    fn from(project: ProjectSchema) -> Self {
        Self {
            name: project.name.into(),
            repository: project.repository.into(),
            revision: project.revision.into(),
            patch: project.patch.into(),
        }
    }
}
/// Decode independent sources and expected byte ranges.
pub(super) fn cases(text: &str) -> Result<types::Cases, Error> {
    toml::from_str::<CasesSchema>(text)?.try_into()
}
impl TryFrom<CasesSchema> for types::Cases {
    type Error = Error;
    fn try_from(wire: CasesSchema) -> Result<Self, Error> {
        Ok(Self {
            cases: wire
                .cases
                .into_iter()
                .map(|case| {
                    if case.reference_discrepancy.is_some()
                        && case.tool_contract_difference.is_some()
                    {
                        return Err(Error::Invalid(
                            "a case cannot declare both discrepancy authorities".into(),
                        ));
                    }
                    Ok(types::Case {
                        performance: case.performance.into(),
                        selected_rules: case
                            .selected_rules
                            .map(|rules| rules.into_iter().map(|id| rule(&id)).collect())
                            .transpose()?,
                        rule_context: case
                            .rule_context
                            .map(TryInto::try_into)
                            .transpose()
                            .map_err(Error::Invalid)?,
                        rule_observations: case
                            .rule_observations
                            .map(observation_declarations)
                            .transpose()
                            .map_err(Error::Invalid)?,
                        requested_files: case.requested_files.map(|paths| {
                            paths
                                .into_iter()
                                .map(|path| ryo::linter::RelativePath(path.into_boxed_str()))
                                .collect()
                        }),
                        file_metadata: case.file_metadata.map(|values| {
                            values
                                .into_iter()
                                .map(|(path, executable)| {
                                    (ryo::linter::RelativePath(path.into_boxed_str()), executable)
                                })
                                .collect()
                        }),
                        reference_discrepancy: None,
                        context: case.context.map(Into::into),
                        comparison: match case.comparison {
                            ComparisonSchema::Rule => types::CaseComparison::Rule,
                            ComparisonSchema::Full => types::CaseComparison::Full,
                        },
                        name: case.name.into(),
                        files: case
                            .files
                            .into_iter()
                            .map(|(path, text)| {
                                (
                                    ryo::linter::RelativePath(path.into_boxed_str()),
                                    ryo::linter::SourceText(text.into_boxed_str()),
                                )
                            })
                            .collect(),
                        expected: case
                            .expected
                            .into_iter()
                            .map(|expected| {
                                Ok(types::Expected {
                                    identity: match expected.rule {
                                        Some(value) => types::ExpectedRule::Explicit(rule(&value)?),
                                        None => types::ExpectedRule::Parent,
                                    },
                                    file: ryo::linter::RelativePath(expected.file.into_boxed_str()),
                                    start: crate::domain::diagnostics::types::SourceByteOffset(
                                        expected.start,
                                    ),
                                    end: crate::domain::diagnostics::types::SourceByteOffset(
                                        expected.end,
                                    ),
                                })
                            })
                            .collect::<Result<_, Error>>()?,
                    })
                })
                .collect::<Result<_, Error>>()?,
        })
    }
}
#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Authored project context materialized independently for one case.
struct CaseContextSchema {
    #[serde(rename = "logical-root")]
    /// Stable run-root path inside the case boundary.
    logical_root: std::path::PathBuf,
    #[serde(default, rename = "auxiliary-files")]
    /// Explicit auxiliary file paths and contents.
    auxiliary_files: BTreeMap<std::path::PathBuf, ContextBytes>,
    #[serde(default)]
    /// Declared paths that must remain absent.
    missing: Vec<std::path::PathBuf>,
}
#[derive(Deserialize)]
#[serde(untagged)]
/// Text or exact byte content for an auxiliary context file.
enum ContextBytes {
    /// UTF-8 source content.
    Text(String),
    /// Exact bytes, including malformed UTF-8 controls.
    Bytes(Vec<u8>),
}
impl From<CaseContextSchema> for types::CaseContext {
    fn from(value: CaseContextSchema) -> Self {
        Self {
            logical_root: value.logical_root,
            auxiliary_files: value
                .auxiliary_files
                .into_iter()
                .map(|(path, bytes)| {
                    (
                        path,
                        match bytes {
                            ContextBytes::Text(text) => text.into_bytes(),
                            ContextBytes::Bytes(bytes) => bytes,
                        },
                    )
                })
                .collect(),
            missing: value.missing,
        }
    }
}

/// Read raw reference wire values without constructing domain expectations.
pub(super) fn case_reference_wires(
    text: &str,
) -> Result<
    Vec<(
        String,
        crate::outbound::suite::types::discrepancy::ReferenceDiscrepancyWire,
        crate::outbound::suite::types::discrepancy::DiscrepancyBasisWire,
    )>,
    toml::de::Error,
> {
    use serde::de::Error as _;
    let mut wires = Vec::new();
    for case in toml::from_str::<CasesSchema>(text)?.cases {
        match (case.reference_discrepancy, case.tool_contract_difference) {
            (Some(wire), None) => wires.push((
                case.name,
                wire,
                crate::outbound::suite::types::discrepancy::DiscrepancyBasisWire::LanguageDefect,
            )),
            (None, Some(wire)) => wires.push((
                case.name,
                wire,
                crate::outbound::suite::types::discrepancy::DiscrepancyBasisWire::ToolContract,
            )),
            (None, None) => {}
            (Some(_), Some(_)) => {
                return Err(toml::de::Error::custom(
                    "one discrepancy authority per case",
                ));
            }
        }
    }
    Ok(wires)
}

/// Explicit authored input at a private process/TOML serialization boundary.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationBindingSchema {
    name: String,
    columns: Vec<ObservationColumnSchema>,
    producers: Vec<ObservationProducerSchema>,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationColumnSchema {
    name: String,
    kind: ObservationKindSchema,
}
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ObservationKindSchema {
    Node,
    Symbol,
    File,
    Span,
    Number,
    Flag,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum ObservationProducerSchema {
    Output { rule: String, output: String },
    NoQuery { rule: String },
}

/// Convert checked identities while leaving actual producer/schema validation to request compilation.
fn observation_declarations(
    declarations: std::collections::BTreeMap<String, Vec<ObservationBindingSchema>>,
) -> Result<crate::domain::suite::types::RuleObservationDeclarations, String> {
    use ryo::linter::{
        ObservationBinding, ObservationBindingName, ObservationColumn, ObservationColumnName,
        ObservationKind, ObservationName, ObservationProducer, RuleId,
    };
    declarations
        .into_iter()
        .map(|(consumer, bindings)| {
            let consumer =
                RuleId::try_from(consumer.as_str()).map_err(|error| error.to_string())?;
            let bindings = bindings
                .into_iter()
                .map(|binding| {
                    let producers = binding
                        .producers
                        .into_iter()
                        .map(|producer| {
                            Ok(match producer {
                                ObservationProducerSchema::Output { rule, output } => {
                                    ObservationProducer::Output {
                                        rule: RuleId::try_from(rule.as_str())
                                            .map_err(|error| error.to_string())?,
                                        output: ObservationName(output.into()),
                                    }
                                }
                                ObservationProducerSchema::NoQuery { rule } => {
                                    ObservationProducer::NoQuery {
                                        rule: RuleId::try_from(rule.as_str())
                                            .map_err(|error| error.to_string())?,
                                    }
                                }
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    Ok(ObservationBinding {
                        name: ObservationBindingName(binding.name.into()),
                        columns: binding
                            .columns
                            .into_iter()
                            .map(|column| ObservationColumn {
                                name: ObservationColumnName(column.name.into()),
                                kind: match column.kind {
                                    ObservationKindSchema::Node => ObservationKind::Node,
                                    ObservationKindSchema::Symbol => ObservationKind::Symbol,
                                    ObservationKindSchema::File => ObservationKind::File,
                                    ObservationKindSchema::Span => ObservationKind::Span,
                                    ObservationKindSchema::Number => ObservationKind::Number,
                                    ObservationKindSchema::Flag => ObservationKind::Flag,
                                },
                            })
                            .collect(),
                        producers,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok((consumer, bindings))
        })
        .collect()
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleContextTablesSchema {
    selected: String,
    disabled: String,
    external: String,
}
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleConfigurationSchema {
    #[serde(rename = "external-prefixes")]
    external_prefixes: Vec<String>,
    #[serde(rename = "disabled-rules")]
    disabled_rules: std::collections::BTreeMap<String, Vec<String>>,
}
impl From<RuleContextTablesSchema> for crate::domain::suite::types::RuleContextTables {
    fn from(value: RuleContextTablesSchema) -> Self {
        Self {
            selected: value.selected,
            disabled: value.disabled,
            external: value.external,
        }
    }
}
impl TryFrom<RuleConfigurationSchema> for crate::domain::suite::types::RuleConfiguration {
    type Error = String;
    fn try_from(value: RuleConfigurationSchema) -> Result<Self, Self::Error> {
        Ok(Self {
            external_prefixes: value.external_prefixes,
            disabled_rules: value
                .disabled_rules
                .into_iter()
                .map(|(path, rules)| {
                    Ok((
                        ryo::linter::RelativePath(path.into()),
                        rules
                            .into_iter()
                            .map(|rule| {
                                ryo::linter::RuleId::try_from(rule.as_str())
                                    .map_err(|error| error.to_string())
                            })
                            .collect::<Result<Vec<_>, String>>()?,
                    ))
                })
                .collect::<Result<_, String>>()?,
        })
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleContextDeclarationSchema {
    tables: RuleContextTablesSchema,
    defaults: RuleConfigurationSchema,
}
impl TryFrom<RuleContextDeclarationSchema> for crate::domain::suite::types::RuleContextDeclaration {
    type Error = String;
    fn try_from(value: RuleContextDeclarationSchema) -> Result<Self, Self::Error> {
        Ok(Self {
            tables: value.tables.into(),
            defaults: value.defaults.try_into()?,
        })
    }
}
