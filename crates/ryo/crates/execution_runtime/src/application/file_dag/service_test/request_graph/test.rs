//! Request-graph identities and ordered dependency contracts over the operators the application prepares.
use super::fixtures::{Graphed, Requested, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::file_program::types::SchemaValidity;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::types::DemandOrder;

type Input = Requested;
type Value = Graphed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("caller_origins_do_not_split_pure_identity"),
        word: Word::Succeeds,
        input: || Requested::Origins,
        expect: Expect::Value(Graphed::Shared(true)),
    },
    Case {
        name: CaseName("different_bound_texts_keep_distinct_identity"),
        word: Word::Succeeds,
        input: || Requested::Bindings,
        expect: Expect::Value(Graphed::Shared(false)),
    },
    Case {
        name: CaseName("duplicate_steps_share_physical_identity"),
        word: Word::Succeeds,
        input: || Requested::Duplicates,
        expect: Expect::Value(Graphed::Duplicated([true; 3])),
    },
    Case {
        name: CaseName("equal_derived_text_steps_of_different_rules_share_one_producer"),
        word: Word::Succeeds,
        input: || Requested::DerivedText,
        expect: Expect::Value(Graphed::Shared(true)),
    },
    Case {
        name: CaseName("equal_request_source_steps_of_different_rules_keep_their_own_producers"),
        word: Word::Succeeds,
        input: || Requested::RequestSource,
        expect: Expect::Value(Graphed::Shared(false)),
    },
    Case {
        name: CaseName("ordered_dependencies_remain_authored_inputs"),
        word: Word::Succeeds,
        input: || Requested::Dependencies,
        expect: Expect::Value(Graphed::Dependencies {
            distinct_union: true,
            orders: [
                DemandOrder::LeftThenRight,
                DemandOrder::LeftThenRight,
                DemandOrder::Unary,
            ],
            authored: [true; 3],
        }),
    },
    Case {
        name: CaseName("invalid_schema_stays_deferred_in_same_graph"),
        word: Word::Succeeds,
        input: || Requested::InvalidContractShape,
        expect: Expect::Value(Graphed::Validity(SchemaValidity::Invalid)),
    },
];

/// Compiles the request's plans into one graph and reads its producers.
fn subject(requested: Input) -> Result<Value, Failure> {
    exercise(requested)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
