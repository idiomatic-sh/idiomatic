//! The declarations a profile and a cases document share: rule observations, rule context and project scope.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

/// Explicit authored input at a private process/TOML serialization boundary.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::outbound::suite::adapters::local) struct ObservationBindingSchema {
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
pub(in crate::outbound::suite::adapters::local) fn observation_declarations(
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
pub(in crate::outbound::suite::adapters::local) struct RuleConfigurationSchema {
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
pub(in crate::outbound::suite::adapters::local) struct RuleContextDeclarationSchema {
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
pub(in crate::outbound::suite::adapters::local) fn project_search_roots(
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


#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
