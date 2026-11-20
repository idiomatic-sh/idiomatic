//! The `spec_facts` service: free functions over the interpolations of a walked file. An interpolation belongs to the f-string
//! that holds it, through any format spec it stands in; the interpolations of one f-string are numbered in the order they are
//! written, and one that stands in the format spec of another has that one for its parent.

use crate::domain::format_specification::types::{OwnedField, SpecOrdinal};
use crate::domain::labels::types::Field;
use crate::domain::node::service::field_code;
use crate::domain::node::types::SyntaxFile;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, kind_of};
use crate::domain::syntax::types::Kind;
use ryo::languages::{Position, node_table};

/// The f-string that holds the element at `at`: the nearest f-string above it that is no format spec; none outside one.
#[must_use]
pub(crate) fn owner(file: &SyntaxFile<'_>, at: Position) -> Option<Position> {
    let mut below = at;
    let mut above = node_table::parent_of(file.table, at);
    while above != below {
        if kind_of(node_table::kind_of(file.table, above)) == Kind::Node(Node::JoinedStr)
            && node_table::field_of(file.table, above) != field_code(Field::FormatSpec)
        {
            return Some(above);
        }
        below = above;
        above = node_table::parent_of(file.table, above);
    }
    None
}

/// Writes every interpolation of an f-string of `file` into `owned` with the f-string that holds it, the interpolations of one
/// f-string together and in the order they are written.
pub(crate) fn owned_fields(file: &SyntaxFile<'_>, owned: &mut Vec<OwnedField>) {
    owned.clear();
    for field in node_table::ids_of_kind(file.table, kind_bits(Kind::Node(Node::FormattedValue))) {
        if let Some(owner) = owner(file, *field) {
            owned.push(OwnedField {
                owner,
                field: *field,
            });
        }
    }
    owned.sort_unstable();
}

/// The place, among the interpolations `group` of one f-string, of the interpolation whose format spec holds `field`; none for
/// an interpolation that stands in the f-string itself.
#[must_use]
pub(crate) fn parent_place(
    file: &SyntaxFile<'_>,
    group: &[OwnedField],
    field: OwnedField,
) -> Option<SpecOrdinal> {
    let mut above = node_table::parent_of(file.table, field.field);
    loop {
        if above == field.owner {
            return None;
        }
        if let Some(found) = group.iter().position(|each| each.field == above) {
            return Some(SpecOrdinal(found));
        }
        let next = node_table::parent_of(file.table, above);
        if next == above {
            return None;
        }
        above = next;
    }
}
