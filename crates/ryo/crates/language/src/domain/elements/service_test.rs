//! The `elements` service's cases, on values alone.

use super::service::{
    child_of_kind, children, end_at, end_of, first_token_text, kind_at, number_mut, positions,
    start_at, text_at,
};
use super::types::Elements;
use crate::domain::builders::service::{enter_node, leave_node, table_builder, token};
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{KindBits, Offset, Position};

use crate::cases::{Case, CaseName, Expect, Word, run};

fn syntax_acceptance_is_separate_and_resets_conservatively() -> usize {
    use super::service::{clear_elements, syntax_acceptance};
    use super::types::SyntaxAcceptance;
    use crate::domain::builders::service::set_syntax_acceptance;

    let mut elements = Elements::default();
    assert_eq!(syntax_acceptance(&elements), SyntaxAcceptance::Undetermined);
    let mut symbols = Symbols::default();
    {
        let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"x"));
        set_syntax_acceptance(&mut builder, SyntaxAcceptance::Rejected);
    }
    assert_eq!(syntax_acceptance(&elements), SyntaxAcceptance::Rejected);
    clear_elements(&mut elements);
    usize::from(syntax_acceptance(&elements) == SyntaxAcceptance::Undetermined)
}

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

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("syntax_acceptance_is_separate_and_resets_conservatively"),
        word: Word::Succeeds,
        input: || syntax_acceptance_is_separate_and_resets_conservatively,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("clearing_elements_removes_previous_source_coordinates"),
        word: Word::Succeeds,
        input: || {
            || {
                use crate::domain::location::{
                    service::{column, index, line},
                    types::{SourceLineBytes, SourceLineOffset},
                };
                let mut elements = Elements::default();
                index(
                    &mut super::service::engine_mut(&mut elements).line_starts,
                    SourceLineBytes(b"old\nfile\n"),
                );
                super::service::clear_elements(&mut elements);
                index(
                    &mut super::service::engine_mut(&mut elements).line_starts,
                    SourceLineBytes("é".as_bytes()),
                );
                let index = &super::service::engine(&elements).line_starts;
                usize::from(
                    line(index, SourceLineOffset(2)).0 == 1
                        && column(index, SourceLineOffset(2)).0 == 2,
                )
            }
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_write_at_a_position_past_the_table_crashes_instead_of_being_lost"),
        word: Word::Succeeds,
        input: || a_write_past_the_table_crashes,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_child_is_found_by_its_kind_and_a_node_holds_the_text_of_its_first_token"),
        word: Word::Succeeds,
        input: || a_child_is_found_by_its_kind_and_a_node_holds_the_text_of_its_first_token,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_node_with_a_token_is_three_positions_with_its_root"),
        word: Word::Succeeds,
        input: || a_node_with_a_token_is_three_positions_with_its_root,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_embedded_source_is_found_in_n_plus_m_log_m_comparisons_not_n_times_m"),
        word: Word::Succeeds,
        input: || an_embedded_source_is_found_by_bisection,
        expect: Expect::Value(1),
    },
];

/// Native elements end at this position; embedded source `i` spans `NATIVE + 4i ..+ 3`.
const NATIVE: u32 = 1000;

/// Looks up 1024 occurrences over 1024 embedded sources, counting each interval read: 1 when every
/// answer matches its source and the reads stay within N(log2(M + 1) + 2). A scan of the sources
/// per occurrence reads at least 1024 * 1024 / 2 of them here, over the bound.
fn an_embedded_source_is_found_by_bisection() -> usize {
    use super::service::{embedded_source_at, engine_mut, holding};
    use ryo_relations::domain::elements::types::{EmbeddedMapping, EmbeddedSource};

    let rows = 1024_u32;
    let spans = 1024_u32;
    let mut elements = Elements::default();
    engine_mut(&mut elements).embedded = (0..spans)
        .map(|index| EmbeddedSource {
            host: Position(index),
            root: Position(NATIVE + 4 * index),
            end: Position(NATIVE + 4 * index + 3),
            mapping: EmbeddedMapping::WholeHost,
        })
        .collect();
    let mut count = 0_u64;
    let last = u64::from(NATIVE + 4 * spans);
    let mut answers = 0_u32;
    for row in 0..u64::from(rows) {
        let at = Position(u32::try_from(row * last / u64::from(rows)).unwrap_or(u32::MAX));
        let expected =
            at.0.checked_sub(NATIVE)
                .filter(|offset| offset % 4 < 3)
                .map(|offset| Position(offset / 4));
        let counted = holding(&super::service::engine(&elements).embedded, at, |source| {
            count += 1;
            source.root..source.end
        });
        let found = embedded_source_at(&elements, at);
        if found == counted && found.map(|source| source.host) == expected {
            answers += 1;
        }
    }
    let depth = u64::from((spans + 1).next_power_of_two().trailing_zeros());
    usize::from(answers == rows && count <= u64::from(rows) * (depth + 2))
}

/// Writes at a position past an empty table: 1 when that crashes as a position the table never gave.
fn a_write_past_the_table_crashes() -> usize {
    let message = crash_of(|| {
        let mut elements = Elements::default();
        *number_mut(&mut elements.size, Position(3)) =
            ryo_relations::domain::relations::types::Extent(1);
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
