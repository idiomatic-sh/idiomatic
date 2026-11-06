//! The `decoded` service's cases, on values alone.

use super::service::decoded_at;
use crate::domain::builders::service::{
    decoded_token, enter_node, leave_node, table_builder, token,
};
use crate::domain::elements::types::Elements;
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{KindBits, Offset, Position};
use ryo_relations::domain::terms::types::NameText;

use crate::cases::{Case, CaseName, Expect, Word, run};

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

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

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_decoded_token_keeps_the_text_its_library_read_and_a_plain_one_has_none"),
    word: Word::Succeeds,
    input: || a_decoded_token_keeps_the_text_its_library_read_and_a_plain_one_has_none,
    expect: Expect::Value(1),
}];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
