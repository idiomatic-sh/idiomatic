//! The `shapes` part's cases: a shape's words read back and decided over a small tree (a module holding a call whose
//! `func` child is a name): a star whose neighbourhood an element has holds, one whose child is in another field is
//! empty, a kind no element has is empty, of two stars the one with fewer elements is entered, and an element is held
//! to the kinds of its ancestors (a token by its parent node).

use super::service::{shape_held, stars_of};
use super::types::{EnclosingColumns, ShapeCounts, ShapeWords};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::types::WordSlot;
use crate::domain::needs::types::NeedVerdict;
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, token};
use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
use ryo_relations::domain::relations::types::{Count, KindBits, Offset};

/// A shape's words, as `encoding::needs` writes them.
type Input = &'static [WordSlot];

/// The verdict, and the entry star's candidates and survivors.
type Value = (NeedVerdict, usize, usize);

/// The run's constants: the kind `call`, the kind `name`, the field `func`, a name no vocabulary holds, the kind `module`.
const CONSTANTS: [CellWord; 5] = [CellWord(102), CellWord(103), CellWord(201), CellWord(999), CellWord(101)];
/// Kind codes 0 to 3: none, `module`, `call`, `name`.
const KINDS: [CellWord; 4] = [CellWord(100), CellWord(101), CellWord(102), CellWord(103)];
/// Field codes 0 and 1: none, `func`.
const FIELDS: [CellWord; 2] = [CellWord(200), CellWord(201)];

/// `f(x)`: a module holding a call whose `func` child is a name token, and a second name token as its argument.
fn tree() -> NodeTable {
    let mut table = NodeTable::default();
    let mut writer = builder(&mut table, Count(4));
    enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
    enter_node(&mut writer, KindBits(2), FieldCode(0), Offset(0));
    token(&mut writer, KindBits(3), FieldCode(1), Offset(0), Offset(1));
    token(&mut writer, KindBits(3), FieldCode(0), Offset(2), Offset(3));
    leave_node(&mut writer, Offset(4));
    leave_node(&mut writer, Offset(4));
    let _done = finished(&mut writer);
    table
}

const ANY: WordSlot = WordSlot(0);
const CALL: WordSlot = WordSlot(0);
const NAME: WordSlot = WordSlot(1);
const FUNC: WordSlot = WordSlot(2);
const UNKNOWN: WordSlot = WordSlot(3);
const MODULE: WordSlot = WordSlot(4);
const ONE: WordSlot = WordSlot(1);
const TWO: WordSlot = WordSlot(2);

const CASES: &[Case<Input, Value, ()>] = &[
    Case {
        name: CaseName("a_call_with_a_name_in_its_func_field_holds"),
        word: Word::Succeeds,
        input: || &[ONE, CALL, NO_SLOT, ANY, ANY, ONE, NAME, FUNC, ANY, ANY],
        expect: Expect::Value((NeedVerdict::Holds, 1, 1)),
    },
    Case {
        name: CaseName("a_call_whose_name_child_is_in_another_field_is_empty"),
        word: Word::Succeeds,
        input: || &[ONE, CALL, FUNC, ANY, ANY, ONE, NAME, NO_SLOT, ANY, ANY],
        expect: Expect::Value((NeedVerdict::Empty, 1, 0)),
    },
    Case {
        name: CaseName("a_kind_no_element_has_is_empty"),
        word: Word::Succeeds,
        input: || &[ONE, UNKNOWN, NO_SLOT, ANY, ANY, ANY, ANY],
        expect: Expect::Value((NeedVerdict::Empty, 0, 0)),
    },
    Case {
        name: CaseName("of_two_stars_the_kind_with_fewer_elements_is_entered"),
        word: Word::Succeeds,
        input: || {
            &[
                TWO, NAME, FUNC, ANY, ONE, CALL, NO_SLOT, ANY, ANY, ANY, CALL, NO_SLOT, ANY, ANY, ONE, NAME, FUNC, ANY,
                ANY,
            ]
        },
        expect: Expect::Value((NeedVerdict::Holds, 1, 1)),
    },
    Case {
        name: CaseName("a_name_inside_a_module_and_a_call_holds_for_both_names"),
        word: Word::Succeeds,
        input: || &[ONE, NAME, NO_SLOT, ANY, ANY, ANY, TWO, MODULE, CALL],
        expect: Expect::Value((NeedVerdict::Holds, 2, 2)),
    },
    Case {
        name: CaseName("a_name_inside_a_name_is_empty_a_token_encloses_nothing"),
        word: Word::Succeeds,
        input: || &[ONE, NAME, NO_SLOT, ANY, ANY, ANY, ONE, NAME],
        expect: Expect::Value((NeedVerdict::Empty, 2, 0)),
    },
];

fn subject(words: Input) -> Result<Value, ()> {
    let nodes = tree();
    let vocabulary = ShapeWords {
        constants: &CONSTANTS,
        kinds: &KINDS,
        fields: &FIELDS,
    };
    let mut counts = ShapeCounts::default();
    let mut columns = EnclosingColumns::default();
    let verdict = shape_held(&stars_of(words), &nodes, &vocabulary, &mut counts, &mut columns);
    Ok((verdict, counts.candidates.0, counts.survivors.0))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
