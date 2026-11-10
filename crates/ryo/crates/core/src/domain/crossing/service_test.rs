//! The `crossing` service's cases, on values alone.

use super::service::placeable;
use super::types::Placeability;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::rows::types::Table;
use std::convert::Infallible;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Asks whether an emit over a table of columns of these kinds can have a place in a file, by its subject column and its key columns.
    Placing {
        kinds: &'static [CellKind],
        subject: ColumnIndex,
        key: &'static [ColumnIndex],
    },
}

/// What a row of the cases does.
type Input = Scenario;
/// Whether the finding can have a place in a file.
type Value = Placeability;
/// Nothing of this part is refused.
type Failure = Infallible;

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::Placing {
            kinds,
            subject,
            key,
        } => {
            let table = Table {
                kinds: kinds.to_vec(),
                ..Table::default()
            };
            let emit = Emit {
                step: StepId(0),
                subject,
                key: key.to_vec(),
                headings: Vec::new(),
            };
            placeable(&emit, &table)
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_subject_that_is_a_name_has_a_place"),
        word: Word::Succeeds,
        input: || Scenario::Placing {
            kinds: &[CellKind::Symbol, CellKind::Number],
            subject: ColumnIndex(0),
            key: &[],
        },
        expect: Expect::Value(Placeability::Placeable),
    },
    Case {
        name: CaseName("a_subject_that_is_a_stretch_has_a_place"),
        word: Word::Succeeds,
        input: || Scenario::Placing {
            kinds: &[CellKind::Number, CellKind::Span],
            subject: ColumnIndex(1),
            key: &[],
        },
        expect: Expect::Value(Placeability::Placeable),
    },
    Case {
        name: CaseName("a_subject_that_is_a_node_has_a_place"),
        word: Word::Succeeds,
        input: || Scenario::Placing {
            kinds: &[CellKind::Node],
            subject: ColumnIndex(0),
            key: &[],
        },
        expect: Expect::Value(Placeability::Placeable),
    },
    Case {
        name: CaseName("a_subject_that_is_a_number_has_no_place"),
        word: Word::Succeeds,
        input: || Scenario::Placing {
            kinds: &[CellKind::Symbol, CellKind::Number],
            subject: ColumnIndex(1),
            key: &[],
        },
        expect: Expect::Value(Placeability::Unplaceable),
    },
    Case {
        name: CaseName("a_key_column_that_is_a_name_gives_a_place"),
        word: Word::Succeeds,
        input: || Scenario::Placing {
            kinds: &[CellKind::Symbol, CellKind::Number],
            subject: ColumnIndex(1),
            key: &[ColumnIndex(0)],
        },
        expect: Expect::Value(Placeability::Placeable),
    },
    Case {
        name: CaseName("a_column_the_table_does_not_have_gives_no_place"),
        word: Word::Succeeds,
        input: || Scenario::Placing {
            kinds: &[CellKind::Symbol],
            subject: ColumnIndex(4),
            key: &[],
        },
        expect: Expect::Value(Placeability::Unplaceable),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<_, Failure>(subject(input))
    });
}
