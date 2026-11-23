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
    let r = document(path);
    let tables = match r.get("named_tables") {
        Some(tables) => named_tables(tables),
        None => recorded_tables(&r),
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

fn document(path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read request")).expect("request JSON")
}

pub(super) fn workers(path: Option<&str>) -> Option<std::num::NonZeroUsize> {
    let doc = document(path?);
    let value = doc.get("workers")?;
    let count = value.as_u64().and_then(|n| usize::try_from(n).ok())
        .and_then(std::num::NonZeroUsize::new).expect("workers must be a positive integer");
    Some(count)
}
