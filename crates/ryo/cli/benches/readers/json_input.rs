//! The benchmark request carries caller values separately from the program text.
use ryo_engine::domain::plan_lowering::types::RuleContext;
use ryo_values::domain::finish_plans::expressions::types::{
    ConstantEntry, ConstantEntryKey, PlanConstant, PlanNumber,
};
use ryo_values::domain::plan_parts::types::{LiteralText, WholeNumber};
use serde_json::Value;

fn constant(value: &Value) -> PlanConstant {
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

type FileRequest = super::finish::json_input::Request;

#[derive(serde::Deserialize)]
struct FileBindings {
    hosted: Option<Vec<TextTable>>,
    parameters: Option<BTreeMap<String, Value>>,
}

#[derive(serde::Deserialize)]
struct TextTable {
    name: String,
    columns: Vec<(String, String)>,
    rows: Vec<Vec<Box<str>>>,
}

impl FileBindings {
    fn context(self, selected: &[String]) -> RuleContext {
        use ryo_engine::domain::plan_lowering::types::ContextTable;
        use ryo_values::domain::descriptions::types::{ColumnName, RelationName};
        let mut context = RuleContext::default();
        for table in self.hosted.unwrap_or_default() {
            assert!(table.columns.iter().all(|(_, kind)| kind == "Text"),
                "host relation {} needs typed context columns", table.name);
            context.tables.push(ContextTable {
                relation: RelationName(table.name.into()),
                columns: table.columns.into_iter().map(|(name, _)| ColumnName(name.into())).collect(),
                rows: table.rows,
            });
        }
        if let Some(values) = self.parameters.filter(|values| !values.is_empty()) {
            let [id] = selected else { panic!("single-rule parameters require one selected rule") };
            let _previous = context.parameters.insert(
                id.as_str().into(),
                values.into_iter().map(|(name, value)| (name.into(), constant(&value))).collect(),
            );
        }
        context
    }
}

pub(super) fn file_request(path: &str) -> (FileRequest, RuleContext) {
    #[derive(serde::Deserialize)]
    struct Input {
        #[serde(flatten)]
        file: FileRequest,
        #[serde(flatten)]
        bindings: FileBindings,
    }
    let input: Input = super::finish::json_input::decode(&std::fs::read(path).expect("read request"))
        .expect("file request");
    input.file.check_file_context();
    let context = input.bindings.context(&input.file.selected_rules);
    (input.file, context)
}

pub(super) fn rule_context(request: Option<&str>) -> RuleContext {
    #[derive(serde::Deserialize)]
    struct Input {
        rule_context: Option<serde::de::IgnoredAny>,
        rule_observations: Option<serde::de::IgnoredAny>,
        #[serde(default)]
        selected_rules: Vec<String>,
        #[serde(flatten)]
        bindings: FileBindings,
    }
    let Some(path) = request else { return RuleContext::default() };
    let input: Input = super::finish::json_input::decode(&std::fs::read(path).expect("read request"))
        .expect("request context");
    assert!(input.rule_context.is_none() && input.rule_observations.is_none(),
        "this host provides no named context tables");
    input.bindings.context(&input.selected_rules)
}

use ryo::linter::*;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
#[derive(serde::Deserialize)]
struct ShippingWire {
    root: PathBuf,
    #[serde(default)]
    given: bool,
    files: Vec<String>,
    selected_rules: Vec<String>,
    #[serde(default)]
    parameters: BTreeMap<String, Value>,
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    inventory: Option<Vec<String>>,
    #[serde(default)]
    hosted: Vec<ShippingTable>,
}

#[derive(serde::Deserialize)]
struct ShippingTable {
    name: String,
    columns: Vec<(String, String)>,
    rows: Vec<Vec<Value>>,
}

impl ShippingTable {
    fn into_relation(self) -> HostRelation {
        let columns = self
            .columns
            .into_iter()
            .map(|(name, kind)| HostColumn {
                name: HostColumnName(name.into()),
                kind: match kind.as_str() {
                    "File" => HostKind::File,
                    "Text" => HostKind::Text,
                    "Number" => HostKind::Number,
                    _ => panic!("unknown host column kind {kind}"),
                },
            })
            .collect::<Vec<_>>();
        let rows = self
            .rows
            .into_iter()
            .map(|row| {
                assert_eq!(row.len(), columns.len(), "host row width");
                HostRow(
                    row.into_iter()
                        .zip(&columns)
                        .map(|(v, col)| match col.kind {
                            HostKind::File => Cell::File(RelativePath(
                                v["file"].as_str().expect("file cell").into(),
                            )),
                            HostKind::Text => {
                                Cell::Text(HostText(v.as_str().expect("text cell").into()))
                            }
                            HostKind::Number => {
                                Cell::Number(HostNumber(v.as_i64().expect("number cell")))
                            }
                            _ => panic!("unsupported hosted column"),
                        })
                        .collect(),
                )
            })
            .collect();
        HostRelation {
            name: HostRelationName(self.name.into()),
            columns,
            rows,
        }
    }
}

fn parameter(v: &Value) -> ParameterValue {
    let text = |v: &Value| ParameterText(v.as_str().expect("text parameter").into());
    match v {
        Value::String(_) => ParameterValue::Text(text(v)),
        Value::Number(n) => {
            ParameterValue::Number(ParameterNumber(n.as_i64().expect("whole number")))
        }
        Value::Bool(b) => ParameterValue::Flag(if *b { FlagState::On } else { FlagState::Off }),
        Value::Array(items) if items.iter().all(Value::is_string) => {
            ParameterValue::Texts(items.iter().map(text).collect())
        }
        Value::Array(items) if items.iter().all(Value::is_i64) => ParameterValue::Numbers(
            ParameterNumbers(items.iter().map(|n| n.as_i64().unwrap()).collect()),
        ),
        Value::Array(items) if items.iter().all(Value::is_array) => ParameterValue::TextSets(
            items
                .iter()
                .map(|a| a.as_array().unwrap().iter().map(text).collect())
                .collect(),
        ),
        Value::Array(items) if items.iter().all(Value::is_object) => ParameterValue::Tables(
            items
                .iter()
                .map(|m| {
                    ParameterTable(
                        m.as_object()
                            .unwrap()
                            .iter()
                            .map(|(k, v)| ParameterCell {
                                key: ParameterText(k.as_str().into()),
                                value: text(v),
                            })
                            .collect(),
                    )
                })
                .collect(),
        ),
        _ => panic!("unrepresented parameter {v}"),
    }
}

pub(super) struct Shipping {
    pub root: PathBuf,
    pub given: bool,
    pub files: Vec<String>,
    pub selected_rules: Vec<String>,
    pub parameters: Vec<(ParameterName, ParameterValue)>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub inventory: Option<Vec<String>>,
    pub hosted: Vec<HostRelation>,
}
pub(super) fn shipping(path: &Path) -> Shipping {
    let r: ShippingWire =
        serde_json::from_slice(&std::fs::read(path).expect("request")).expect("shipping request");
    Shipping {
        root: r.root,
        given: r.given,
        files: r.files,
        selected_rules: r.selected_rules,
        parameters: r
            .parameters
            .into_iter()
            .map(|(k, v)| (ParameterName(k.into()), parameter(&v)))
            .collect(),
        include: r.include,
        exclude: r.exclude,
        inventory: r.inventory,
        hosted: r
            .hosted
            .into_iter()
            .map(ShippingTable::into_relation)
            .collect(),
    }
}
#[derive(serde::Deserialize)]
pub(super) struct Project {
    pub id: String,
    pub query: String,
    pub files: Vec<String>,
    pub inventory: Option<Vec<String>>,
    pub hosted: Vec<Hosted>,
    #[serde(deserialize_with = "constants")]
    pub parameters: BTreeMap<String, PlanConstant>,
}
#[derive(serde::Deserialize, Debug, PartialEq, Eq, Hash)]
pub(super) struct Hosted {
    pub name: String,
    pub columns: Vec<(String, String)>,
    pub rows: Vec<Vec<ProjectCell>>,
    #[serde(default)]
    pub partial: Option<String>,
}

#[derive(serde::Deserialize, Debug, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub(super) enum ProjectCell {
    File { file: String },
    Text(String),
    Number(i64),
    Unsupported(#[serde(deserialize_with = "ignore_cell")] ()),
}
fn ignore_cell<'de, D: serde::Deserializer<'de>>(d: D) -> Result<(), D::Error> {
    use serde::Deserialize as _;
    serde::de::IgnoredAny::deserialize(d).map(|_| ())
}
pub(super) fn constants<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<BTreeMap<String, PlanConstant>, D::Error> {
    use serde::Deserialize as _;
    Ok(BTreeMap::<String, Value>::deserialize(d)?
        .into_iter()
        .map(|(k, v)| (k, constant(&v)))
        .collect())
}
pub(super) fn project(line: &str) -> Project {
    serde_json::from_str(line).expect("project request")
}
