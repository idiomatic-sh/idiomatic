//! Values entering the vector benchmark from request, applicability and plan files.
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

pub(crate) type Accepted = HashSet<String>;

#[derive(Deserialize)]
pub(crate) struct Request {
    pub(crate) rule_context: Option<serde::de::IgnoredAny>,
    pub(crate) rule_observations: Option<serde::de::IgnoredAny>,
    pub root: String,
    pub files: Vec<String>,
    pub(crate) selected_rules: Vec<String>,
    #[serde(default)]
    pub(crate) file_rules: HashMap<String, String>,
    #[serde(default, deserialize_with = "accepted")]
    pub(crate) rule_applicability: Option<Accepted>,
}

impl Request {
}

pub(crate) fn accepted<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Accepted>, D::Error> {
    Ok(Option::<serde_json::Value>::deserialize(d)?
        .as_ref()
        .map(accepted_rules))
}

pub(crate) fn accepted_rules(value: &serde_json::Value) -> Accepted {
    value
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, v)| v.as_str() == Some("accepted-syntax"))
        .map(|(k, _)| k.clone())
        .collect()
}



pub(crate) fn check_context(path: &str) {
    #[derive(Deserialize)]
    struct Context {
        rule_context: Option<serde::de::IgnoredAny>,
        rule_observations: Option<serde::de::IgnoredAny>,
    }
    let context: Context = decode(&std::fs::read(path).expect("read request")).expect("request");
    assert!(
        context.rule_context.is_none() && context.rule_observations.is_none(),
        "this host provides no named context tables"
    );
}

/// Reference tools and fixture manifests supply typed wire records at this boundary.
pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(bytes)
}
