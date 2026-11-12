//! The `footprint` service's cases: the bytes and the allocation identity of what a stored payload and a lent scratch occupy.

use super::service::{describe, working_bytes};
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::service::DerivedStorage;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::derived::factory::semantics;
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::contract::types::{BuildInputs, BuildOutcome, BuildWorkspace};
use ryo_derived::domain::membership::types::MarkSpace;
use ryo_derived::domain::native::types::NativeForm;
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
    /// The members of three numbers, marked over a dense space of eight values.
    DenseMembership,
    /// A scratch never lent a row.
    EmptyWorkspace,
}
type Input = Scenario;
/// What the payload occupies, rendered: its owned bytes and each shared backing's tag and bytes, whether the
/// backing is keyed by the payload's own allocation, and for marks whether the bytes cover the marks' capacity.
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
            "owned 64; shared [\"tag 11 bytes 288 keyed by its own allocation true covers the marks false\"]",
        ),
    },
    Case {
        name: CaseName("the_capacity_of_marks_covers_them"),
        word: Word::Succeeds,
        input: || Scenario::DenseMembership,
        expect: Expect::Value(
            "owned 64; shared [\"tag 10 bytes 48 keyed by its own allocation true covers the marks true\"]",
        ),
    },
    Case {
        name: CaseName("marks_are_identified_by_their_allocation"),
        word: Word::Succeeds,
        input: || Scenario::DenseMembership,
        expect: Expect::Value(
            "owned 64; shared [\"tag 10 bytes 48 keyed by its own allocation true covers the marks true\"]",
        ),
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

/// The payload the real derived semantics build for `request` over `inputs`, stored as the owner stores it.
fn stored(
    request: DerivedRequest,
    schemas: Vec<Vec<CellKind>>,
    inputs: BuildInputs<'_>,
) -> Result<DerivedStorage, OperatingError> {
    let port: Arc<dyn DerivedPort> = Arc::new(semantics());
    let prepared = port.prepare(request, schemas);
    let BuildOutcome::Built(payload) = port
        .build(&prepared, inputs, &mut BuildWorkspace::default(), None)
        .map_err(derived_fault)?
    else {
        return Err(OperatingError::WrongOperation);
    };
    Ok(DerivedStorage::new(port, Arc::new(prepared), payload, None))
}

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let rows = numbers(&[3, 1, 3]);
    let space: Vec<Cell> = (0..8).map(|value| Cell::Number(Integer(value))).collect();
    let storage = match input {
        Scenario::EmptyWorkspace => {
            let bytes = working_bytes(&BuildWorkspace::default()).map_err(derived_fault)?;
            return Ok(Box::leak(format!("bytes {}", bytes.0).into_boxed_str()));
        }
        Scenario::Native => stored(
            DerivedRequest::NativeLookup {
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                view: vec![ColumnIndex(0)],
            },
            vec![vec![CellKind::Node], vec![CellKind::Node]],
            BuildInputs::Native(NativeForm::Elements),
        )?,
        Scenario::Equality => stored(
            DerivedRequest::Equality {
                columns: vec![ColumnIndex(0)],
            },
            vec![vec![CellKind::Number]],
            BuildInputs::Rows(&rows),
        )?,
        Scenario::DenseMembership => stored(
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
    let footprint = describe(&storage)?;
    let payload = storage.payload();
    let own = payload
        .share_equality()
        .map(|index| Arc::as_ptr(&index.0).addr())
        .or_else(|| {
            payload
                .share_marks()
                .map(|marks| Arc::as_ptr(&marks.0).addr())
        });
    let marks = payload.share_marks().map(|marks| marks.0.capacity());
    let shared = footprint
        .shared
        .iter()
        .map(|backing| {
            format!(
                "tag {} bytes {} keyed by its own allocation {} covers the marks {}",
                backing.tag.0,
                backing.bytes.0,
                Some(backing.allocation_key.0) == own,
                marks.is_some_and(|capacity| backing.bytes.0 >= capacity),
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
