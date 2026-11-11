//! Real publication admissions bind owner, generation, descriptor and semantic source.
use super::service::AcquiredView;
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
use crate::application::file_dag::{
    fixtures::program_derived_port, relations::service::RelationOwner,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{
    derived_views::types::{DerivedInputs, DerivedScratch, MembershipSpace, ViewRequest},
    operating::errors::OperatingError,
    operation_schema::types::InputShape,
};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    rows::types::{RowCount, Table},
};
use std::sync::Arc;

/// The acquisition attempted: what is reused, foreign, expired or substituted.
#[derive(Clone, Copy, Debug)]
enum Acquisition {
    Reuse,
    ForeignOwner,
    ExpiredAdmission,
    ForeignDescriptor,
    SubstitutedRows,
    UnprovedDense,
    ExpiredView,
    CloneKeepsLease,
}

/// What a completed acquisition shows.
#[derive(Debug, PartialEq, Eq)]
enum Acquired {
    /// A second acquisition: whether it reached the first's storage, how it was built, and the first view's row order.
    Reused {
        same_storage: bool,
        construction: Construction,
        order: [usize; 3],
    },
    /// The row order a view still reads after its population is released.
    Held { order: [usize; 3] },
    /// The required bytes before, while one view is held, after it drops with a clone alive (and
    /// the order the clone reads), and after the clone drops.
    Leased {
        before: usize,
        held: usize,
        cloned: usize,
        read: [usize; 3],
        released: usize,
    },
}

type Input = Acquisition;
type Value = Acquired;
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("actual_publication_reuses_same_physical_view"),
        word: Word::Succeeds,
        input: || Acquisition::Reuse,
        // Keys 2, 1, 2: key 2 hashes below key 1 under the derived key hash (one `FxHasher` step), so its rows come first.
        expect: Expect::Value(Acquired::Reused {
            same_storage: true,
            construction: Construction::Reused,
            order: [0, 2, 1],
        }),
    },
    Case {
        name: CaseName("another_owner_cannot_adopt_source_admission"),
        word: Word::Refuses,
        input: || Acquisition::ForeignOwner,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("source_admission_expires_with_its_generation"),
        word: Word::Refuses,
        input: || Acquisition::ExpiredAdmission,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("equivalent_descriptor_does_not_retarget_capability"),
        word: Word::Refuses,
        input: || Acquisition::ForeignDescriptor,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("equal_values_do_not_substitute_physical_source"),
        word: Word::Refuses,
        input: || Acquisition::SubstitutedRows,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("dense_domain_requires_actual_parent_graph"),
        word: Word::Refuses,
        input: || Acquisition::UnprovedDense,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("held_payload_remains_immutable_under_its_required_lease"),
        word: Word::Succeeds,
        input: || Acquisition::ExpiredView,
        expect: Expect::Value(Acquired::Held { order: [0, 2, 1] }),
    },
    Case {
        name: CaseName("acquired_view_clone_keeps_the_required_lease_until_the_last_drops"),
        word: Word::Succeeds,
        input: || Acquisition::CloneKeepsLease,
        expect: Expect::Value(Acquired::Leased {
            before: 401,
            held: 545,
            cloned: 545,
            read: [0, 2, 1],
            released: 401,
        }),
    },
];
fn table() -> Table {
    Table {
        kinds: vec![CellKind::Number],
        cells: vec![
            Cell::Number(Integer(2)),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(2)),
        ],
        count: RowCount(3),
    }
}
/// Admits a computed publication, plans its view, and runs the acquisition.
fn subject(input: Input) -> Result<Value, Failure> {
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        program_derived_port(),
    );
    let publication = owner.computed(table())?;
    let rows = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let proof = owner.admit(publication.as_ref(), RepresentationInput::from(&rows))?;
    let request = match input {
        Acquisition::UnprovedDense => ViewRequest::Membership {
            column: ColumnIndex(0),
        },
        Acquisition::Reuse
        | Acquisition::ForeignOwner
        | Acquisition::ExpiredAdmission
        | Acquisition::ForeignDescriptor
        | Acquisition::SubstitutedRows
        | Acquisition::ExpiredView
        | Acquisition::CloneKeepsLease => ViewRequest::PartitionOrder {
            keys: vec![ColumnIndex(0)],
        },
    };
    let plan = Arc::new(owner.derived().prepare_view(
        &request,
        &[InputShape {
            kinds: proof.rows().kinds(),
            tree: None,
        }],
    )?);
    let admitted = owner.derived().admit_view(&plan, &owner, &[&proof])?;
    let mut scratch = DerivedScratch::default();
    let original = [proof.rows()];
    match input {
        Acquisition::ForeignOwner => {
            let foreign = RelationOwner::new(
                crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
                program_derived_port(),
            );
            foreign
                .derived()
                .acquire_stored(
                    &plan,
                    &admitted,
                    Default::default(),
                    DerivedInputs::Rows(&original),
                    &mut scratch,
                    None,
                )
                .and_then(|view| held(&view))
        }
        Acquisition::ExpiredAdmission => {
            owner.release_population();
            owner
                .derived()
                .acquire_stored(
                    &plan,
                    &admitted,
                    Default::default(),
                    DerivedInputs::Rows(&original),
                    &mut scratch,
                    None,
                )
                .and_then(|view| held(&view))
        }
        Acquisition::ForeignDescriptor => owner
            .derived()
            .acquire_stored(
                &Arc::new((*plan).clone()),
                &admitted,
                Default::default(),
                DerivedInputs::Rows(&original),
                &mut scratch,
                None,
            )
            .and_then(|view| held(&view)),
        Acquisition::SubstitutedRows => owner
            .derived()
            .acquire_stored(
                &plan,
                &admitted,
                Default::default(),
                DerivedInputs::Rows(&[RepresentationInput::Computed(&table())]),
                &mut scratch,
                None,
            )
            .and_then(|view| held(&view)),
        Acquisition::UnprovedDense => owner
            .derived()
            .acquire_stored(
                &plan,
                &admitted,
                Default::default(),
                DerivedInputs::Membership {
                    rows: proof.rows(),
                    space: MembershipSpace::Dense(&[]),
                },
                &mut scratch,
                None,
            )
            .and_then(|view| held(&view)),
        Acquisition::CloneKeepsLease => {
            let required = || owner.derived().ledger.borrow().snapshot().required_bytes;
            let before = required();
            let first = owner.derived().acquire_stored(
                &plan,
                &admitted,
                Default::default(),
                DerivedInputs::Rows(&original),
                &mut scratch,
                None,
            )?;
            let held = required();
            let clone = first.clone();
            drop(first);
            let cloned = required();
            let read = order(&clone)?;
            drop(clone);
            Ok(Acquired::Leased {
                before: before.0,
                held: held.0,
                cloned: cloned.0,
                read,
                released: required().0,
            })
        }
        Acquisition::ExpiredView | Acquisition::Reuse => {
            let first: AcquiredView = owner.derived().acquire_stored(
                &plan,
                &admitted,
                Default::default(),
                DerivedInputs::Rows(&original),
                &mut scratch,
                None,
            )?;
            if matches!(input, Acquisition::ExpiredView) {
                owner.release_population();
                return held(&first);
            }
            let second = owner.derived().acquire_stored(
                &plan,
                &admitted,
                Default::default(),
                DerivedInputs::Rows(&original),
                &mut scratch,
                None,
            )?;
            Ok(Acquired::Reused {
                same_storage: first.storage_identity() == second.storage_identity(),
                construction: second.construction(),
                order: order(&first)?,
            })
        }
    }
}

/// The row order a partition-order view reads.
fn order(view: &AcquiredView) -> Result<[usize; 3], OperatingError> {
    let ryo_derived::domain::contract::types::DerivedView::PartitionOrder(order) = view.view()?
    else {
        return Err(OperatingError::WrongOperation);
    };
    order
        .iter()
        .map(|row| row.0)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| OperatingError::WrongOperation)
}

/// The view as held: the row order it reads.
fn held(view: &AcquiredView) -> Result<Acquired, OperatingError> {
    order(view).map(|order| Acquired::Held { order })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
