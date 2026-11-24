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
    #[serde(deserialize_with = "read_parameters")]
    pub parameters: Parameters,
}
#[derive(Clone, Eq, PartialEq, Hash, serde::Deserialize)]
pub(super) struct Hosted {
    pub name: String,
    pub columns: Vec<(String, String)>,
    #[serde(deserialize_with = "read_host_rows")]
    pub rows: std::sync::Arc<[std::sync::Arc<[HostCell]>]>,
    #[serde(default)]
    pub partial: Option<String>,
}

fn read_host_rows<'de, D: serde::Deserializer<'de>>(input: D) -> Result<std::sync::Arc<[std::sync::Arc<[HostCell]>]>, D::Error> {
    use serde::Deserialize;
    Vec::<Vec<HostCell>>::deserialize(input).map(|rows| rows.into_iter().map(std::sync::Arc::from).collect())
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub(super) enum HostCell {
    File { file: String },
    Number(i64),
    Text(String),
}

impl<'de> serde::Deserialize<'de> for HostCell {
    fn deserialize<D: serde::Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        use serde::de::{Error, IgnoredAny, MapAccess, SeqAccess, Visitor, value::MapAccessDeserializer};

        struct CellVisitor;
        impl<'de> Visitor<'de> for CellVisitor {
            type Value = Option<HostCell>;

            fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                fmt.write_str("a file object, an integer, or text")
            }

            fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
                #[derive(serde::Deserialize)]
                #[serde(untagged)]
                enum Object {
                    File { file: String },
                    Other(IgnoredAny),
                }
                let value = serde::Deserialize::deserialize(MapAccessDeserializer::new(map))?;
                Ok(match value {
                    Object::File { file } => Some(HostCell::File { file }),
                    Object::Other(_) => None,
                })
            }

            fn visit_i64<E: Error>(self, number: i64) -> Result<Self::Value, E> {
                Ok(Some(HostCell::Number(number)))
            }

            fn visit_u64<E: Error>(self, number: u64) -> Result<Self::Value, E> {
                Ok(i64::try_from(number).ok().map(HostCell::Number))
            }

            fn visit_str<E: Error>(self, text: &str) -> Result<Self::Value, E> {
                Ok(Some(HostCell::Text(text.to_owned())))
            }

            fn visit_string<E: Error>(self, text: String) -> Result<Self::Value, E> {
                Ok(Some(HostCell::Text(text)))
            }
            fn visit_bool<E: Error>(self, _: bool) -> Result<Self::Value, E> { Ok(None) }
            fn visit_f64<E: Error>(self, _: f64) -> Result<Self::Value, E> { Ok(None) }
            fn visit_unit<E: Error>(self) -> Result<Self::Value, E> { Ok(None) }
            fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<Self::Value, S::Error> {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(None)
            }

        }

        // Consume the complete cell before reporting the wire decoder's existing error.
        input.deserialize_any(CellVisitor)?.ok_or_else(|| {
            D::Error::custom("data did not match any variant of untagged enum HostCell")
        })
    }
}
pub(super) type Parameters = BTreeMap<Box<str>, PlanConstant>;
pub(super) fn read_parameters<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Parameters, D::Error> {
    use serde::Deserialize as _;
    Ok(BTreeMap::<Box<str>, Value>::deserialize(d)?
        .into_iter()
        .map(|(k, v)| (k, constant(&v)))
        .collect())
}
pub(super) fn project(line: &str) -> Project {
    serde_json::from_str(line).expect("project request")
}

pub(crate) fn decode<T: serde::de::DeserializeOwned>(input: &[u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(input)
}
pub(crate) fn decode_line<T: serde::de::DeserializeOwned>(
    line: &str,
) -> Result<T, serde_json::Error> {
    serde_json::from_str(line)
}
pub(super) fn scenario<T: serde::de::DeserializeOwned>(request: &str) -> Option<T> {
    #[derive(serde::Deserialize)]
    struct Header {
        inventory: Option<Vec<serde::de::IgnoredAny>>,
        selected_rules: Vec<String>,
    }
    let bytes = std::fs::read(request).expect("read request");
    let h: Header = decode(&bytes).ok()?;
    (h.inventory.is_some() && h.selected_rules == ["query"])
        .then(|| decode(&bytes).expect("scenario request"))
}
