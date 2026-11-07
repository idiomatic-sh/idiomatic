//! The independent parse5 DOM dump compared with the complete emitted syntax tree and edges.
//! This never reparses with the production library or compares a second production projection.
use ryo::languages::{
    Relation,
    contract::{Cell, OracleRows},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// The pinned parse5 8.0.0 dumps of every fixture source, keyed by the source text.
const STANDARD: &str = include_str!("../../../../../../tests/fixtures/standard/parse5.json");

/// The node of a node cell; none for any other.
fn node_id(cell: Option<&Cell>) -> Option<u32> {
    if let Some(Cell::Node(value)) = cell {
        Some(*value)
    } else {
        None
    }
}

/// The text of a name cell; none for any other.
fn name(cell: Option<&Cell>) -> Option<&str> {
    if let Some(Cell::Name(value)) = cell {
        Some(value)
    } else {
        None
    }
}

/// The numbers the dump writes each DOM scalar property under, by the kind of the node the rows write it as.
const PROPERTIES: [(&str, u64); 7] = [
    ("name", 1),
    ("namespace", 2),
    ("prefix", 3),
    ("value", 4),
    ("public-id", 5),
    ("system-id", 6),
    ("document-mode", 7),
];

/// The number of the property a node of `kind` stands for; none for a node of the DOM itself.
fn property_number(kind: &str) -> Option<u64> {
    PROPERTIES
        .iter()
        .find(|(name, _)| *name == kind)
        .map(|(_, number)| *number)
}

/// The children of a node of the DOM whose published value is `text`, with the nodes of its properties taken out: an error unless
/// they say exactly its properties, each once, a property the DOM leaves empty left out unless it is a value.
fn without_properties(text: &str, descendants: Vec<Value>) -> Result<Vec<Value>, String> {
    let published: Vec<(u64, String)> = if text.is_empty() {
        Vec::new()
    } else {
        serde_json::from_str::<Vec<(u64, String)>>(text).map_err(|error| error.to_string())?
    };
    let wanted: Vec<(u64, &str)> = published
        .iter()
        .filter(|(number, value)| !value.is_empty() || *number == 4)
        .map(|(number, value)| (*number, value.as_str()))
        .collect();
    let mut found = Vec::new();
    let mut kept = Vec::new();
    for child in descendants {
        let kind = child.get(0).and_then(Value::as_str).unwrap_or_default();
        match property_number(kind) {
            Some(number) => {
                if child
                    .get(2)
                    .and_then(Value::as_array)
                    .is_none_or(|nested| !nested.is_empty())
                {
                    return Err(format!("the property node {kind} holds nodes"));
                }
                found.push((
                    number,
                    child
                        .get(1)
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                ));
            }
            None => kept.push(child),
        }
    }
    let found: Vec<(u64, &str)> = found
        .iter()
        .map(|(number, value)| (*number, value.as_str()))
        .collect();
    if found == wanted {
        Ok(kept)
    } else {
        Err(format!(
            "the property nodes {found:?} do not say the node's properties {wanted:?}"
        ))
    }
}

/// The tree under `id` as the rows say it, `[kind, text, children]`, the children in the order of their ordinals; an error
/// for an edge to no node, or for a cycle, which `remaining` bounds.
fn subtree(
    id: u32,
    nodes: &BTreeMap<u32, (&str, &str)>,
    children: &BTreeMap<u32, Vec<(i64, u32)>>,
    remaining: usize,
) -> Result<Value, String> {
    if remaining == 0 {
        return Err("syntax_child contains a cycle".to_owned());
    }
    let (kind, text) = nodes
        .get(&id)
        .ok_or_else(|| format!("edge points at missing node {id}"))?;
    let descendants = children
        .get(&id)
        .into_iter()
        .flatten()
        .map(|(_, child)| subtree(*child, nodes, children, remaining - 1))
        .collect::<Result<Vec<_>, _>>()?;
    let descendants = if matches!(
        *kind,
        "document" | "document-type" | "element" | "attribute" | "text" | "comment"
    ) {
        without_properties(text, descendants)?
    } else {
        descendants
    };
    Ok(json!([kind, text, descendants]))
}

/// The whole tree the `syntax_tree` and `syntax_child` rows say, each node under its one parent; an error when a row is
/// incomplete or the edges do not make one tree.
fn emitted(rows: &OracleRows) -> Result<Value, String> {
    let mut nodes = BTreeMap::new();
    for row in rows.of(Relation::SyntaxTree) {
        let id = node_id(row.first()).ok_or("syntax_tree has no id")?;
        let kind = name(row.get(1)).ok_or("syntax_tree has no kind")?;
        let text = name(row.get(2)).ok_or("syntax_tree has no text")?;
        if nodes.insert(id, (kind, text)).is_some() {
            return Err(format!("duplicate node {id}"));
        }
    }
    let mut children: BTreeMap<u32, Vec<(i64, u32)>> = BTreeMap::new();
    let mut owners = BTreeMap::new();
    for row in rows.of(Relation::SyntaxChild) {
        let parent = node_id(row.first()).ok_or("syntax_child has no parent")?;
        let child = node_id(row.get(1)).ok_or("syntax_child has no child")?;
        let Some(Cell::Number(ordinal)) = row.get(2) else {
            return Err("syntax_child has no ordinal".to_owned());
        };
        if !nodes.contains_key(&parent) || owners.insert(child, parent).is_some() {
            return Err("invalid or duplicate parent edge".to_owned());
        }
        children.entry(parent).or_default().push((ordinal.0, child));
    }
    if owners.len() + 1 != nodes.len() || owners.contains_key(&0) {
        return Err("syntax_child omits or adds DOM nodes".to_owned());
    }
    for values in children.values_mut() {
        values.sort_unstable();
    }
    subtree(0, &nodes, &children, nodes.len())
}
/// Compare every emitted node, attribute, namespace, decoded value, template subtree and edge; the nodes of the properties are held to
/// the properties their node publishes, and then left out, as the dump has none.
#[must_use]
pub(crate) fn oracle(text: &str, rows: &OracleRows) -> Vec<String> {
    let standard: Value = match serde_json::from_str(STANDARD) {
        Ok(value) => value,
        Err(error) => return vec![error.to_string()],
    };
    let Some(expected) = standard.get(text) else {
        return vec!["source has no pinned parse5 8.0.0 whole-tree dump".to_owned()];
    };
    against(rows, expected)
}

/// Compare actual emitted rows with an independently supplied whole-tree expectation.
pub(crate) fn against(rows: &OracleRows, expected: &Value) -> Vec<String> {
    match emitted(rows).and_then(|actual| normalize(&actual)) {
        Ok(actual) if normalize(expected).as_ref() == Ok(&actual) => Vec::new(),
        Ok(actual) => vec![format!(
            "DOM differs from parse5 8.0.0: expected {expected}; emitted {actual}"
        )],
        Err(error) => vec![error],
    }
}

/// DOM location conventions differ between implementations. Compare every DOM node and property,
/// and all source bytes represented by the lexical wrapper, without pretending their token ranges agree.
fn normalize(tree: &Value) -> Result<Value, String> {
    let children = tree
        .get(2)
        .and_then(Value::as_array)
        .ok_or("missing source wrapper")?;
    let document = children
        .first()
        .filter(|node| node.get(0).and_then(Value::as_str) == Some("document"))
        .ok_or("missing native document")?;
    let mut source = String::new();
    for leaf in children.iter().skip(1) {
        let kind = leaf
            .get(0)
            .and_then(Value::as_str)
            .ok_or("missing token kind")?;
        if !matches!(
            kind,
            "doctype-token" | "start-tag" | "end-tag" | "comment-token" | "characters" | "error"
        ) || leaf
            .get(2)
            .and_then(Value::as_array)
            .is_none_or(|nested| !nested.is_empty())
        {
            return Err("non-lexical node beside native document".to_owned());
        }
        source.push_str(
            leaf.get(1)
                .and_then(Value::as_str)
                .ok_or("missing source spelling")?,
        );
    }
    Ok(json!([tree.get(0), tree.get(1), document, source]))
}
