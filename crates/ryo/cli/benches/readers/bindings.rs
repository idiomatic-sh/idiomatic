//! The benchmark request carries caller values separately from the program text.
use ryo_engine::domain::plan_lowering::types::RuleContext;
use ryo_values::domain::finish_plans::expressions::types::{
    ConstantEntry, ConstantEntryKey, PlanConstant, PlanNumber,
};
use ryo_values::domain::plan_parts::types::{LiteralText, WholeNumber};
use serde_json::Value;

pub(super) fn constant(value: &Value) -> PlanConstant {
    match value {
        Value::Bool(v) => PlanConstant::Text(LiteralText(v.to_string().into())),
        Value::String(v) => PlanConstant::Text(LiteralText(v.as_str().into())),
        Value::Number(n) => PlanConstant::Number(PlanNumber::Whole(WholeNumber(
            n.as_i64().expect("whole number parameter"),
        ))),
        Value::Array(v) => PlanConstant::List(v.iter().map(constant).collect()),
        Value::Object(v) => PlanConstant::Map(
            v.iter()
                .map(|(k, v)| ConstantEntry {
                    key: ConstantEntryKey(k.as_str().into()),
                    value: constant(v),
                })
                .collect(),
        ),
        Value::Null => panic!("null query parameter"),
    }
}

pub(super) fn rule_context(request: Option<&str>) -> RuleContext {
    let mut context = super::reader_cli::rule_context(request);
    if let Some(path) = request {
        let input: Value =
            serde_json::from_slice(&std::fs::read(path).expect("read request")).expect("request");
        if let Some(tables) = input["hosted"].as_array() {
            use ryo_engine::domain::plan_lowering::types::ContextTable;
            use ryo_values::domain::descriptions::types::{ColumnName, RelationName};
            for table in tables {
                let name = table["name"].as_str().expect("host table name");
                let schema = table["columns"].as_array().expect("host schema");
                assert!(
                    schema.iter().all(|c| c[1] == "Text"),
                    "host relation {name} needs typed context columns"
                );
                context.tables.push(ContextTable {
                    relation: RelationName(name.into()),
                    columns: schema
                        .iter()
                        .map(|c| ColumnName(c[0].as_str().expect("column name").into()))
                        .collect(),
                    rows: table["rows"]
                        .as_array()
                        .expect("host rows")
                        .iter()
                        .map(|row| {
                            row.as_array()
                                .expect("host row")
                                .iter()
                                .map(|cell| cell.as_str().expect("text cell").into())
                                .collect()
                        })
                        .collect(),
                });
            }
        }
        if let Some(values) = input["parameters"].as_object().filter(|m| !m.is_empty()) {
            let [id] = input["selected_rules"]
                .as_array()
                .expect("selected rules")
                .as_slice()
            else {
                panic!("single-rule parameters require one selected rule");
            };
            let _previous = context.parameters.insert(
                id.as_str().expect("rule id").into(),
                values
                    .iter()
                    .map(|(k, v)| (k.as_str().into(), constant(v)))
                    .collect(),
            );
        }
    }
    context
}
