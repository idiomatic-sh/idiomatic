//! The `expansions` service's cases, on values alone.

use super::service::{adopt, mark_expansion, release};
use crate::domain::elements::service::{
    end_of, enter_node, leave_node, origin_at, real_count, source_order, table_builder, token,
};
use crate::domain::elements::types::{Elements, SourceOrder};
use crate::domain::relations::types::{
    Count, ElementOrigin, ExpansionId, KindBits, Offset, Position,
};
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;

use crate::cases::{Case, CaseName, Expect, Word, run};

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

fn adopted_elements_are_marked_and_are_not_the_files_own() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"fn"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    token(&mut builder, KindBits(2), Offset(1), Offset(2));
    leave_node(&mut builder, Offset(2));
    adopt(&mut builder, Position(0));
    token(&mut builder, KindBits(3), Offset(0), Offset(2));
    mark_expansion(&mut builder, Position(3), Position(0), ExpansionId(0));
    release(&mut builder);
    usize::from(
        end_of(&elements) == Position(4)
            && real_count(&elements) == Count(3)
            && source_order(&elements) == SourceOrder::Forward
            && origin_at(&elements, Position(1)) == ElementOrigin::File
            && origin_at(&elements, Position(3))
                == (ElementOrigin::Expansion {
                    call_site: Position(0),
                    expansion: ExpansionId(0),
                }),
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("adopted_elements_are_marked_and_are_not_the_files_own"),
    word: Word::Succeeds,
    input: || adopted_elements_are_marked_and_are_not_the_files_own,
    expect: Expect::Value(1),
}];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
