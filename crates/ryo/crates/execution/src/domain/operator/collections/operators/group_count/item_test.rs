//! The `group_count` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::super::interface::OperatorsItem;
use super::item::GroupCountItem;
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::QueryOperator;

/// What a row asks of the item.
#[derive(Clone, Copy)]
enum Asked {
    /// The shared cases of the collection.
    Describe,
    /// Preparing an operation that is not this operator's.
    Foreign,
}

type Input = Asked;
type Value = ();
type Failure = OperatingError;

fn subject(asked: Input) -> Result<Value, Failure> {
    match asked {
        Asked::Describe => {
            check(QueryOperator::GroupCount);
            Ok(())
        }
        Asked::Foreign => GroupCountItem
            .prepare(
                &Operation::Union {
                left: Source::Relation(Relation::Function),
                right: Source::Relation(Relation::Function),
            },
                &[],
                &[],
            )
            .map(|_| ()),
    }
}

/// The rows of this item's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_group_count_operator_describes_itself"),
        word: Word::Succeeds,
        input: || Asked::Describe,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("the_group_count_item_refuses_an_operation_that_is_not_its_own"),
        word: Word::Refuses,
        input: || Asked::Foreign,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
