//! The benchmark request carries caller values separately from the program text.
use ryo_query_language::domain::plan_lowering::types::RuleContext;
use ryo_values::domain::finish_plans::expressions::types::{
    ConstantEntry, ConstantEntryKey, PlanConstant, PlanNumber,
};
use ryo_values::domain::plan_parts::types::{LiteralText, WholeNumber};
use serde_json::Value;
use std::collections::BTreeMap;

pub fn constant(value: &Value) -> PlanConstant {
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

pub fn rule_context(request: Option<&str>) -> RuleContext {
    let mut context = RuleContext::default();
    if let Some(path) = request {
        super::finish::json_input::check_context(path);
        let input: Value =
            serde_json::from_slice(&std::fs::read(path).expect("read request")).expect("request");
        if let Some(tables) = input["hosted"].as_array() {
            use ryo_query_language::domain::plan_lowering::types::ContextTable;
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

#[derive(serde::Deserialize)]
pub struct Project {
    pub id: String,
    pub query: String,
    pub files: Vec<String>,
    pub inventory: Option<Vec<String>>,
    pub hosted: Vec<Hosted>,
    #[serde(deserialize_with = "read_parameters")]
    pub parameters: Parameters,
}
#[derive(Clone, Eq, PartialEq, Hash, serde::Deserialize)]
pub struct Hosted {
    pub name: String,
    pub columns: Vec<(String, String)>,
    #[serde(deserialize_with = "read_host_rows")]
    pub rows: std::sync::Arc<[std::sync::Arc<[HostCell]>]>,
    #[serde(default)]
    pub partial: Option<String>,
}

pub fn read_host_rows<'de, D: serde::Deserializer<'de>>(input: D) -> Result<std::sync::Arc<[std::sync::Arc<[HostCell]>]>, D::Error> {
    use serde::Deserialize;
    Vec::<Vec<HostCell>>::deserialize(input).map(|rows| rows.into_iter().map(std::sync::Arc::from).collect())
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum HostCell {
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
pub type Parameters = BTreeMap<Box<str>, PlanConstant>;
pub fn read_parameters<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Parameters, D::Error> {
    use serde::Deserialize as _;
    Ok(BTreeMap::<Box<str>, Value>::deserialize(d)?
        .into_iter()
        .map(|(k, v)| (k, constant(&v)))
        .collect())
}
pub fn project(line: &str) -> Project {
    serde_json::from_str(line).expect("project request")
}

pub fn decode<T: serde::de::DeserializeOwned>(input: &[u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(input)
}
pub fn decode_line<T: serde::de::DeserializeOwned>(
    line: &str,
) -> Result<T, serde_json::Error> {
    serde_json::from_str(line)
}
pub fn scenario<T: serde::de::DeserializeOwned>(request: &str) -> Option<T> {
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
