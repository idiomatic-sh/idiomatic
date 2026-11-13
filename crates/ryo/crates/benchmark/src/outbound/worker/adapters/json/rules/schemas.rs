//! Rule configuration and named observation declarations, checked into the core's types.
use crate::outbound::worker::adapters::json::types::DocumentWire;
use ryo::linter::{
    ObservationBinding, ObservationBindingName, ObservationColumn, ObservationColumnName,
    ObservationKind, ObservationName, ObservationProducer, RelativePath, RuleId,
};
use ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;
use ryo_benchmark_contracts::domain::context::types::{
    ExternalPrefix, PreparedRuleContext, RuleConfiguration, RuleContextTables, TableName,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Authored observation binding at the process boundary.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationBindingSchema {
    name: String,
    columns: Vec<ObservationColumnSchema>,
    producers: Vec<ObservationProducerSchema>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationColumnSchema {
    name: String,
    kind: ObservationKindSchema,
}
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ObservationKindSchema {
    Node,
    Symbol,
    File,
    Span,
    Number,
    Flag,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum ObservationProducerSchema {
    Output { rule: String, output: String },
    NoQuery { rule: String },
}

/// Convert checked identities while leaving producer/schema validation to request compilation.
impl TryFrom<DocumentWire<serde_json::Value>> for RuleObservationDeclarations {
    type Error = Error;
    fn try_from(wire: DocumentWire<serde_json::Value>) -> Result<Self, Self::Error> {
        let declarations: BTreeMap<String, Vec<ObservationBindingSchema>> =
            serde_json::from_value(wire.0).map_err(|error| Error::Json(Box::new(error)))?;
        let invalid =
            |message: String| Error::Invalid(format!("invalid observation declaration: {message}"));
        declarations
            .into_iter()
            .map(|(consumer, bindings)| {
                let consumer = RuleId::try_from(consumer.as_str())
                    .map_err(|error| invalid(error.to_string()))?;
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
                                                .map_err(|error| invalid(error.to_string()))?,
                                            output: ObservationName(output.into()),
                                        }
                                    }
                                    ObservationProducerSchema::NoQuery { rule } => {
                                        ObservationProducer::NoQuery {
                                            rule: RuleId::try_from(rule.as_str())
                                                .map_err(|error| invalid(error.to_string()))?,
                                        }
                                    }
                                })
                            })
                            .collect::<Result<Vec<_>, Error>>()?;
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
                    .collect::<Result<Vec<_>, Error>>()?;
                Ok((consumer, bindings))
            })
            .collect()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleContextTablesSchema {
    selected: String,
    disabled: String,
    external: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleConfigurationSchema {
    #[serde(rename = "external-prefixes")]
    external_prefixes: Vec<String>,
    #[serde(rename = "disabled-rules")]
    disabled_rules: BTreeMap<String, Vec<String>>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedRuleContextSchema {
    tables: RuleContextTablesSchema,
    configuration: RuleConfigurationSchema,
}

impl TryFrom<DocumentWire<serde_json::Value>> for PreparedRuleContext {
    type Error = Error;
    fn try_from(wire: DocumentWire<serde_json::Value>) -> Result<Self, Self::Error> {
        let schema: PreparedRuleContextSchema =
            serde_json::from_value(wire.0).map_err(|error| Error::Json(Box::new(error)))?;
        let invalid =
            |message: String| Error::Invalid(format!("invalid rule configuration: {message}"));
        Ok(Self {
            tables: RuleContextTables {
                selected: TableName(schema.tables.selected),
                disabled: TableName(schema.tables.disabled),
                external: TableName(schema.tables.external),
            },
            configuration: RuleConfiguration {
                external_prefixes: schema
                    .configuration
                    .external_prefixes
                    .into_iter()
                    .map(ExternalPrefix)
                    .collect(),
                disabled_rules: schema
                    .configuration
                    .disabled_rules
                    .into_iter()
                    .map(|(path, rules)| {
                        Ok((
                            RelativePath(path.into()),
                            rules
                                .into_iter()
                                .map(|rule| {
                                    RuleId::try_from(rule.as_str())
                                        .map_err(|error| invalid(error.to_string()))
                                })
                                .collect::<Result<Vec<_>, Error>>()?,
                        ))
                    })
                    .collect::<Result<_, Error>>()?,
            },
        })
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
