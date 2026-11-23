use ryo_engine::domain::plan_lowering::types::{
    ContextObservation, ContextTable, ObservationProducer, RuleContext,
};
use ryo_values::domain::descriptions::types::{ColumnName, RelationName};
use ryo_values::domain::observations::types::ObservationName;
use ryo_values::domain::runs::types::RuleId;

/// The run's rule context from its request (the bench's `request.json`), as the shipping host gives it to the compiler:
/// each `rule_context.tables` entry's rows (`selected`: the selected rules' codes; `disabled`: the configuration's (file,
/// code) pairs; `external`: its external prefixes), and each `rule_observations` spec. No request: none.
pub(super) fn rule_context(request: Option<&str>) -> RuleContext {
    let Some(path) = request else {
        return RuleContext::default();
    };
    let r: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("request"))
            .expect("request json");
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
    }
}
