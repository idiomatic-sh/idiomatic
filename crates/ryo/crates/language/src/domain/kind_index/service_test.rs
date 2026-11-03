//! The `kind_index` service's cases, on values alone.

use super::service::{index_kinds, of_kind};
use crate::domain::elements::service::{enter_node, leave_node, table_builder, token};
use crate::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::{KindBits, Offset, Position};
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;

use crate::cases::{Case, CaseName, Expect, Word, run};

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

fn the_elements_of_a_kind_are_found_in_order() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"abcd"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(3), Offset(0), Offset(1));
    enter_node(&mut builder, KindBits(1), Offset(1));
    token(&mut builder, KindBits(3), Offset(1), Offset(2));
    leave_node(&mut builder, Offset(2));
    leave_node(&mut builder, Offset(2));
    index_kinds(&mut elements);
    usize::from(
        of_kind(&elements, KindBits(1)) == [Position(0), Position(2)]
            && of_kind(&elements, KindBits(3)) == [Position(1), Position(3)]
            && of_kind(&elements, KindBits(2)).is_empty()
            && of_kind(&elements, KindBits(40)).is_empty(),
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_elements_of_a_kind_are_found_in_order"),
    word: Word::Succeeds,
    input: || the_elements_of_a_kind_are_found_in_order,
    expect: Expect::Value(1),
}];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
