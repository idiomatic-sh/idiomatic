//! The services of the `shapes` part: a shape term's codes found once a worker, and a file's tree read for them once a
//! file. Every element is a row of the node relations with its kind, its field and its parent as the node table holds
//! them (`stages::nodes`): a root is its own parent, an embedded root's parent is its host. A shape is read off the lists
//! by kind: of the element's kind when the shape names it, else of the parent's kind with each one's children (and the
//! embedded roots, which are no host's children).

use super::types::{ShapeCodes, ShapeFile, ShapeWants};
use crate::domain::code::types::WordSlot;
use ryo_language::domain::node_table::service::{
    children_of, embedded_segments, field_of, ids_of_kind, kind_of, parent_of,
};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_relations::domain::relations::types::{KindBits, Position};

/// The place of a shape term among the shapes' terms, its codes found the first time it is met.
pub(crate) fn term_wants(
    wants: &mut ShapeWants,
    key: (WordSlot, WordSlot, WordSlot),
    codes: impl FnOnce() -> ShapeCodes,
) -> u32 {
    if let Some(place) = wants.terms.get(&key) {
        return *place;
    }
    let place = u32::try_from(wants.wants.len()).unwrap_or(u32::MAX);
    wants.wants.push(codes());
    let _old = wants.terms.insert(key, place);
    place
}

/// Whether a part of a shape admits a code: any when the shape does not name the part.
fn admits<T: PartialEq>(part: Option<&[T]>, code: &T) -> bool {
    part.is_none_or(|codes| codes.contains(code))
}

/// Whether an element and its parent hold the shape.
fn element_holds(nodes: &NodeTable, shape: &ShapeCodes, element: Position) -> bool {
    admits(shape.field.as_deref(), &field_of(nodes, element))
        && admits(shape.child.as_deref(), &kind_of(nodes, element))
        && admits(shape.parent.as_deref(), &kind_of(nodes, parent_of(nodes, element)))
}

/// How many elements a list of kinds holds in the file.
fn elements_of(nodes: &NodeTable, kinds: &[KindBits]) -> usize {
    kinds.iter().map(|kind| ids_of_kind(nodes, *kind).len()).sum()
}

/// Whether the file's tree holds the shape: read off the list of the element's kind, or of the parent's kind with each
/// one's children (and the embedded roots, which are no host's children), whichever holds fewer elements. A shape that
/// names neither kind holds.
pub(crate) fn shape_holds(nodes: &NodeTable, shape: &ShapeCodes) -> bool {
    let by_child = match (shape.child.as_deref(), shape.parent.as_deref()) {
        (Some(child), Some(parent)) => elements_of(nodes, child) <= elements_of(nodes, parent),
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => return true,
    };
    if by_child {
        let kinds = shape.child.as_deref().unwrap_or_default();
        return kinds
            .iter()
            .any(|kind| ids_of_kind(nodes, *kind).iter().any(|element| element_holds(nodes, shape, *element)));
    }
    let kinds = shape.parent.as_deref().unwrap_or_default();
    let under = kinds.iter().any(|kind| {
        ids_of_kind(nodes, *kind).iter().any(|parent| {
            // A root is its own parent and is no child of its own.
            (parent_of(nodes, *parent) == *parent && element_holds(nodes, shape, *parent))
                || children_of(nodes, *parent).iter().any(|child| element_holds(nodes, shape, *child))
        })
    });
    under || embedded_segments(nodes).any(|segment| element_holds(nodes, shape, segment.root))
}

/// Whether the file's tree holds the shape of the term at `place`; the tree is read for the shapes not yet answered in
/// this file, all of them at once.
pub(crate) fn held(wants: &ShapeWants, nodes: &NodeTable, file: &mut ShapeFile, place: u32) -> bool {
    if file.known < wants.wants.len() {
        file_read(wants, nodes, file);
    }
    let at = usize::try_from(place).unwrap_or(usize::MAX);
    file.held.get(at / 64).is_none_or(|word| (word >> (at % 64)) & 1 == 1)
}

/// Reads the file's tree for the shapes not yet answered.
pub(crate) fn file_read(wants: &ShapeWants, nodes: &NodeTable, file: &mut ShapeFile) {
    if file.known == 0 {
        file.held.clear();
    }
    let count = wants.wants.len();
    file.held.resize(count.div_ceil(64), 0);
    for (at, shape) in wants.wants.iter().enumerate().skip(file.known) {
        if shape_holds(nodes, shape)
            && let Some(word) = file.held.get_mut(at / 64)
        {
            *word |= 1 << (at % 64);
        }
    }
    file.known = count;
}
