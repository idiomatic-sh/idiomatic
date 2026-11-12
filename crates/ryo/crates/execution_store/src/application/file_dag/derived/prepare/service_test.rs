//! Behavioral proof of `prepare`: a view request is prepared as a contract of its own kind, whose identity follows the
//! request's meaning and the input schemas.
use super::service::view;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::derived_views::types::ViewRequest;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operation_schema::types::InputShape;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

/// The second request prepared beside an equality on column 0 of two number columns.
#[derive(Clone, Copy, Debug)]
enum Second {
    /// The same equality.
    Same,
    /// An equality on column 1 of the same columns.
    OtherColumn,
    /// The equality on column 0 of a number and a symbol column.
    OtherKinds,
    /// A membership of column 0 of the same columns.
    Membership,
}

type Input = Second;

/// The second contract's kind, and whether its identity is the first's.
type Value = (DerivedKind, bool);
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_same_request_over_the_same_schemas_is_one_identity"),
        word: Word::Succeeds,
        input: || Second::Same,
        expect: Expect::Value((DerivedKind::Equality, true)),
    },
    Case {
        name: CaseName("another_column_is_another_identity"),
        word: Word::Succeeds,
        input: || Second::OtherColumn,
        expect: Expect::Value((DerivedKind::Equality, false)),
    },
    Case {
        name: CaseName("another_input_schema_is_another_identity"),
        word: Word::Succeeds,
        input: || Second::OtherKinds,
        expect: Expect::Value((DerivedKind::Equality, false)),
    },
    Case {
        name: CaseName("a_request_is_prepared_as_its_own_kind"),
        word: Word::Succeeds,
        input: || Second::Membership,
        expect: Expect::Value((DerivedKind::Membership, false)),
    },
];

/// Prepares the equality on column 0 and the second request, both through the fake derived port.
fn subject(second: Second) -> Result<Value, Failure> {
    let numbers = [CellKind::Number, CellKind::Number];
    let mixed = [CellKind::Number, CellKind::Symbol];
    let port = || Arc::new(crate::application::file_dag::fixtures::derived());
    let equality = |column| ViewRequest::Equality {
        columns: vec![ColumnIndex(column)],
    };
    let first = view(
        port(),
        &equality(0),
        &[InputShape {
            kinds: &numbers,
            tree: None,
        }],
    )?;
    let (request, kinds): (ViewRequest, &[CellKind]) = match second {
        Second::Same => (equality(0), &numbers),
        Second::OtherColumn => (equality(1), &numbers),
        Second::OtherKinds => (equality(0), &mixed),
        Second::Membership => (
            ViewRequest::Membership {
                column: ColumnIndex(0),
            },
            &numbers,
        ),
    };
    let other = view(port(), &request, &[InputShape { kinds, tree: None }])?;
    Ok((other.kind, other.identity_hash() == first.identity_hash()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
