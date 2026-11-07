//! The `hosted` item's cases.

use super::super::interface::OperatorsItem;
use super::item::HostedItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{HostedNumber, Input as Source};
use ryo_values::domain::plans::types::Operation;

/// What a row asks of the item.
#[derive(Clone, Copy)]
enum Asked {
    /// Preparing a hosted operation over one file input.
    Hosted,
    /// Preparing an operation that is not hosted.
    Foreign,
}

type Input = Asked;
type Value = ();
type Failure = OperatingError;

fn subject(asked: Input) -> Result<Value, Failure> {
    let kinds = [CellKind::File];
    let inputs = [InputShape {
        kinds: &kinds,
        tree: None,
    }];
    let operation = match asked {
        Asked::Hosted => Operation::Hosted {
            relation: HostedNumber(0),
        },
        Asked::Foreign => Operation::Union {
            left: Source::Relation(Relation::Function),
            right: Source::Relation(Relation::Function),
        },
    };
    HostedItem
        .prepare(&operation, &inputs, &[])
        .and_then(|contract| contract.schema.map(|_| ()))
}

/// The rows of this item's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_hosted_operation_takes_the_schema_of_its_input"),
        word: Word::Succeeds,
        input: || Asked::Hosted,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("the_hosted_item_refuses_an_operation_that_is_not_hosted"),
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
