//! A closure answers its partition's key, the source and each vertex the source reaches, in walk order.
use super::service::evaluate;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::contract::types::RowScratch;
use crate::domain::evaluation::fixtures::{FakeDriver, number, written};

/// Which partitions the engine lends.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// One partition with two sources, the second reaching nothing.
    Reaching,
    /// Two partitions, each with one source.
    Partitioned,
    /// No partition.
    Empty,
}
type Input = Scenario;
type Value = std::borrow::Cow<'static, str>;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_reached_vertex_is_one_row_after_its_key_and_source"),
        word: Word::Succeeds,
        input: || Scenario::Reaching,
        expect: Expect::Value(std::borrow::Cow::Borrowed("10 1 3|10 1 4")),
    },
    Case {
        name: CaseName("each_partition_answers_under_its_own_key"),
        word: Word::Succeeds,
        input: || Scenario::Partitioned,
        expect: Expect::Value(std::borrow::Cow::Borrowed("10 1 3|11 2 5")),
    },
    Case {
        name: CaseName("no_partition_answers_no_row"),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut driver = FakeDriver {
        reaches: vec![
            (number(1), vec![number(3), number(4)]),
            (number(2), vec![number(5)]),
        ],
        ..FakeDriver::default()
    };
    driver.partitions = match scenario {
        Scenario::Reaching => vec![(vec![number(10)], vec![number(1), number(6)])],
        Scenario::Partitioned => vec![
            (vec![number(10)], vec![number(1)]),
            (vec![number(11)], vec![number(2)]),
        ],
        Scenario::Empty => Vec::new(),
    };
    if let Scenario::Partitioned = scenario {
        driver.reaches[0].1.truncate(1);
    }
    evaluate(&mut driver, &mut RowScratch::default())?;
    Ok(std::borrow::Cow::Owned(written(&driver.emitted)))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
