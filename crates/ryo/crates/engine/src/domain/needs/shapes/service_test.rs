//! The `shapes` part's cases: a file's tree read once for the shapes the shape terms ask, by the element's kind or, when
//! the shape names none, by the parent's; a root, its own parent, read as one; a kind the vocabulary does not name held by
//! no element; and a shape met after the file's first read answered by a second.

use super::service::{held, term_wants};
use super::types::{ShapeCodes, ShapeFile, ShapeWants};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::WordSlot;
use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, token};
use ryo_language::domain::node_table::types::{FieldCode, NodeTable, NodeTableBuilder};
use ryo_relations::domain::relations::types::{Count, KindBits, Offset};

/// A shape as codes: the parent's kind, the field, the element's kind; none: any; an empty list: a name the vocabulary
/// does not have.
type Shape = (Option<&'static [u16]>, Option<&'static [u16]>, Option<&'static [u16]>);

/// Two shapes asked of the file one after the other.
type Input = [Shape; 2];

/// Whether each shape holds in the file.
type Value = [bool; 2];

const CASES: &[Case<Input, Value, ()>] = &[
    Case {
        name: CaseName("a_child_under_its_parent_in_its_field_holds"),
        word: Word::Succeeds,
        input: || [(Some(&[2]), Some(&[5]), Some(&[3])), (None, Some(&[5]), Some(&[3]))],
        expect: Expect::Value([true, true]),
    },
    Case {
        name: CaseName("a_child_in_another_field_or_under_another_kind_is_empty"),
        word: Word::Succeeds,
        input: || [(Some(&[2]), Some(&[4]), Some(&[3])), (Some(&[1]), None, Some(&[3]))],
        expect: Expect::Value([false, false]),
    },
    Case {
        name: CaseName("a_shape_with_no_child_kind_is_read_by_the_parent_kind"),
        word: Word::Succeeds,
        input: || [(Some(&[2]), Some(&[6]), None), (Some(&[2]), Some(&[4]), None)],
        expect: Expect::Value([true, false]),
    },
    Case {
        name: CaseName("the_root_is_its_own_parent"),
        word: Word::Succeeds,
        input: || [(Some(&[1]), Some(&[0]), None), (Some(&[1]), Some(&[0]), Some(&[1]))],
        expect: Expect::Value([true, true]),
    },
    Case {
        name: CaseName("a_kind_the_vocabulary_does_not_name_is_held_by_no_element"),
        word: Word::Succeeds,
        input: || [(Some(&[]), Some(&[5]), None), (None, Some(&[5]), Some(&[]))],
        expect: Expect::Value([false, false]),
    },
];

/// A module (kind 1), holding a loop (kind 2) in field 4, which holds a name (kind 3) in field 5 and a token (kind 7) in
/// field 6.
fn tree(writer: &mut NodeTableBuilder<'_>) {
    enter_node(writer, KindBits(1), FieldCode(0), Offset(0));
    enter_node(writer, KindBits(2), FieldCode(4), Offset(0));
    enter_node(writer, KindBits(3), FieldCode(5), Offset(4));
    leave_node(writer, Offset(5));
    token(writer, KindBits(7), FieldCode(6), Offset(5), Offset(6));
    leave_node(writer, Offset(6));
    leave_node(writer, Offset(6));
}

fn codes((parent, field, child): Shape) -> ShapeCodes {
    let kinds = |codes: &[u16]| codes.iter().copied().map(KindBits).collect();
    ShapeCodes {
        parent: parent.map(kinds),
        field: field.map(|codes| codes.iter().copied().map(FieldCode).collect()),
        child: child.map(kinds),
    }
}

fn subject(asked: Input) -> Result<Value, ()> {
    let mut table = NodeTable::default();
    let mut writer = builder(&mut table, Count(8));
    tree(&mut writer);
    let nodes = finished(&mut writer);
    let (mut wants, mut file) = (ShapeWants::default(), ShapeFile::default());
    let first = term_wants(&mut wants, (WordSlot(0), WordSlot(0), WordSlot(0)), || codes(asked[0]));
    // The tree is read for the first term's shape before the second term is met.
    let first_held = held(&wants, nodes, &mut file, first);
    let second = term_wants(&mut wants, (WordSlot(1), WordSlot(0), WordSlot(0)), || codes(asked[1]));
    Ok([first_held, held(&wants, nodes, &mut file, second)])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
