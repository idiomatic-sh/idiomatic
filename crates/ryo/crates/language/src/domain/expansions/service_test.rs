//! The `expansions` service's cases, on values alone.

use super::service::{adopt, embedded_expressions, mark_expansion, release, write_embedded};
use crate::domain::builders::service::{
    discarding_builder, enter_node, leave_node, table_builder, token,
};
use crate::domain::elements::service::{
    clear_elements, end_at, end_of, origin_at, real_count, source_order, start_at, text_at,
};
use crate::domain::elements::types::{Elements, SourceOrder};
use crate::domain::symbols::service::text_of;
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{
    Count, ElementOrigin, EmbeddedMapping, ExpansionId, KindBits, Offset, Position,
};
use ryo_relations::domain::terms::types::NameText;

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

fn embedded_text_and_physical_spans_are_independent() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let source = ByteSlice(b"\"\\x61\"");
    let mut builder = table_builder(&mut elements, &mut symbols, source);
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(6));
    leave_node(&mut builder, Offset(6));
    write_embedded(
        &mut builder,
        Position(0),
        ByteSlice(b"a"),
        EmbeddedMapping::WholeHost,
        |logical| {
            enter_node(logical, KindBits(3), Offset(0));
            token(logical, KindBits(4), Offset(0), Offset(1));
            leave_node(logical, Offset(1));
        },
    );
    let valid = real_count(&elements) == Count(2)
        && start_at(&elements, Position(3)) == Offset(0)
        && end_at(&elements, Position(3)) == Offset(6)
        && text_of(&symbols, source, text_at(&elements, Position(3))) == Some(NameText("a"))
        && text_of(&symbols, source, text_at(&elements, Position(1)))
            == Some(NameText("\"\\x61\""))
        && origin_at(&elements, Position(3)) == ElementOrigin::Embedded { host: Position(0) }
        && embedded_expressions(&elements).count() == 1;
    clear_elements(&mut elements);
    usize::from(valid && embedded_expressions(&elements).next().is_none())
}

fn discarded_embedding_never_reads_or_writes_a_host() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = discarding_builder(&mut elements, &mut symbols, ByteSlice(b""));
    let mut traversed = 0_usize;
    write_embedded(
        &mut builder,
        Position(0),
        ByteSlice(b"x"),
        EmbeddedMapping::WholeHost,
        |logical| {
            traversed += 1;
            enter_node(logical, KindBits(3), Offset(0));
            token(logical, KindBits(4), Offset(0), Offset(1));
            leave_node(logical, Offset(1));
        },
    );
    usize::from(
        traversed == 1
            && end_of(&elements) == Position(0)
            && embedded_expressions(&elements).next().is_none(),
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("discarded embedding traverses without requiring a stored host"),
        word: Word::Succeeds,
        input: || discarded_embedding_never_reads_or_writes_a_host,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("adopted_elements_are_marked_and_are_not_the_files_own"),
        word: Word::Succeeds,
        input: || adopted_elements_are_marked_and_are_not_the_files_own,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("embedded text, physical spans, and native ownership remain independent"),
        word: Word::Succeeds,
        input: || embedded_text_and_physical_spans_are_independent,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
