//! An axis answers one row for each admitted candidate, in walk order; the nearest ancestor answers one.
use super::service::evaluate;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::axis::types::AxisAdmission;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::evaluation::fixtures::{FakeDriver, number, written};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, JoinColumn, Side};

/// Which walk the engine answers with.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Three entities, the second not admitted.
    Entities,
    /// The nearest ancestor of three candidates, the first not admitted.
    Nearest,
    /// A complete answer the engine already holds.
    Cached,
    /// Two elements, answered in the declared output over the row and the element.
    Elements,
}
type Input = Scenario;
type Value = std::borrow::Cow<'static, str>;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_admitted_entity_is_one_row_in_walk_order"),
        word: Word::Succeeds,
        input: || Scenario::Entities,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 2|1 4")),
    },
    Case {
        name: CaseName("the_nearest_ancestor_is_the_first_admitted_candidate_alone"),
        word: Word::Succeeds,
        input: || Scenario::Nearest,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 3")),
    },
    Case {
        name: CaseName("a_cached_answer_is_emitted_as_it_is"),
        word: Word::Succeeds,
        input: || Scenario::Cached,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 7|1 8")),
    },
    Case {
        name: CaseName("an_element_answers_the_declared_output"),
        word: Word::Succeeds,
        input: || Scenario::Elements,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 21|1 31")),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    let row = [number(1)];
    let mut driver = FakeDriver {
        candidates: vec![number(2), number(3), number(4)],
        ..FakeDriver::default()
    };
    let cached = [number(7), number(8)];
    let output = [
        JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        },
        JoinColumn {
            side: Side::Right,
            column: ColumnIndex(1),
        },
    ];
    let (direction, output, cached): (Direction, &[JoinColumn], Option<&[_]>) = match scenario {
        Scenario::Entities => {
            driver.admitted = vec![AxisAdmission::In, AxisAdmission::Out, AxisAdmission::In];
            (Direction::Descendant, &[][..], None)
        }
        Scenario::Nearest => {
            driver.admitted = vec![AxisAdmission::Out, AxisAdmission::In, AxisAdmission::In];
            (Direction::NearestAncestor, &[][..], None)
        }
        Scenario::Cached => (Direction::Descendant, &[][..], Some(&cached[..])),
        Scenario::Elements => {
            driver.candidates = vec![number(2), number(3)];
            driver.elements = vec![vec![number(20), number(21)], vec![number(30), number(31)]];
            (Direction::Descendant, &output[..], None)
        }
    };
    evaluate(
        &row,
        ColumnIndex(0),
        direction,
        output,
        cached,
        &mut driver,
        &mut Vec::new(),
    )?;
    Ok(std::borrow::Cow::Owned(written(&driver.emitted)))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
