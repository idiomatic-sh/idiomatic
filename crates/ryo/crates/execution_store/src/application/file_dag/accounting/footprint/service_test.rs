//! The `footprint` service's cases: what a built payload and a lent scratch occupy.

use super::service::{describe, working_bytes};
use crate::application::file_dag::derived::owner::fixtures::resources::ResourceFixture;
use crate::application::file_dag::derived::owner::index::service::LIMIT;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::contract::types::{BuildWorkspace, DerivedView};
use ryo_derived::domain::native::types::NativeFinding;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::service::derived_fault;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy)]
enum Scenario {
    NativeHoldsNoSharedBacking,
    EqualityHoldsItsIndex,
    EmptyWorkspaceHoldsNothing,
}
type Input = Scenario;
/// The shared backings of a payload, or the bytes of a scratch.
type Value = usize;
type Failure = OperatingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_native_payload_holds_no_shared_backing"),
        word: Word::Succeeds,
        input: || Scenario::NativeHoldsNoSharedBacking,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("an_equality_payload_holds_its_index_as_one_shared_backing"),
        word: Word::Succeeds,
        input: || Scenario::EqualityHoldsItsIndex,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_empty_workspace_holds_no_bytes"),
        word: Word::Succeeds,
        input: || Scenario::EmptyWorkspaceHoldsNothing,
        expect: Expect::Value(0),
    },
];

/// Calls the part on one input; the payloads are built by the real derived semantics.
fn subject(input: Input) -> Result<Value, Failure> {
    let fixture =
        ResourceFixture::new(&[Relation::Function], &[], LIMIT).ok_or(OperatingError::NotRun)?;
    let column = FactColumn(vec![1, 2]);
    let held = fixture.index(
        Relation::Function,
        &Rows::Stored(ColumnsView {
            kinds: &[CellKind::Number],
            columns: std::slice::from_ref(&column),
        }),
        &[ColumnIndex(0)],
    )?;
    let payload = held.storage().payload();
    match input {
        Scenario::NativeHoldsNoSharedBacking => Ok(describe(
            &DerivedView::NativeLookup(&NativeFinding::Span(ColumnIndex(0))),
            payload,
        )
        .map_err(derived_fault)?
        .shared
        .len()),
        Scenario::EqualityHoldsItsIndex => Ok(describe(&held.view()?, payload)
            .map_err(derived_fault)?
            .shared
            .len()),
        Scenario::EmptyWorkspaceHoldsNothing => Ok(working_bytes(&BuildWorkspace::default())
            .map_err(derived_fault)?
            .0),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
