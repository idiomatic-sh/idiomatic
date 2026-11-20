//! The service of the `walks` part: the words of `Children`, `Axis` and `Within`, each with the flag of the node rows its
//! relation covers, from the one function every node-source word takes it from.

use crate::domain::code::small_codes::service::direction_code;
use crate::domain::code::types::CodeWord;
use crate::domain::code::walking::service::{axis, children, children_of_field, within};
use crate::domain::code::walking::types::{
    AxisWord, ChildrenOfFieldWord, ChildrenWord, WithinWord,
};
use crate::domain::encoding::matches::service::extent_flag;
use crate::domain::steps::types::{AxisDirection, Register, StepRows};
use ryo_language::domain::node_table::types::FieldCode;
use ryo_relations::domain::relations::types::Relation;

/// The word of a `Children`: every child, or the children of one field.
pub(crate) fn children_word(
    relation: Relation,
    parent: Register,
    field: Option<FieldCode>,
    out: Register,
) -> CodeWord {
    let flag = extent_flag(StepRows::Node(relation));
    match field {
        Some(field) => children_of_field(ChildrenOfFieldWord {
            flag,
            field,
            parent,
            out,
        }),
        None => children(ChildrenWord { flag, parent, out }),
    }
}

/// The word of an `Axis`.
pub(crate) fn axis_word(
    relation: Relation,
    direction: AxisDirection,
    start: Register,
    out: Register,
) -> CodeWord {
    axis(AxisWord {
        flag: extent_flag(StepRows::Node(relation)),
        direction: direction_code(direction),
        start,
        out,
    })
}

/// The word of a `Within`.
pub(crate) fn within_word(
    relation: Relation,
    ancestor: Register,
    descendant: Register,
) -> CodeWord {
    within(WithinWord {
        flag: extent_flag(StepRows::Node(relation)),
        ancestor,
        descendant,
    })
}
