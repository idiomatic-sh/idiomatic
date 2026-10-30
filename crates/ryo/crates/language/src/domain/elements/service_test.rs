//! The `elements` service's cases, on values alone.

use crate::domain::symbols::types::ByteSlice;

use super::service::{
    child_of_kind, children, decoded_at, decoded_token, end_at, end_of, enter_node,
    first_token_text, kind_at, leave_node, number_mut, positions, start_at, table_builder, text_at,
    token,
};
use super::types::Elements;
use crate::domain::relations::types::{KindBits, Offset, Position};
use crate::domain::symbols::types::Symbols;
use crate::domain::terms::types::NameText;

use crate::cases::{Case, CaseName, Expect, Word, run};

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

fn a_node_with_a_token_is_three_positions_with_its_root() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let source = ByteSlice(b"fn");
    let mut builder = table_builder(&mut elements, &mut symbols, source);
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(2));
    leave_node(&mut builder, Offset(2));
    let all = positions(&elements).count();
    let children = children(&elements, Position(0)).count();
    usize::from(
        end_of(&elements) == Position(2)
            && all == 2
            && children == 1
            && start_at(&elements, Position(1)) == Offset(0)
            && end_at(&elements, Position(1)) == Offset(2)
            && kind_at(&elements, Position(1)) == KindBits(2),
    )
}

fn a_child_is_found_by_its_kind_and_a_node_holds_the_text_of_its_first_token() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"fn"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(2));
    leave_node(&mut builder, Offset(2));
    usize::from(
        child_of_kind(&elements, Position(0), KindBits(2)) == Some(Position(1))
            && child_of_kind(&elements, Position(0), KindBits(9)).is_none()
            && first_token_text(&elements, Position(0)) == Some(text_at(&elements, Position(1))),
    )
}

fn a_decoded_token_keeps_the_text_its_library_read_and_a_plain_one_has_none() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"\"\\u{41}\"fn"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    decoded_token(
        &mut builder,
        KindBits(2),
        Offset(0),
        Offset(8),
        NameText("A"),
    );
    token(&mut builder, KindBits(3), Offset(8), Offset(10));
    leave_node(&mut builder, Offset(10));
    usize::from(
        decoded_at(&elements, Position(1)) == Some(NameText("A"))
            && decoded_at(&elements, Position(2)).is_none()
            && decoded_at(&elements, Position(0)).is_none(),
    )
}

fn implied_nodes_keep_properties_without_claiming_bytes() -> usize {
    use super::service::{enter_implied_node, node_property, origin_at, property_at, source_order};
    use super::types::SourceOrder;
    use crate::domain::completion::service::intern_decoded;
    use crate::domain::relations::{
        constants::NO_SPAN,
        types::{ElementOrigin, PropertyId},
    };
    use crate::domain::symbols::service::text_of;
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let source = ByteSlice(b"<p>&amp;");
    let mut builder = table_builder(&mut elements, &mut symbols, source);
    enter_node(&mut builder, KindBits(1), Offset(0));
    enter_implied_node(&mut builder, KindBits(2));
    node_property(&mut builder, PropertyId(2), NameText("html"));
    node_property(&mut builder, PropertyId(0), NameText("decoded &"));
    leave_node(&mut builder, Offset(8));
    token(&mut builder, KindBits(3), Offset(0), Offset(3));
    token(&mut builder, KindBits(3), Offset(3), Offset(8));
    leave_node(&mut builder, Offset(8));
    intern_decoded(&mut elements, &mut symbols, source);
    usize::from(
        start_at(&elements, Position(1)) == NO_SPAN.start
            && end_at(&elements, Position(1)) == NO_SPAN.end
            && origin_at(&elements, Position(1)) == ElementOrigin::Implied
            && source_order(&elements) == SourceOrder::Reordered
            && property_at(&elements, Position(1), PropertyId(2)) == Some(NameText("html"))
            && text_of(&symbols, source, text_at(&elements, Position(1)))
                == Some(NameText("decoded &"))
            && text_of(&symbols, source, text_at(&elements, Position(2))) == Some(NameText("<p>"))
            && positions(&elements).count() == 4,
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_write_at_a_position_past_the_table_crashes_instead_of_being_lost"),
        word: Word::Succeeds,
        input: || a_write_past_the_table_crashes,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("implied_nodes_keep_properties_without_claiming_bytes"),
        word: Word::Succeeds,
        input: || implied_nodes_keep_properties_without_claiming_bytes,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_child_is_found_by_its_kind_and_a_node_holds_the_text_of_its_first_token"),
        word: Word::Succeeds,
        input: || a_child_is_found_by_its_kind_and_a_node_holds_the_text_of_its_first_token,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_decoded_token_keeps_the_text_its_library_read_and_a_plain_one_has_none"),
        word: Word::Succeeds,
        input: || a_decoded_token_keeps_the_text_its_library_read_and_a_plain_one_has_none,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_node_with_a_token_is_three_positions_with_its_root"),
        word: Word::Succeeds,
        input: || a_node_with_a_token_is_three_positions_with_its_root,
        expect: Expect::Value(1),
    },
];

/// Writes at a position past an empty table: 1 when that crashes as a position the table never gave.
fn a_write_past_the_table_crashes() -> usize {
    let message = crash_of(|| {
        let mut elements = Elements::default();
        *number_mut(&mut elements.size, Position(3)) = crate::domain::relations::types::Extent(1);
    });
    usize::from(message == "a position this table gave")
}

/// What `work` crashed with: its panic's message, or nothing when it did not crash.
fn crash_of(work: impl FnOnce()) -> String {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).map_or_else(
        |payload| {
            payload
                .downcast_ref::<&str>()
                .map(|message| (*message).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default()
        },
        |()| String::new(),
    )
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
