//! What a contract checks of the rows and trees a language wrote: each check answers whether they are well formed.

use super::collect::Collect;
use crate::domain::elements::service::{end_of, positions};
use crate::domain::elements::types::Elements;
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Relation};

/// Without embedded expressions, projection must not add, omit, or alter native syntax.
pub(super) fn projection_preserves_native(out: &Collect) -> bool {
    [
        (Relation::SyntaxTree, Relation::SemanticSyntax),
        (Relation::SyntaxChild, Relation::SemanticChild),
    ]
    .into_iter()
    .all(|(native, semantic)| {
        let mut original: Vec<_> = out
            .rows()
            .into_iter()
            .filter_map(|(relation, row)| (relation == native).then_some(row))
            .collect();
        let mut projected: Vec<_> = out
            .rows()
            .into_iter()
            .filter_map(|(relation, row)| (relation == semantic).then_some(row))
            .collect();
        original.sort();
        projected.sort();
        original == projected
    })
}

/// With embedded expressions or macro expansions, projection keeps every native row and edge, and adds only the trees read again:
/// every semantic row that is not a native one is of an element the language added (`added` answers for its node id), a syntax row
/// by its `id` and an edge by its `child`.
pub(super) fn projection_extends_native(out: &Collect, added: impl Fn(u64) -> bool) -> bool {
    [
        (Relation::SyntaxTree, Relation::SemanticSyntax, 0),
        (Relation::SyntaxChild, Relation::SemanticChild, 1),
    ]
    .into_iter()
    .all(|(native, semantic, element)| {
        let rows_of = |wanted| -> std::collections::BTreeSet<_> {
            out.rows()
                .into_iter()
                .filter_map(|(relation, row)| (relation == wanted).then_some(row))
                .collect()
        };
        let (native, projected) = (rows_of(native), rows_of(semantic));
        native.is_subset(&projected)
            && projected
                .difference(&native)
                .all(|row| row.get(element).is_some_and(|id| added(*id)))
    })
}

/// Compare the consumer-visible result of a field equality predicate, not only row counts.
pub(super) fn filtered_projection_valid(out: &Collect, field: u64) -> bool {
    let select = |wanted| {
        let mut selected: Vec<_> = out
            .rows()
            .into_iter()
            .filter_map(|(relation, row)| {
                (relation == wanted && row.get(3) == Some(&field)).then_some(row)
            })
            .collect();
        selected.sort();
        selected
    };
    select(Relation::SyntaxChild) == select(Relation::SemanticChild)
}

/// Logical token lengths cannot replace physical source offsets, even after decoding.
pub(super) fn semantic_spans_valid(
    out: &Collect,
    length: usize,
    coordinates: bool,
    implied: impl Fn(u64) -> bool,
) -> bool {
    let rows = out.rows();
    rows.iter()
        .filter(|(relation, _)| *relation == Relation::SemanticSyntax)
        .all(|(_, row)| {
            let [id, _, _, _, span] = row.as_slice() else {
                return false;
            };
            let start = span >> 32_u32;
            let end = span & u64::from(u32::MAX);
            if start == u64::from(u32::MAX) && end == 0 {
                return implied(*id)
                    && !rows.iter().any(|(relation, coordinate)| {
                        *relation == Relation::SourceLocation && coordinate.first() == Some(id)
                    });
            }
            start <= end
                && end <= u64::try_from(length).unwrap_or(u64::MAX)
                && (!coordinates
                    || rows.iter().any(|(relation, coordinate)| {
                        *relation == Relation::SourceLocation
                            && coordinate.first() == Some(id)
                            && coordinate.get(2) == Some(span)
                    }))
        })
}

/// A normalized identifier remains a known spelling and an actual, nonempty source occurrence.
pub(super) fn identifier_valid(row: &[u64], symbols: &Symbols, source: ByteSlice<'_>) -> bool {
    let [_, name, _, span] = row else {
        return false;
    };
    let Ok(name) = u32::try_from(*name) else {
        return false;
    };
    let start = span >> 32_u32;
    let end = span & u64::from(u32::MAX);
    crate::domain::symbols::service::text_of(
        symbols,
        source,
        ryo_relations::domain::terms::types::Symbol(name),
    )
    .is_some_and(|name| !name.0.is_empty())
        && start < end
        && end <= u64::try_from(source.0.len()).unwrap_or(u64::MAX)
}

/// Every provider must distinguish a proven lexical path from an unknown value and explicit builtin availability.
pub(super) fn qualified_name_valid(row: &[u64], symbols: &Symbols, source: ByteSlice<'_>) -> bool {
    use crate::domain::symbols::service::text_of;
    use ryo_relations::domain::terms::{service::unknown_reason, types::Symbol};
    let [_, path, provenance, major, minor, _, _] = row else {
        return false;
    };
    let (Ok(path), Ok(provenance)) = (u32::try_from(*path), u32::try_from(*provenance)) else {
        return false;
    };
    let path = Symbol(path);
    let named = text_of(symbols, source, path).is_some_and(|name| !name.0.is_empty());
    match text_of(symbols, source, Symbol(provenance)).map(|text| text.0) {
        Some("builtin") => {
            named && *major > 0 && i64::try_from(*major).is_ok() && i64::try_from(*minor).is_ok()
        }
        Some("import") => named && *major == 0 && *minor == 0,
        Some("local" | "unresolved" | "ambiguous") => {
            unknown_reason(path).is_some() && *major == 0 && *minor == 0
        }
        _ => false,
    }
}

/// Whether `elements` is a pre-order tree: every element's subtree lies inside the table and
/// inside its parent's, and the first element is its own parent.
pub(super) fn well_formed(elements: &Elements) -> bool {
    let count = end_of(elements);
    positions(elements).all(|position| {
        let end = crate::domain::elements::service::last_at(elements, position);
        let parent = crate::domain::elements::service::parent_at(elements, position);
        end < count
            && (parent == position
                || (parent < position
                    && end <= crate::domain::elements::service::last_at(elements, parent)))
    })
}

/// Unknown or cross-file class sentinels cannot silently compare equal as structural identities.
pub(super) fn shape_representatives_valid(out: &Collect) -> bool {
    use ryo_relations::domain::relations::{service::node_file, types::NodeId};
    let representatives: std::collections::BTreeSet<_> = out
        .rows()
        .into_iter()
        .filter_map(|(relation, row)| {
            if relation != Relation::ExpressionShape {
                return None;
            }
            let [id, shape, file, _] = row.as_slice() else {
                return None;
            };
            (*id == *shape).then_some((*file, *shape))
        })
        .collect();
    out.rows()
        .into_iter()
        .filter(|(relation, _)| *relation == Relation::ExpressionShape)
        .all(|(_, row)| {
            let [id, shape, file, _] = row.as_slice() else {
                return false;
            };
            let Ok(file) = u32::try_from(*file) else {
                return false;
            };
            node_file(NodeId(*id)) == FileId(file)
                && node_file(NodeId(*shape)) == FileId(file)
                && representatives.contains(&(u64::from(file), *shape))
        })
}

/// Token trivia cannot masquerade as the root of an independently parsed expression.
pub(super) fn embedded_roots_valid(elements: &Elements) -> bool {
    use crate::domain::elements::service::{flags_at, parent_at};
    crate::domain::expansions::service::embedded_expressions(elements).all(|embedded| {
        embedded.root < embedded.end
            && flags_at(elements, embedded.root).0
                & ryo_relations::domain::relations::constants::TOKEN.0
                == 0
            && parent_at(elements, embedded.root) == embedded.host
    })
}

/// Whether the control-flow rows of a file read as file 0 hold together: every row is of file 0; the blocks are numbered from
/// zero in the order written, with flags that are 0 or 1; every edge, step and member names a block the file has; every step and
/// member names an element `node` accepts; and every edge and step kind is a word of its relation, as `word` reads the cell.
pub(super) fn flow_valid(
    rows: &[(Relation, Vec<u64>)],
    node: impl Fn(u64) -> bool,
    word: impl Fn(Relation, u64) -> bool,
) -> bool {
    let mut blocks = 0_u64;
    for (relation, row) in rows {
        if *relation != Relation::FlowBlock {
            continue;
        }
        let [block, reachable, live, implicit_return, file] = row.as_slice() else {
            return false;
        };
        if *block != blocks || *reachable > 1 || *live > 1 || *implicit_return > 1 || *file != 0 {
            return false;
        }
        blocks += 1;
    }
    rows.iter().all(|(relation, row)| {
        if *relation == Relation::FlowEdge {
            return matches!(row.as_slice(), [source, target, kind, 0]
                if *source < blocks && *target < blocks && word(*relation, *kind));
        }
        if *relation == Relation::FlowStep {
            return matches!(row.as_slice(), [block, _, kind, at, 0]
                if *block < blocks && word(*relation, *kind) && node(*at));
        }
        if *relation == Relation::FlowMember {
            return matches!(row.as_slice(), [at, block, 0] if *block < blocks && node(*at));
        }
        true
    })
}

/// Whether an empty file's rows of the flow relation `relation` are its module's own graph: one member, the module's root element
/// `root`; at least one block; and edges only out of the block the root is in.
pub(super) fn empty_module_flow(
    rows: &[(Relation, Vec<u64>)],
    relation: Relation,
    root: u64,
) -> bool {
    let members: Vec<&[u64]> = rows
        .iter()
        .filter(|(each, _)| *each == Relation::FlowMember)
        .map(|(_, row)| row.as_slice())
        .collect();
    let [[node, entry, _]] = members.as_slice() else {
        return false;
    };
    if *node != root {
        return false;
    }
    let mut kept = rows.iter().filter(|(each, _)| *each == relation);
    if relation == Relation::FlowBlock {
        return kept.next().is_some();
    }
    if relation == Relation::FlowEdge {
        return kept.all(|(_, row)| row.first() == Some(entry));
    }
    relation == Relation::FlowMember
}

#[cfg(test)]
#[path = "validators_test.rs"]
mod tests;
