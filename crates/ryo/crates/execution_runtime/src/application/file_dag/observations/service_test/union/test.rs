//! Named producer populations use the actual prepared union and common file scheduler.
use super::fixtures::exercise;
use crate::cases::{Case, CaseName, Expect, Word, run};
fn singleton_and_many_producers_preserve_distinct_union_order_through_one_prepared_root() {
    let cases: &[(&[&[i64]], &[i64], usize)] = &[
        (&[&[]], &[], 0),
        (&[&[1, 1, 2]], &[1, 2], 6),
        (&[&[1, 1, 2], &[]], &[1, 2], 3),
        (&[&[], &[1, 1, 2]], &[1, 2], 3),
        (&[&[1, 2], &[2, 3], &[1, 3]], &[1, 2, 3], 5),
    ];
    for &(given, expected, registered) in cases {
        exercise(given, Some(expected), registered);
    }
}
fn zero_producers_preserve_the_declared_project_phase_in_file_scope() {
    exercise(&[], None, 0);
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    SingletonAndManyProducersPreserveDistinctUnionOrderThroughOnePreparedRoot,
    ZeroProducersPreserveTheDeclaredProjectPhaseInFileScope,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "singleton_and_many_producers_preserve_distinct_union_order_through_one_prepared_root",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::SingletonAndManyProducersPreserveDistinctUnionOrderThroughOnePreparedRoot
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("zero_producers_preserve_the_declared_project_phase_in_file_scope"),
        word: Word::Succeeds,
        input: || Scenario::ZeroProducersPreserveTheDeclaredProjectPhaseInFileScope,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::SingletonAndManyProducersPreserveDistinctUnionOrderThroughOnePreparedRoot => {
            singleton_and_many_producers_preserve_distinct_union_order_through_one_prepared_root();
            Ok(())
        }
        Scenario::ZeroProducersPreserveTheDeclaredProjectPhaseInFileScope => {
            zero_producers_preserve_the_declared_project_phase_in_file_scope();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
