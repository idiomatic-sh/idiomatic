//! The values an authored document holds, as the authoring's canonical JSON writes them, and the refusal of a value the
//! authoring does not write.
use super::super::constants;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

/// A refusal of the authored cases.
pub(in super::super) fn invalid(text: &str) -> Error {
    Error::Invalid(text.to_owned())
}

/// A table of texts, as the authoring builds them.
pub(in super::super) fn table<const N: usize>(entries: [(&str, Canonical); N]) -> Canonical {
    Canonical::Table(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

/// A text value.
pub(in super::super) fn text(value: &str) -> Canonical {
    Canonical::Text(value.to_owned())
}

/// Read one TOML document as a table of canonical values.
pub(in super::super) fn document(path: &Path) -> Result<BTreeMap<String, Canonical>, Error> {
    let parsed: toml::Table = toml::from_str(&std::fs::read_to_string(path)?)?;
    match Canonical::try_from(toml::Value::Table(parsed))? {
        Canonical::Table(table) => Ok(table),
        Canonical::Null
        | Canonical::Flag(_)
        | Canonical::Integer(_)
        | Canonical::Float(_)
        | Canonical::Text(_)
        | Canonical::List(_) => Err(invalid(constants::UNSUPPORTED_DOCUMENT_VALUE)),
    }
}

/// A value as the authoring's canonical JSON writes it: keys sorted, compact, characters beyond ASCII unescaped.
#[derive(Clone, Debug, PartialEq, serde::Serialize, Deserialize)]
#[serde(untagged)]
pub(in super::super) enum Canonical {
    /// No value.
    Null,
    /// A flag.
    Flag(bool),
    /// A whole number.
    Integer(i64),
    /// A number with a fraction.
    Float(f64),
    /// A text.
    Text(String),
    /// Values in order.
    List(Vec<Canonical>),
    /// Values by key, in key order.
    Table(BTreeMap<String, Canonical>),
}

impl Canonical {
    /// The compact text of the value.
    pub(in super::super) fn text(&self) -> Result<String, Error> {
        serde_json::to_string(self).map_err(|error| Error::Json(Box::new(error)))
    }
    /// The SHA-256 of the compact text.
    pub(in super::super) fn digest(&self) -> Result<String, Error> {
        Ok(format!("{:x}", Sha256::digest(self.text()?.as_bytes())))
    }
    /// The text, when the value is one.
    pub(in super::super) fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Null
            | Self::Flag(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::List(_)
            | Self::Table(_) => None,
        }
    }
    /// The table, when the value is one.
    pub(in super::super) fn as_table(&self) -> Option<&BTreeMap<String, Self>> {
        match self {
            Self::Table(table) => Some(table),
            Self::Null
            | Self::Flag(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Text(_)
            | Self::List(_) => None,
        }
    }
    /// The list, when the value is one.
    pub(in super::super) fn as_list(&self) -> Option<&[Self]> {
        match self {
            Self::List(items) => Some(items),
            Self::Null
            | Self::Flag(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Text(_)
            | Self::Table(_) => None,
        }
    }
    /// The whole number, when the value is one.
    pub(in super::super) fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(number) => Some(*number),
            Self::Null
            | Self::Flag(_)
            | Self::Float(_)
            | Self::Text(_)
            | Self::List(_)
            | Self::Table(_) => None,
        }
    }
}

impl TryFrom<toml::Value> for Canonical {
    type Error = Error;
    fn try_from(value: toml::Value) -> Result<Self, Error> {
        Ok(match value {
            toml::Value::String(text) => Self::Text(text),
            toml::Value::Integer(number) => Self::Integer(number),
            toml::Value::Float(number) => Self::Float(number),
            toml::Value::Boolean(flag) => Self::Flag(flag),
            toml::Value::Datetime(_) => {
                return Err(Error::Invalid(constants::UNSUPPORTED_DOCUMENT_VALUE.into()));
            }
            toml::Value::Array(items) => Self::List(
                items
                    .into_iter()
                    .map(Self::try_from)
                    .collect::<Result<_, _>>()?,
            ),
            toml::Value::Table(table) => Self::Table(
                table
                    .into_iter()
                    .map(|(key, value)| Ok((key, Self::try_from(value)?)))
                    .collect::<Result<_, Error>>()?,
            ),
        })
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
