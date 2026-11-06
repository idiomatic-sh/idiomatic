//! Private TOML DTOs map to canonical benchmark values at this boundary.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types;
use serde::Deserialize;
use std::collections::BTreeMap;

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
    /// Per-case ordered import roots override the profile declaration.
    #[serde(default, rename = "project-search-roots")]
    project_search_roots: Option<Vec<String>>,
    /// Contextual entry kinds for the candidate's Given corpus.
    #[serde(default, rename = "given-project-inventory")]
    given_project_inventory:
        Option<crate::outbound::context::adapters::local::schemas::ProjectInventorySchema>,
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

fn project_search_roots(
    roots: Option<Vec<String>>,
) -> Result<Option<Vec<ryo::linter::RelativePath>>, Error> {
    roots
        .map(|roots| {
            roots
                .into_iter()
                .map(|root| {
                    if !root.is_empty() {
                        ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(&root))?;
                    }
                    Ok(ryo::linter::RelativePath(root.into_boxed_str()))
                })
                .collect::<Result<Vec<_>, Error>>()
        })
        .transpose()
}

fn given_project_inventory(
    inventory: crate::outbound::context::adapters::local::schemas::ProjectInventorySchema,
) -> Result<ryo::linter::GivenProjectInventory, Error> {
    let inventory: ryo::linter::GivenProjectInventory = inventory.into();
    for entry in &inventory.entries {
        ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(entry.path.0.as_ref()))?;
    }
    Ok(inventory)
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
                        project_search_roots: project_search_roots(case.project_search_roots)?,
                        given_project_inventory: case
                            .given_project_inventory
                            .map(given_project_inventory)
                            .transpose()?,
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
                                    start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(
                                        expected.start,
                                    ),
                                    end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(
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
impl From<CaseContextSchema> for ryo_benchmark_contracts::domain::context::types::CaseContext {
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
) -> Result<
    ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations,
    String,
>{
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
impl From<RuleContextTablesSchema>
    for ryo_benchmark_contracts::domain::context::types::RuleContextTables
{
    fn from(value: RuleContextTablesSchema) -> Self {
        Self {
            selected: value.selected,
            disabled: value.disabled,
            external: value.external,
        }
    }
}
impl TryFrom<RuleConfigurationSchema>
    for ryo_benchmark_contracts::domain::context::types::RuleConfiguration
{
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
impl TryFrom<RuleContextDeclarationSchema>
    for ryo_benchmark_contracts::domain::context::types::RuleContextDeclaration
{
    type Error = String;
    fn try_from(value: RuleContextDeclarationSchema) -> Result<Self, Self::Error> {
        Ok(Self {
            tables: value.tables.into(),
            defaults: value.defaults.try_into()?,
        })
    }
}

/// The native receipt row: read back by the inventory and written by the case observer.
/// Unknown fields are refused; the documents are a contract.
pub(super) mod observation {
    use crate::outbound::suite::types::{
        finding::{ByteOffset, Finding, SkippedPair},
        observation_document::{
            ContextManifest, ContextMode, ExitStatus, ObservationDocument, ProcessArtifacts,
            ProcessObservation,
        },
        reference_discrepancy::{ArtifactPointer, DiscrepancyValidation, ExpectationKind},
        text::{
            CanonicalJson, CaseName, PathText, ReasonText, RuleName, SchemaName, Sha256Text, Side,
            YesNo,
        },
    };
    use ryo_benchmark_contracts::domain::reports::types::{
        ComparisonContract, ExecutionMetadata, RuleCoverage,
    };
    use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
    use serde::{Deserialize, Serialize};

    fn sha(value: Option<String>) -> Option<Sha256Text> {
        value.map(Sha256Text)
    }
    fn text(value: &Sha256Text) -> String {
        value.0.clone()
    }
    fn opt_text(value: &Option<Sha256Text>) -> Option<String> {
        value.as_ref().map(text)
    }
    fn canonical(value: Option<serde_json::Value>) -> Option<CanonicalJson> {
        value.map(|value| CanonicalJson(value.to_string()))
    }
    fn tree(value: &Option<CanonicalJson>) -> Option<serde_json::Value> {
        value
            .as_ref()
            .map(|json| serde_json::from_str(&json.0).unwrap_or(serde_json::Value::Null))
    }
    fn flag(value: bool) -> YesNo {
        YesNo::from(value)
    }
    fn is(value: YesNo) -> bool {
        value == YesNo::Yes
    }

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct FindingSchema {
        rule: String,
        file: String,
        start: u64,
        end: u64,
    }
    impl From<FindingSchema> for Finding {
        fn from(wire: FindingSchema) -> Self {
            Self {
                rule: RuleName(wire.rule),
                file: PathText(wire.file),
                start: ByteOffset(wire.start),
                end: ByteOffset(wire.end),
            }
        }
    }
    impl From<&Finding> for FindingSchema {
        fn from(value: &Finding) -> Self {
            Self {
                rule: value.rule.0.clone(),
                file: value.file.0.clone(),
                start: value.start.0,
                end: value.end.0,
            }
        }
    }
    fn findings(rows: Vec<FindingSchema>) -> Vec<Finding> {
        rows.into_iter().map(Finding::from).collect()
    }
    fn wire_findings(rows: &[Finding]) -> Vec<FindingSchema> {
        rows.iter().map(FindingSchema::from).collect()
    }

    #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "kebab-case")]
    enum SideSchema {
        Reference,
        Candidate,
    }
    #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "kebab-case")]
    enum SkipReasonSchema {
        SyntaxRejected,
    }
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct SkippedSchema {
        rule: String,
        path: String,
        reason: SkipReasonSchema,
    }
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "availability", rename_all = "kebab-case", deny_unknown_fields)]
    enum ExecutionSchema {
        Unknown {},
        Unavailable { reason: String },
        Complete {},
    }
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct CoverageSchema {
        selected_rule_file_pairs: usize,
        evaluated_rule_file_pairs: usize,
        skipped_rule_file_pairs: usize,
        unknown_or_unread_rule_file_pairs: usize,
    }
    #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "kebab-case")]
    enum ModeSchema {
        Isolated,
        ListedDisk,
    }
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct ContextManifestSchema {
        mode: ModeSchema,
        logical_root: String,
        actual_root: String,
        manifest_hash: String,
    }
    #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "kebab-case")]
    enum ContractSchema {
        Diagnostics,
        CompleteReport,
    }
    #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "kebab-case")]
    enum KindSchema {
        Diagnostics,
        ProcessFailure,
    }
    #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "kebab-case")]
    enum BasisSchema {
        LanguageDefect,
        ToolContractDifference,
    }
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct PointerSchema {
        path: String,
        sha256: String,
    }
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct ProcessArtifactsSchema {
        stdout: PointerSchema,
        stderr: PointerSchema,
        invocation: PointerSchema,
    }
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct ProcessObservationSchema {
        kind: KindSchema,
        exit: i64,
        signal: Option<i64>,
        actual_request_root: String,
        observer_no_deadline: bool,
        executable_sha256: Option<String>,
        settings_sha256: Option<String>,
        metadata_error: Option<String>,
        template_sha256: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        artifacts: Option<ProcessArtifactsSchema>,
    }
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct ValidationSchema {
        discrepancy_basis: BasisSchema,
        process_observation_group_sha256: Option<String>,
        reference_expectation_kind: KindSchema,
        settings_sha256: String,
        actual_executable_sha256: String,
        profile_sha256: String,
        source_sha256: String,
        reference_output_sha256: String,
        intended_output_sha256: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_metadata_sha256: Option<String>,
    }

    /// One native side's receipt row.
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct ObservationSchema {
        rule: String,
        name: String,
        side: SideSchema,
        complete: bool,
        error: Option<String>,
        case_passed: bool,
        files: Vec<String>,
        source_sha256: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_metadata_sha256: Option<String>,
        project_inputs_sha256: Option<String>,
        expected: Vec<FindingSchema>,
        reference_expected: Option<Vec<FindingSchema>>,
        diagnostics: Option<Vec<FindingSchema>>,
        skipped: Option<Vec<SkippedSchema>>,
        observation_schema: String,
        expectation_contract: ContractSchema,
        execution_metadata: Option<ExecutionSchema>,
        coverage: Option<CoverageSchema>,
        context_manifest: Option<ContextManifestSchema>,
        project_context: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        rule_context: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        rule_observations: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        selected_rules: Option<Vec<String>>,
        actual_request_root: Option<String>,
        documented_reference_discrepancy: bool,
        documented_tool_contract_difference: Option<bool>,
        discrepancy_validation: Option<ValidationSchema>,
        raw_parity_matches_intended: Option<bool>,
        diagnostics_match_intended: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        process_observation: Option<ProcessObservationSchema>,
    }

    fn contract(wire: ContractSchema) -> ComparisonContract {
        match wire {
            ContractSchema::Diagnostics => ComparisonContract::Diagnostics,
            ContractSchema::CompleteReport => ComparisonContract::CompleteReport,
        }
    }
    fn wire_contract(value: ComparisonContract) -> ContractSchema {
        match value {
            ComparisonContract::Diagnostics => ContractSchema::Diagnostics,
            ComparisonContract::CompleteReport => ContractSchema::CompleteReport,
        }
    }
    fn kind(wire: KindSchema) -> ExpectationKind {
        match wire {
            KindSchema::Diagnostics => ExpectationKind::Diagnostics,
            KindSchema::ProcessFailure => ExpectationKind::ProcessFailure,
        }
    }
    fn wire_kind(value: ExpectationKind) -> KindSchema {
        match value {
            ExpectationKind::Diagnostics => KindSchema::Diagnostics,
            ExpectationKind::ProcessFailure => KindSchema::ProcessFailure,
        }
    }
    fn basis(wire: BasisSchema) -> DiscrepancyBasis {
        match wire {
            BasisSchema::LanguageDefect => DiscrepancyBasis::LanguageDefect,
            BasisSchema::ToolContractDifference => DiscrepancyBasis::ToolContract,
        }
    }
    fn wire_basis(value: DiscrepancyBasis) -> BasisSchema {
        match value {
            DiscrepancyBasis::LanguageDefect => BasisSchema::LanguageDefect,
            DiscrepancyBasis::ToolContract => BasisSchema::ToolContractDifference,
        }
    }
    fn pointer(wire: PointerSchema) -> ArtifactPointer {
        ArtifactPointer {
            path: PathText(wire.path),
            sha256: Sha256Text(wire.sha256),
        }
    }
    fn wire_pointer(value: &ArtifactPointer) -> PointerSchema {
        PointerSchema {
            path: value.path.0.clone(),
            sha256: value.sha256.0.clone(),
        }
    }

    impl From<ObservationSchema> for ObservationDocument {
        fn from(wire: ObservationSchema) -> Self {
            Self {
                rule: RuleName(wire.rule),
                name: CaseName(wire.name),
                side: match wire.side {
                    SideSchema::Reference => Side::Reference,
                    SideSchema::Candidate => Side::Candidate,
                },
                complete: flag(wire.complete),
                error: wire.error.map(ReasonText),
                case_passed: flag(wire.case_passed),
                files: wire.files.into_iter().map(PathText).collect(),
                source_sha256: sha(wire.source_sha256),
                file_metadata_sha256: sha(wire.file_metadata_sha256),
                project_inputs_sha256: sha(wire.project_inputs_sha256),
                expected: findings(wire.expected),
                reference_expected: wire.reference_expected.map(findings),
                diagnostics: wire.diagnostics.map(findings),
                skipped: wire.skipped.map(|rows| {
                    rows.into_iter()
                        .map(|skip| {
                            let SkipReasonSchema::SyntaxRejected = skip.reason;
                            SkippedPair {
                                rule: RuleName(skip.rule),
                                path: PathText(skip.path),
                            }
                        })
                        .collect()
                }),
                observation_schema: SchemaName(wire.observation_schema),
                expectation_contract: contract(wire.expectation_contract),
                execution_metadata: wire.execution_metadata.map(|value| match value {
                    ExecutionSchema::Unknown {} => ExecutionMetadata::Unknown,
                    ExecutionSchema::Unavailable { reason } => {
                        ExecutionMetadata::Unavailable { reason }
                    }
                    ExecutionSchema::Complete {} => ExecutionMetadata::Complete,
                }),
                coverage: wire.coverage.map(|coverage| RuleCoverage {
                    selected: coverage.selected_rule_file_pairs,
                    evaluated: coverage.evaluated_rule_file_pairs,
                    skipped: coverage.skipped_rule_file_pairs,
                    unknown_or_unread: coverage.unknown_or_unread_rule_file_pairs,
                }),
                context_manifest: wire.context_manifest.map(|manifest| ContextManifest {
                    mode: match manifest.mode {
                        ModeSchema::Isolated => ContextMode::Isolated,
                        ModeSchema::ListedDisk => ContextMode::ListedDisk,
                    },
                    logical_root: PathText(manifest.logical_root),
                    actual_root: PathText(manifest.actual_root),
                    manifest_hash: Sha256Text(manifest.manifest_hash),
                }),
                project_context: canonical(wire.project_context),
                rule_context: canonical(wire.rule_context),
                rule_observations: canonical(wire.rule_observations),
                selected_rules: wire
                    .selected_rules
                    .map(|rules| rules.into_iter().map(RuleName).collect()),
                actual_request_root: wire.actual_request_root.map(PathText),
                documented_reference_discrepancy: flag(wire.documented_reference_discrepancy),
                documented_tool_contract_difference: wire
                    .documented_tool_contract_difference
                    .map(flag),
                discrepancy_validation: wire.discrepancy_validation.map(|proof| {
                    DiscrepancyValidation {
                        discrepancy_basis: basis(proof.discrepancy_basis),
                        process_observation_group_sha256: sha(
                            proof.process_observation_group_sha256
                        ),
                        reference_expectation_kind: kind(proof.reference_expectation_kind),
                        settings_sha256: Sha256Text(proof.settings_sha256),
                        actual_executable_sha256: Sha256Text(proof.actual_executable_sha256),
                        profile_sha256: Sha256Text(proof.profile_sha256),
                        source_sha256: Sha256Text(proof.source_sha256),
                        reference_output_sha256: Sha256Text(proof.reference_output_sha256),
                        intended_output_sha256: Sha256Text(proof.intended_output_sha256),
                        file_metadata_sha256: sha(proof.file_metadata_sha256),
                    }
                }),
                raw_parity_matches_intended: wire.raw_parity_matches_intended.map(flag),
                diagnostics_match_intended: wire.diagnostics_match_intended.map(flag),
                process_observation: wire.process_observation.map(|process| ProcessObservation {
                    kind: kind(process.kind),
                    exit: ExitStatus(process.exit),
                    signal: process.signal.map(ExitStatus),
                    actual_request_root: PathText(process.actual_request_root),
                    observer_no_deadline: flag(process.observer_no_deadline),
                    executable_sha256: sha(process.executable_sha256),
                    settings_sha256: sha(process.settings_sha256),
                    metadata_error: process.metadata_error.map(ReasonText),
                    template_sha256: sha(process.template_sha256),
                    artifacts: process.artifacts.map(|artifacts| ProcessArtifacts {
                        stdout: pointer(artifacts.stdout),
                        stderr: pointer(artifacts.stderr),
                        invocation: pointer(artifacts.invocation),
                    }),
                }),
            }
        }
    }

    impl From<&ObservationDocument> for ObservationSchema {
        fn from(value: &ObservationDocument) -> Self {
            Self {
                rule: value.rule.0.clone(),
                name: value.name.0.clone(),
                side: match value.side {
                    Side::Reference => SideSchema::Reference,
                    Side::Candidate => SideSchema::Candidate,
                },
                complete: is(value.complete),
                error: value.error.as_ref().map(|error| error.0.clone()),
                case_passed: is(value.case_passed),
                files: value.files.iter().map(|file| file.0.clone()).collect(),
                source_sha256: opt_text(&value.source_sha256),
                file_metadata_sha256: opt_text(&value.file_metadata_sha256),
                project_inputs_sha256: opt_text(&value.project_inputs_sha256),
                expected: wire_findings(&value.expected),
                reference_expected: value.reference_expected.as_deref().map(wire_findings),
                diagnostics: value.diagnostics.as_deref().map(wire_findings),
                skipped: value.skipped.as_ref().map(|rows| {
                    rows.iter()
                        .map(|skip| SkippedSchema {
                            rule: skip.rule.0.clone(),
                            path: skip.path.0.clone(),
                            reason: SkipReasonSchema::SyntaxRejected,
                        })
                        .collect()
                }),
                observation_schema: value.observation_schema.0.clone(),
                expectation_contract: wire_contract(value.expectation_contract),
                execution_metadata: value.execution_metadata.as_ref().map(
                    |metadata| match metadata {
                        ExecutionMetadata::Unknown => ExecutionSchema::Unknown {},
                        ExecutionMetadata::Unavailable { reason } => ExecutionSchema::Unavailable {
                            reason: reason.clone(),
                        },
                        ExecutionMetadata::Complete => ExecutionSchema::Complete {},
                    },
                ),
                coverage: value.coverage.map(|coverage| CoverageSchema {
                    selected_rule_file_pairs: coverage.selected,
                    evaluated_rule_file_pairs: coverage.evaluated,
                    skipped_rule_file_pairs: coverage.skipped,
                    unknown_or_unread_rule_file_pairs: coverage.unknown_or_unread,
                }),
                context_manifest: value.context_manifest.as_ref().map(|manifest| {
                    ContextManifestSchema {
                        mode: match manifest.mode {
                            ContextMode::Isolated => ModeSchema::Isolated,
                            ContextMode::ListedDisk => ModeSchema::ListedDisk,
                        },
                        logical_root: manifest.logical_root.0.clone(),
                        actual_root: manifest.actual_root.0.clone(),
                        manifest_hash: manifest.manifest_hash.0.clone(),
                    }
                }),
                project_context: tree(&value.project_context),
                rule_context: tree(&value.rule_context),
                rule_observations: tree(&value.rule_observations),
                selected_rules: value
                    .selected_rules
                    .as_ref()
                    .map(|rules| rules.iter().map(|rule| rule.0.clone()).collect()),
                actual_request_root: value
                    .actual_request_root
                    .as_ref()
                    .map(|root| root.0.clone()),
                documented_reference_discrepancy: is(value.documented_reference_discrepancy),
                documented_tool_contract_difference: value
                    .documented_tool_contract_difference
                    .map(is),
                discrepancy_validation: value.discrepancy_validation.as_ref().map(|proof| {
                    ValidationSchema {
                        discrepancy_basis: wire_basis(proof.discrepancy_basis),
                        process_observation_group_sha256: opt_text(
                            &proof.process_observation_group_sha256,
                        ),
                        reference_expectation_kind: wire_kind(proof.reference_expectation_kind),
                        settings_sha256: text(&proof.settings_sha256),
                        actual_executable_sha256: text(&proof.actual_executable_sha256),
                        profile_sha256: text(&proof.profile_sha256),
                        source_sha256: text(&proof.source_sha256),
                        reference_output_sha256: text(&proof.reference_output_sha256),
                        intended_output_sha256: text(&proof.intended_output_sha256),
                        file_metadata_sha256: opt_text(&proof.file_metadata_sha256),
                    }
                }),
                raw_parity_matches_intended: value.raw_parity_matches_intended.map(is),
                diagnostics_match_intended: value.diagnostics_match_intended.map(is),
                process_observation: value.process_observation.as_ref().map(|process| {
                    ProcessObservationSchema {
                        kind: wire_kind(process.kind),
                        exit: process.exit.0,
                        signal: process.signal.map(|signal| signal.0),
                        actual_request_root: process.actual_request_root.0.clone(),
                        observer_no_deadline: is(process.observer_no_deadline),
                        executable_sha256: opt_text(&process.executable_sha256),
                        settings_sha256: opt_text(&process.settings_sha256),
                        metadata_error: process.metadata_error.as_ref().map(|e| e.0.clone()),
                        template_sha256: opt_text(&process.template_sha256),
                        artifacts: process.artifacts.as_ref().map(|artifacts| {
                            ProcessArtifactsSchema {
                                stdout: wire_pointer(&artifacts.stdout),
                                stderr: wire_pointer(&artifacts.stderr),
                                invocation: wire_pointer(&artifacts.invocation),
                            }
                        }),
                    }
                }),
            }
        }
    }
}
