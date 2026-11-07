//! The library's own semantic model of a file, `toml::Value`, compared with the document the rows rebuild: every entry under the
//! mapping its owner is, in the order of its ordinal, and every sequence's elements in the order of theirs. What the oracle rail of
//! the conformance holds `mapping`, `entry` and `sequence_element` to.

use std::collections::BTreeMap;

use ryo::languages::Relation;
use ryo::languages::contract::{Cell, OracleRows};

/// A document, as the oracle compares it: tables by key, arrays in order, and values.
#[derive(Debug, PartialEq)]
enum Document {
    /// A table, its entries by key.
    Table(BTreeMap<String, Document>),
    /// An array, its elements in order.
    Array(Vec<Document>),
    /// A float that is not a number: equal to another, as both readings of `nan` are, and to nothing of another kind.
    Nan,
    /// Any other scalar, compared as the library reads it.
    Value(toml::Value),
}

/// A value as the oracle compares it: a float that is not a number stays a float, equal to another such float.
fn compared(value: toml::Value) -> Document {
    match value {
        toml::Value::Float(float) if float.is_nan() => Document::Nan,
        toml::Value::Table(table) => Document::Table(
            table
                .into_iter()
                .map(|(key, each)| (key, compared(each)))
                .collect(),
        ),
        toml::Value::Array(array) => Document::Array(array.into_iter().map(compared).collect()),
        kept @ (toml::Value::String(_)
        | toml::Value::Integer(_)
        | toml::Value::Float(_)
        | toml::Value::Boolean(_)
        | toml::Value::Datetime(_)) => Document::Value(kept),
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
    /// The mapping each entry or header opens, by the node that opens it.
    opened: BTreeMap<u32, u32>,
}

/// A scalar the rows say: a string by what it decodes to, any other by its text, read as the library reads a value.
fn scalar(said: &Said) -> Result<Document, String> {
    if said.kind == "string" {
        return Ok(Document::Value(toml::Value::String(said.decoded.clone())));
    }
    let table = toml::from_str::<toml::Table>(&format!("v = {}", said.text)).map_err(|error| {
        format!(
            "the {} at node {} is no value: {:?}: {error}",
            said.kind, said.at, said.text
        )
    })?;
    table
        .get("v")
        .cloned()
        .map(compared)
        .ok_or_else(|| "no value".to_owned())
}

/// The entries of the mapping `id`, in the order of their ordinals.
fn table(rows: &Rows, id: u32) -> Result<Document, String> {
    let mut out = BTreeMap::new();
    let mut list = rows.entries.get(&id).cloned().unwrap_or_default();
    list.sort_by_key(|each| each.ordinal);
    for (place, said) in list.iter().enumerate() {
        if usize::try_from(said.ordinal).ok() != Some(place) {
            return Err(format!(
                "the entry `{}` of node {id} has ordinal {}, not {place}",
                said.key, said.ordinal
            ));
        }
        let value = match said.kind.as_str() {
            "mapping" | "inline-mapping" => {
                let opened = rows
                    .opened
                    .get(&said.at)
                    .ok_or_else(|| format!("the entry `{}` opens no mapping", said.key))?;
                table(rows, *opened)?
            }
            "sequence" | "sequence-of-mappings" => sequence(rows, said.at)?,
            _ => scalar(said)?,
        };
        if out.insert(said.key.clone(), value).is_some() {
            return Err(format!("the key `{}` of node {id} is said twice", said.key));
        }
    }
    Ok(Document::Table(out))
}

/// The elements of the sequence `owner` holds, in the order of their ordinals.
fn sequence(rows: &Rows, owner: u32) -> Result<Document, String> {
    let mut list = rows.elements.get(&owner).cloned().unwrap_or_default();
    list.sort_by_key(|each| each.ordinal);
    let mut out = Vec::new();
    for (place, said) in list.iter().enumerate() {
        if usize::try_from(said.ordinal).ok() != Some(place) {
            return Err(format!(
                "the element at node {} has ordinal {}, not {place}",
                said.at, said.ordinal
            ));
        }
        out.push(match said.kind.as_str() {
            "mapping" | "inline-mapping" => table(rows, said.at)?,
            "sequence" => sequence(rows, said.at)?,
            _ => scalar(said)?,
        });
    }
    Ok(Document::Array(out))
}

/// Compares the document the rows of `text` say with the one `toml` reads; what differs, one sentence; nothing when they agree, or when
/// the library reads no document.
pub(crate) fn oracle(text: &str, rows: &OracleRows) -> Vec<String> {
    let Ok(wanted) = toml::from_str::<toml::Table>(text) else {
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
    match table(&said, 0) {
        Ok(document) if document == compared(toml::Value::Table(wanted)) => Vec::new(),
        Ok(_) => vec!["the document the rows say is not the one toml reads".to_owned()],
        Err(why) => vec![why],
    }
}

#[cfg(test)]
#[path = "oracle_test.rs"]
mod tests;
