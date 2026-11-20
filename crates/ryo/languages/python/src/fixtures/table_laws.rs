//! The laws of the engine's node table, checked on a table the language's walk wrote: the id space (nodes in pre-order, then tokens
//! in the order of the file), the subtree ends, the children lists, the kind buckets and the kinds present, and that the tokens
//! read back the whole file.

use ryo::languages::{Count, KindBits, NodeTable, Offset, Position, node_table};

/// The id of an element by its index.
fn id(index: usize) -> Position {
    Position(u32::try_from(index).unwrap_or(u32::MAX))
}

/// Says a law is broken at an element.
fn broken(found: &mut Vec<String>, law: &str, at: Position) {
    found.push(format!("{law} at {}", at.0));
}

/// The laws of the nodes: each opens after its parent, inside its parent's span, and its subtree ends after it and within its
/// parent's.
fn node_laws(table: &NodeTable, found: &mut Vec<String>) {
    let nodes = node_table::node_count(table).0;
    for index in 0..nodes {
        let at = id(index);
        let parent = node_table::parent_of(table, at);
        let end = node_table::subtree_end_of(table, at);
        if index == 0 {
            if parent != at {
                broken(found, "the root is not its own parent", at);
            }
        } else if parent >= at {
            broken(found, "a node opens before its parent", at);
        } else if node_table::subtree_end_of(table, parent) < end {
            broken(found, "a subtree ends after its parent's", at);
        } else {
            let (own, around) = (
                node_table::span_of(table, at),
                node_table::span_of(table, parent),
            );
            if own.start < around.start || own.end > around.end {
                broken(found, "a node spans outside its parent", at);
            }
        }
        if end <= at || usize::from(end) > nodes {
            broken(
                found,
                "a subtree does not end after its node, among the nodes",
                at,
            );
        }
        // Pre-order: the next node is the node's first child or lies after its subtree.
        let next = id(index + 1);
        if index + 1 < nodes && next < end && node_table::parent_of(table, next) != at {
            broken(
                found,
                "the node after one with children is not its child",
                at,
            );
        }
    }
}

/// The laws of the tokens: they follow the nodes in the order of the file, each under a node, and read back the whole file.
fn token_laws(table: &NodeTable, length: Offset, found: &mut Vec<String>) {
    let nodes = node_table::node_count(table).0;
    let count = node_table::element_count(table).0;
    let mut reached = Offset(0);
    for index in nodes..count {
        let at = id(index);
        let span = node_table::span_of(table, at);
        if usize::from(node_table::parent_of(table, at)) >= nodes {
            broken(found, "a token is under no node", at);
        }
        if node_table::subtree_end_of(table, at) != id(index + 1) {
            broken(found, "a token has a subtree", at);
        }
        if span.start != reached || span.end < span.start {
            broken(found, "the tokens leave a gap or go back", at);
        }
        reached = span.end;
    }
    if count > nodes && reached != length {
        broken(
            found,
            "the tokens stop before the end of the file",
            id(count),
        );
    }
}

/// The laws of the children lists: every element but the root is once among its parent's children, at its ordinal, and the
/// children stand in the order of the file.
fn children_laws(table: &NodeTable, found: &mut Vec<String>) {
    let count = node_table::element_count(table).0;
    let mut listed = 0_usize;
    for index in 0..count {
        let at = id(index);
        let mut before = Offset(0);
        for (place, child) in node_table::children_of(table, at).iter().enumerate() {
            listed += 1;
            if node_table::parent_of(table, *child) != at {
                broken(found, "a child is listed under another parent", *child);
            }
            if usize::try_from(node_table::ordinal_of(table, *child).0).ok() != Some(place) {
                broken(found, "a child is not at its ordinal", *child);
            }
            let start = node_table::span_of(table, *child).start;
            if start < before {
                broken(
                    found,
                    "the children are not in the order of the file",
                    *child,
                );
            }
            before = start;
        }
    }
    if count > 0 && listed + 1 != count {
        found.push(format!(
            "{listed} elements are listed as children, of {count} elements"
        ));
    }
}

/// The laws of the kind buckets: each kind's ids rise, hold that kind, and are present exactly when there is one.
fn kind_laws(table: &NodeTable, kinds: Count, found: &mut Vec<String>) {
    let count = node_table::element_count(table).0;
    let present = node_table::kinds_present(table);
    let mut bucketed = 0_usize;
    for kind in 0..kinds.0 {
        let bits = KindBits(u16::try_from(kind).unwrap_or(u16::MAX));
        let ids = node_table::ids_of_kind(table, bits);
        bucketed += ids.len();
        if ids.windows(2).any(|pair| pair.first() >= pair.get(1)) {
            found.push(format!("the ids of kind {kind} do not rise"));
        }
        if ids
            .iter()
            .any(|each| node_table::kind_of(table, *each) != bits)
        {
            found.push(format!("the bucket of kind {kind} holds another kind"));
        }
        let bit = present
            .0
            .get(kind / 64)
            .is_some_and(|word| word.0 >> (kind % 64) & 1 == 1);
        if bit == ids.is_empty() {
            found.push(format!(
                "kind {kind} is present and has no id, or the reverse"
            ));
        }
    }
    if bucketed != count {
        found.push(format!(
            "{bucketed} elements are in kind buckets, of {count}"
        ));
    }
}

/// Every law of the node table the table of a walked file of `length` bytes breaks, in a language of `kinds` kinds; none when it
/// keeps them all.
#[must_use]
pub fn broken_laws(table: &NodeTable, kinds: Count, length: Offset) -> Vec<String> {
    let mut found = Vec::new();
    node_laws(table, &mut found);
    token_laws(table, length, &mut found);
    children_laws(table, &mut found);
    kind_laws(table, kinds, &mut found);
    found
}
