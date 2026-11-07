//! The library's own semantic model of a file, `serde_json::Value`, compared with the document the rows rebuild: every entry under the
//! mapping its owner is, in the order of its ordinal, and every sequence's elements in the order of theirs. What the oracle rail of
//! the conformance holds `mapping`, `entry` and `sequence_element` to. `serde_json` reads json as the standard defines it; a file
//! with comments or a trailing comma is no document to it, and `json5`, which reads both, gives the document instead.

use std::collections::BTreeMap;

use ryo::languages::Relation;
use ryo::languages::contract::{Cell, OracleRows};
use serde_json::Value;

/// A document, as the oracle compares it: objects by key, arrays in order, and values.
#[derive(Debug, PartialEq)]
enum Document {
    /// An object, its members by key.
    Object(BTreeMap<String, Document>),
    /// An array, its elements in order.
    Array(Vec<Document>),
    /// A string, number, boolean or `null`, compared as the library reads it.
    Scalar(Value),
}

/// A value as the oracle compares it.
fn compared(value: Value) -> Document {
    match value {
        Value::Object(object) => Document::Object(
            object
                .into_iter()
                .map(|(key, each)| (key, compared(each)))
                .collect(),
        ),
        Value::Array(array) => Document::Array(array.into_iter().map(compared).collect()),
        kept @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => {
            Document::Scalar(kept)
        }
    }
}

/// The text of a name cell; nothing for any other.
fn name(cell: Option<&Cell>) -> String {
    match cell {
        Some(Cell::Name(text)) => text.clone(),
        _ => String::new(),
    }
}

/// The place of a node cell.
fn node(cell: Option<&Cell>) -> Option<u32> {
    match cell {
        Some(Cell::Node(place)) => Some(*place),
        _ => None,
    }
}

/// The number of a number cell.
fn number(cell: Option<&Cell>) -> Option<i64> {
    match cell {
        Some(Cell::Number(number)) => Some(number.0),
        _ => None,
    }
}

/// An entry or an element as the rows say it: its ordinal, its key (none for an element), its kind, its element, its text and what it
/// decodes to.
#[derive(Clone)]
struct Said {
    /// Its place among its owner's entries or elements.
    ordinal: i64,
    /// Its key; empty for an element.
    key: String,
    /// The kind of its value, as the rows name it.
    kind: String,
    /// The node of its value.
    at: u32,
    /// The text of its value as written.
    text: String,
    /// What a string value decodes to.
    decoded: String,
}

/// What the rows say, by owner.
struct Rows {
    /// The entries of each mapping, by the mapping's node.
    entries: BTreeMap<u32, Vec<Said>>,
    /// The elements of each sequence, by the node that holds it.
    elements: BTreeMap<u32, Vec<Said>>,
    /// The mapping each entry or element opens, by the node that opens it.
    opened: BTreeMap<u32, u32>,
}

/// A scalar the rows say: a string by what it decodes to, any other by its text, read as the library reads a value.
fn scalar(said: &Said) -> Result<Document, String> {
    if said.kind == "string" {
        return Ok(Document::Scalar(Value::String(said.decoded.clone())));
    }
    serde_json::from_str::<Value>(&said.text)
        .map(Document::Scalar)
        .map_err(|error| {
            format!(
                "the {} at node {} is no value: {:?}: {error}",
                said.kind, said.at, said.text
            )
        })
}

/// The entries of the mapping `id`, in the order of their ordinals.
fn object(rows: &Rows, id: u32) -> Result<Document, String> {
    let mut out = BTreeMap::new();
    let mut list = rows.entries.get(&id).cloned().unwrap_or_default();
    list.sort_by_key(|each| each.ordinal);
    let mut before = -1;
    for said in &list {
        if said.ordinal <= before {
            return Err(format!(
                "the entry `{}` of node {id} has ordinal {}, not after {before}",
                said.key, said.ordinal
            ));
        }
        before = said.ordinal;
        let value = match said.kind.as_str() {
            "inline-mapping" => {
                let opened = rows
                    .opened
                    .get(&said.at)
                    .ok_or_else(|| format!("the entry `{}` opens no mapping", said.key))?;
                object(rows, *opened)?
            }
            "sequence" => array(rows, said.at)?,
            _ => scalar(said)?,
        };
        // A key said twice keeps its last value, as `serde_json` reads a duplicate key: the one it replaces is dropped.
        drop(out.insert(said.key.clone(), value));
    }
    Ok(Document::Object(out))
}

/// The elements of the sequence `owner` holds, in the order of their ordinals.
fn array(rows: &Rows, owner: u32) -> Result<Document, String> {
    let mut list = rows.elements.get(&owner).cloned().unwrap_or_default();
    list.sort_by_key(|each| each.ordinal);
    let mut out = Vec::new();
    let mut before = -1;
    for said in &list {
        if said.ordinal <= before {
            return Err(format!(
                "the element at node {} has ordinal {}, not after {before}",
                said.at, said.ordinal
            ));
        }
        before = said.ordinal;
        out.push(match said.kind.as_str() {
            "inline-mapping" => object(rows, said.at)?,
            "sequence" => array(rows, said.at)?,
            _ => scalar(said)?,
        });
    }
    Ok(Document::Array(out))
}

/// The document `text` is, as the libraries read it: `serde_json` first, and `json5` for a text with comments or a trailing comma.
fn read(text: &str) -> Option<Value> {
    serde_json::from_str::<Value>(text)
        .ok()
        .or_else(|| json5::from_str::<Value>(text).ok())
}

/// Compares the document the rows of `text` say with the one the libraries read; what differs, one sentence; nothing when they agree,
/// or when the libraries read no document.
pub(crate) fn oracle(text: &str, rows: &OracleRows) -> Vec<String> {
    let Some(wanted) = read(text) else {
        return Vec::new();
    };
    let mut said = Rows {
        entries: BTreeMap::new(),
        elements: BTreeMap::new(),
        opened: BTreeMap::new(),
    };
    for row in rows.of(Relation::Entry) {
        let (Some(at), Some(owner), Some(ordinal)) =
            (node(row.get(3)), node(row.get(5)), number(row.get(4)))
        else {
            return vec!["an entry row has no occurrence, owner or ordinal".to_owned()];
        };
        said.entries.entry(owner).or_default().push(Said {
            ordinal,
            key: name(row.get(1)),
            kind: name(row.get(2)),
            at,
            text: name(row.get(8)),
            decoded: name(row.first()),
        });
    }
    for row in rows.of(Relation::SequenceElement) {
        let (Some(at), Some(owner), Some(ordinal)) =
            (node(row.get(2)), node(row.get(4)), number(row.get(3)))
        else {
            return vec!["a sequence element row has no occurrence, owner or ordinal".to_owned()];
        };
        said.elements.entry(owner).or_default().push(Said {
            ordinal,
            key: String::new(),
            kind: name(row.get(1)),
            at,
            text: name(row.get(7)),
            decoded: name(row.first()),
        });
    }
    for row in rows.of(Relation::Mapping) {
        if let (Some(id), Some(owner)) = (node(row.first()), node(row.get(3)))
            && said.opened.insert(owner, id).is_some()
        {
            return vec![format!("node {owner} opens two mappings")];
        }
    }
    let rebuilt = match &wanted {
        Value::Object(_) => object(&said, 0),
        Value::Array(_) => array(&said, 0),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => return Vec::new(),
    };
    match rebuilt {
        Ok(document) => {
            if document == compared(wanted) {
                Vec::new()
            } else {
                vec!["the document the rows say is not the one the libraries read".to_owned()]
            }
        }
        Err(why) => vec![why],
    }
}
