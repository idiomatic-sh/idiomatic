//! The vec host resolves the request into named tables before the planner sees the pack.
//! Current requests can supply any relation with ordered columns and rows. Recorded
//! benchmark requests carry the three older declarations and configuration separately.

use ryo_engine::domain::plan_lowering::types::{
    ContextObservation, ContextTable, ObservationProducer, RuleContext,
};
use ryo_values::domain::descriptions::types::{ColumnName, RelationName};
use ryo_values::domain::observations::types::ObservationName;
use ryo_values::domain::runs::types::RuleId;
use serde_json::Value;

/// Read the parts used by the planner and file frame. A request may carry a large
/// project manifest and file list for the host; serde validates and skips those values.
fn fields(path: &str) -> Value {
    use serde::de::{IgnoredAny, MapAccess, Visitor};
    struct Metadata;
    impl<'de> Visitor<'de> for Metadata {
        type Value = Value;

        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a host request")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut entries: M) -> Result<Value, M::Error> {
            let mut fields = serde_json::Map::new();
            while let Some(name) = entries.next_key::<String>()? {
                match name.as_str() {
                    "named_tables"
                    | "rule_context"
                    | "rule_observations"
                    | "selected_rules"
                    | "project_search_roots"
                    | "rule_applicability" => {
                        fields.insert(name, entries.next_value()?);
                    }
                    _ => {
                        entries.next_value::<IgnoredAny>()?;
                    }
                }
            }
            Ok(Value::Object(fields))
        }
    }
    let input = std::fs::read(path).expect("request");
    if input.iter().find(|b| !b.is_ascii_whitespace()) != Some(&b'{') {
        return serde_json::from_slice(&input).expect("request json");
    }
    let mut parser = serde_json::Deserializer::from_slice(&input);
    let value = serde::Deserializer::deserialize_map(&mut parser, Metadata).expect("request json");
    parser.end().expect("end of request");
    value
}

fn named_tables(value: &Value) -> Vec<ContextTable> {
    value
        .as_object()
        .expect("named_tables object")
        .iter()
        .map(|(name, table)| {
            let strings = |v: &Value| {
                v.as_array()
                    .expect("table array")
                    .iter()
                    .map(|v| Box::<str>::from(v.as_str().expect("table text")))
                    .collect::<Vec<_>>()
            };
            let columns: Vec<_> = strings(&table["columns"])
                .into_iter()
                .map(ColumnName)
                .collect();
            let rows = table["rows"]
                .as_array()
                .expect("table rows")
                .iter()
                .map(|r| {
                    let cells = strings(r);
                    assert_eq!(cells.len(), columns.len(), "row width in table {name}");
                    cells
                })
                .collect();
            ContextTable {
                relation: RelationName(name.as_str().into()),
                columns,
                rows,
            }
        })
        .collect()
}

fn recorded_tables(r: &Value) -> Vec<ContextTable> {
    let texts = |v: &serde_json::Value| {
        v.as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(Box::<str>::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let names = |cols: &[&str]| {
        cols.iter()
            .map(|c| ColumnName((*c).into()))
            .collect::<Vec<_>>()
    };
    let conf = &r["rule_context"]["configuration"];
    let mut tables = Vec::new();
    for (which, relation) in r["rule_context"]["tables"]
        .as_object()
        .into_iter()
        .flatten()
    {
        let relation = RelationName(relation.as_str().expect("table name").into());
        let (columns, rows) = match which.as_str() {
            "selected" => (
                names(&["code"]),
                texts(&r["selected_rules"])
                    .into_iter()
                    .map(|c| vec![c])
                    .collect(),
            ),
            "disabled" => (
                names(&["file", "code"]),
                conf["disabled-rules"]
                    .as_object()
                    .into_iter()
                    .flatten()
                    .flat_map(|(f, cs)| {
                        texts(cs)
                            .into_iter()
                            .map(move |c| vec![Box::<str>::from(f.as_str()), c])
                    })
                    .collect(),
            ),
            "external" => (
                names(&["prefix"]),
                texts(&conf["external-prefixes"])
                    .into_iter()
                    .map(|p| vec![p])
                    .collect(),
            ),
            other => panic!("rule-context table {other}"),
        };
        tables.push(ContextTable {
            relation,
            columns,
            rows,
        });
    }
    tables
}

pub(super) fn rule_context(request: Option<&str>) -> RuleContext {
    let Some(path) = request else {
        return RuleContext::default();
    };
    context_from(&fields(path))
}

fn context_from(r: &Value) -> RuleContext {
    let tables = match r.get("named_tables") {
        Some(tables) => named_tables(tables),
        None => recorded_tables(r),
    };
    let mut observations = Vec::new();
    for specs in r["rule_observations"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(_, v)| v)
    {
        for spec in specs.as_array().into_iter().flatten() {
            observations.push(ContextObservation {
                name: ObservationName(spec["name"].as_str().expect("observation name").into()),
                columns: spec["columns"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|c| ColumnName(c["name"].as_str().expect("column").into()))
                    .collect(),
                producers: spec["producers"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|p| ObservationProducer {
                        rule: RuleId(p["rule"].as_str().expect("producer rule").into()),
                        output: ObservationName(
                            p["output"].as_str().expect("producer output").into(),
                        ),
                    })
                    .collect(),
            });
        }
    }
    RuleContext {
        tables,
        observations,
        ..RuleContext::default()
    }
}

pub(super) struct Metadata {
    pub context: RuleContext,
    pub accepted: Option<super::finish::json_input::Accepted>,
    pub search_roots: Option<Vec<String>>,
}

pub(super) fn metadata(path: &str) -> Metadata {
    let input = fields(path);
    Metadata {
        context: context_from(&input),
        accepted: input.get("rule_applicability").map(|v| {
            v.as_object()
                .into_iter()
                .flatten()
                .filter(|(_, v)| v.as_str() == Some("accepted-syntax"))
                .map(|(k, _)| k.clone())
                .collect()
        }),
        search_roots: input
            .get("project_search_roots")
            .and_then(Value::as_array)
            .map(|roots| {
                roots
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            }),
    }
}

pub(super) fn configuration(path: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Settings {
        parser_configuration: Option<String>,
    }
    serde_json::from_slice::<Settings>(&std::fs::read(path).ok()?)
        .ok()?
        .parser_configuration
}

pub(super) fn literal(text: &str) -> String {
    serde_json::from_str(text).expect("text literal")
}

pub(super) fn literals(text: &str) -> Vec<String> {
    serde_json::from_str(text).expect("text literals")
}
