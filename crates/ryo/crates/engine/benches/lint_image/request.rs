//! Convert the frozen benchmark request's host inputs to public SDK values.
use crate::sdk;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct Context {
    configuration: Configuration,
    tables: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Configuration {
    disabled_rules: BTreeMap<String, Vec<String>>,
    external_prefixes: Vec<String>,
}

impl Context {
    pub fn tables(self, selected: &[String]) -> Result<Vec<sdk::HostRelation>, String> {
        let text = |s: &str| sdk::Cell::Text(sdk::HostText(s.into()));
        self.tables
            .into_iter()
            .map(|(role, name)| {
                let (schema, rows) = match role.as_str() {
                    "selected" => (
                        vec![("code", sdk::HostKind::Text)],
                        selected
                            .iter()
                            .map(|s| sdk::HostRow(vec![text(s)]))
                            .collect(),
                    ),
                    "external" => (
                        vec![("prefix", sdk::HostKind::Text)],
                        self.configuration
                            .external_prefixes
                            .iter()
                            .map(|s| sdk::HostRow(vec![text(s)]))
                            .collect(),
                    ),
                    "disabled" => (
                        vec![("file", sdk::HostKind::File), ("code", sdk::HostKind::Text)],
                        self.configuration
                            .disabled_rules
                            .iter()
                            .flat_map(|(p, rules)| {
                                rules.iter().map(move |rule| {
                                    sdk::HostRow(vec![
                                        sdk::Cell::File(sdk::RelativePath(p.as_str().into())),
                                        text(rule),
                                    ])
                                })
                            })
                            .collect(),
                    ),
                    _ => return Err(format!("unknown host declaration {role}")),
                };
                let columns = schema
                    .into_iter()
                    .map(|(name, kind)| sdk::HostColumn {
                        name: sdk::HostColumnName(name.into()),
                        kind,
                    })
                    .collect();
                Ok(sdk::HostRelation {
                    name: sdk::HostRelationName(name.into()),
                    columns,
                    rows,
                })
            })
            .collect()
    }
}

#[derive(Deserialize)]
pub struct Observation {
    name: Box<str>,
    columns: Vec<Column>,
    producers: Vec<Producer>,
}

#[derive(Deserialize)]
struct Column {
    name: Box<str>,
    kind: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum Producer {
    Output { rule: String, output: Box<str> },
    NoQuery { rule: String },
}

impl Observation {
    pub fn binding(self) -> Result<sdk::ObservationBinding, String> {
        let producers = self
            .producers
            .into_iter()
            .map(|producer| {
                let id = |name: &str| sdk::RuleId::try_from(name).map_err(|e| e.to_string());
                Ok(match producer {
                    Producer::Output { rule, output } => sdk::ObservationProducer::Output {
                        rule: id(&rule)?,
                        output: sdk::ObservationName(output),
                    },
                    Producer::NoQuery { rule } => {
                        sdk::ObservationProducer::NoQuery { rule: id(&rule)? }
                    }
                })
            })
            .collect::<Result<_, String>>()?;
        let columns = self
            .columns
            .into_iter()
            .map(|c| {
                let kind = match c.kind.as_str() {
                    "file" => sdk::ObservationKind::File,
                    "node" => sdk::ObservationKind::Node,
                    "symbol" => sdk::ObservationKind::Symbol,
                    "number" => sdk::ObservationKind::Number,
                    "flag" => sdk::ObservationKind::Flag,
                    "span" => sdk::ObservationKind::Span,
                    other => return Err(format!("unknown observation kind {other}")),
                };
                Ok(sdk::ObservationColumn {
                    name: sdk::ObservationColumnName(c.name),
                    kind,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(sdk::ObservationBinding {
            name: sdk::ObservationBindingName(self.name),
            columns,
            producers,
        })
    }
}
