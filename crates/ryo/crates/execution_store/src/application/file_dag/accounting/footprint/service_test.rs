//! The `footprint` service's cases: the bytes and the allocation identity of what a stored payload and a lent scratch occupy.

use super::service::{describe, working_bytes};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
use ryo_derived::domain::contract::types::{BuildInputs, BuildOutcome, BuildWorkspace};
use ryo_derived::domain::dispatch::service::build;
use ryo_derived::domain::membership::types::MarkSpace;
use ryo_derived::domain::native::types::NativeForm;
use ryo_derived::domain::preparation::service::prepare;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::service::derived_fault;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};
use std::sync::Arc;

#[derive(Clone, Copy)]
enum Scenario {
    /// A native lookup over the element table.
    Native,
    /// A key index over three numbers.
    Equality,
    /// The members of three numbers, marked over a dense space of eight values: the bytes past the marks' own vector.
    MarksCapacity,
    /// The same members: the allocation the marks' backing is keyed by.
    MarksIdentity,
    /// A scratch never lent a row.
    EmptyWorkspace,
}
type Input = Scenario;
/// What the payload occupies, rendered: its owned bytes and each shared backing's tag and bytes and whether the backing is
/// keyed by the payload's own allocation; for the marks, the bytes past their vector against their capacity, or their key.
type Value = &'static str;
type Failure = OperatingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_native_payload_holds_no_shared_backing"),
        word: Word::Succeeds,
        input: || Scenario::Native,
        expect: Expect::Value("owned 64; shared []"),
    },
    Case {
        name: CaseName("an_equality_payload_holds_its_index_as_one_shared_backing"),
        word: Word::Succeeds,
        input: || Scenario::Equality,
        expect: Expect::Value(
            "owned 64; shared [\"tag 11 bytes 288 keyed by its own allocation true\"]",
        ),
    },
    Case {
        name: CaseName("the_capacity_of_marks_covers_them"),
        word: Word::Succeeds,
        input: || Scenario::MarksCapacity,
        expect: Expect::Value("bytes past the marks' vector 8, marks capacity 8"),
    },
    Case {
        name: CaseName("marks_are_identified_by_their_allocation"),
        word: Word::Succeeds,
        input: || Scenario::MarksIdentity,
        expect: Expect::Value("tag 10 keyed by the marks' allocation true"),
    },
    Case {
        name: CaseName("an_empty_workspace_holds_no_bytes"),
        word: Word::Succeeds,
        input: || Scenario::EmptyWorkspace,
        expect: Expect::Value("bytes 0"),
    },
];

fn numbers(values: &[i64]) -> Table {
    Table {
        kinds: vec![CellKind::Number],
        cells: values
            .iter()
            .map(|value| Cell::Number(Integer(*value)))
            .collect(),
        count: RowCount(values.len()),
    }
}

/// The payload the real derived semantics build for `request` over `inputs`.
fn built(
    request: DerivedRequest,
    schemas: Vec<Vec<CellKind>>,
    inputs: BuildInputs<'_>,
) -> Result<DerivedPayload, OperatingError> {
    let prepared = prepare(request, schemas);
    let BuildOutcome::Built(payload) =
        build(&prepared, inputs, &mut BuildWorkspace::default(), None).map_err(derived_fault)?
    else {
        return Err(OperatingError::WrongOperation);
    };
    Ok(payload)
}

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let rows = numbers(&[3, 1, 3]);
    let space: Vec<Cell> = (0..8).map(|value| Cell::Number(Integer(value))).collect();
    let payload = match input {
        Scenario::EmptyWorkspace => {
            let bytes = working_bytes(&BuildWorkspace::default()).map_err(derived_fault)?;
            return Ok(Box::leak(format!("bytes {}", bytes.0).into_boxed_str()));
        }
        Scenario::Native => built(
            DerivedRequest::NativeLookup {
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                view: vec![ColumnIndex(0)],
            },
            vec![vec![CellKind::Node], vec![CellKind::Node]],
            BuildInputs::Native(NativeForm::Elements),
        )?,
        Scenario::Equality => built(
            DerivedRequest::Equality {
                columns: vec![ColumnIndex(0)],
            },
            vec![vec![CellKind::Number]],
            BuildInputs::Rows(&rows),
        )?,
        Scenario::MarksCapacity | Scenario::MarksIdentity => built(
            DerivedRequest::Membership {
                column: ColumnIndex(0),
            },
            vec![vec![CellKind::Number]],
            BuildInputs::Membership {
                rows: &rows,
                space: MarkSpace::Dense(&space),
            },
        )?,
    };
    let footprint = describe(&payload).map_err(derived_fault)?;
    if let Some(marks) = payload.share_marks() {
        let [backing] = footprint.shared.as_slice() else {
            return Err(OperatingError::ShapeMismatch);
        };
        let shown = if let Scenario::MarksCapacity = input {
            let vector = size_of::<Vec<bool>>() + 2 * size_of::<usize>();
            format!(
                "bytes past the marks' vector {}, marks capacity {}",
                backing.bytes.0.saturating_sub(vector),
                marks.0.capacity()
            )
        } else {
            format!(
                "tag {} keyed by the marks' allocation {}",
                backing.tag.0,
                backing.allocation_key.0 == Arc::as_ptr(&marks.0).addr()
            )
        };
        return Ok(Box::leak(shown.into_boxed_str()));
    }
    let own = payload
        .share_equality()
        .map(|index| Arc::as_ptr(&index.0).addr());
    let shared = footprint
        .shared
        .iter()
        .map(|backing| {
            format!(
                "tag {} bytes {} keyed by its own allocation {}",
                backing.tag.0,
                backing.bytes.0,
                Some(backing.allocation_key.0) == own,
            )
        })
        .collect::<Vec<_>>();
    Ok(Box::leak(
        format!("owned {}; shared {shared:?}", footprint.owned_bytes.0).into_boxed_str(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
