//! Authored cases: independent sources, expected findings and declared inputs.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types;
use crate::outbound::suite::adapters::local::reference::schemas::ReferenceDiscrepancySchema;
use crate::outbound::suite::errors::SuiteError;
use crate::outbound::suite::types::ClaimedDiscrepancy;
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
use serde::Deserialize;
use std::collections::BTreeMap;
use crate::outbound::suite::adapters::local::declarations::schemas::{
    CaseContextSchema, ObservationBindingSchema, OptionsSchema, ProjectInventorySchema, RuleConfigurationSchema, RuleContextDeclarationSchema,
    observation_declarations,
    project_search_roots,
};

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

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
    performance: OptionsSchema,
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
        Option<ProjectInventorySchema>,
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
        Option<ReferenceDiscrepancySchema>,
    /// A documented difference from an already public tool selection contract.
    #[serde(default, rename = "tool-contract-difference")]
    tool_contract_difference:
        Option<ReferenceDiscrepancySchema>,
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

fn given_project_inventory(
    inventory: ProjectInventorySchema,
) -> Result<ryo::linter::GivenProjectInventory, Error> {
    let inventory: ryo::linter::GivenProjectInventory = inventory.into();
    for entry in &inventory.entries {
        ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(entry.path.0.as_ref()))?;
    }
    Ok(inventory)
}

/// Decode independent sources and expected byte ranges.
pub(in super::super) fn cases(text: &str) -> Result<types::Cases, Error> {
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
                        reference_discrepancy: ryo_benchmark_contracts::domain::workloads::types::ReferenceBehavior::Agrees,
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

/// Read the authored reference certificates, each with the authority it asserts.
pub(in super::super) fn case_claims(
    text: &str,
) -> Result<Vec<(String, ClaimedDiscrepancy, DiscrepancyBasis)>, SuiteError> {
    let mut claims = Vec::new();
    for case in toml::from_str::<CasesSchema>(text)?.cases {
        let (schema, basis) = match (case.reference_discrepancy, case.tool_contract_difference) {
            (Some(schema), None) => (schema, DiscrepancyBasis::LanguageDefect),
            (None, Some(schema)) => (schema, DiscrepancyBasis::ToolContract),
            (None, None) => continue,
            (Some(_), Some(_)) => {
                return Err(SuiteError::Invalid(
                    "one discrepancy authority per case".into(),
                ));
            }
        };
        let claim = ClaimedDiscrepancy::try_from(schema)
            .map_err(|error| SuiteError::Invalid(error.to_string()))?;
        claims.push((case.name, claim, basis));
    }
    Ok(claims)
}

/// Validate a rule identifier before it enters the benchmark domain.
pub(in super::super) fn rule(value: &str) -> Result<ryo::linter::RuleId, Error> {
    ryo::linter::RuleId::try_from(value).map_err(|error| Error::Invalid(error.to_string()))
}
