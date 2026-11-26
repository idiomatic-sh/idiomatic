//! Values entering the vector benchmark from request, applicability and plan files.
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

pub(crate) type Accepted = HashSet<String>;

#[derive(Deserialize)]
pub(crate) struct Request {
    rule_context: Option<serde::de::IgnoredAny>,
    rule_observations: Option<serde::de::IgnoredAny>,
    pub(crate) root: String,
    pub(crate) files: Vec<String>,
    pub(crate) selected_rules: Vec<String>,
    #[serde(default)]
    pub(crate) file_rules: HashMap<String, String>,
    #[serde(default, deserialize_with = "accepted")]
    pub(crate) rule_applicability: Option<Accepted>,
}

impl Request {
    pub(crate) fn check_file_context(&self) {
        assert!(
            self.rule_context.is_none() && self.rule_observations.is_none(),
            "this host provides no named context tables"
        );
    }
}

fn accepted<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Accepted>, D::Error> {
    Ok(Option::<serde_json::Value>::deserialize(d)?
        .as_ref()
        .map(accepted_rules))
}

fn accepted_rules(value: &serde_json::Value) -> Accepted {
    value
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, v)| v.as_str() == Some("accepted-syntax"))
        .map(|(k, _)| k.clone())
        .collect()
}

pub(crate) fn request(path: &str) -> Request {
    decode(&std::fs::read(path).expect("read request")).expect("request")
}

pub(crate) fn applicability(path: &str) -> Accepted {
    let input: serde_json::Value =
        decode(&std::fs::read(path).expect("read applicability")).expect("applicability");
    accepted_rules(input.get("rule_applicability").unwrap_or(&input))
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
pub(crate) fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(bytes)
}

/// The stage command supplies the engine's plan codec for its external snapshot.
pub(crate) fn snapshot<T, E: std::fmt::Display>(
    path: &str,
    decode_plan: impl FnOnce(&[u8]) -> Result<T, E>,
) -> T {
    let bytes = std::fs::read(path).expect("read plan");
    decode_plan(&bytes).unwrap_or_else(|error| panic!("{error}"))
}
