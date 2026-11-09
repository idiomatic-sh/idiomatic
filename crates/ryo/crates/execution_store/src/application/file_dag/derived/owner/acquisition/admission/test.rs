//! Real publication admissions bind owner, generation, descriptor and semantic source.
use super::service::AcquiredView;
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
use crate::application::file_dag::{fixtures::program_derived_port, relations::RelationOwner};
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

#[derive(Clone, Copy, Debug)]
enum Scenario {
    Reuse,
    ForeignOwner,
    ExpiredAdmission,
    ForeignDescriptor,
    SubstitutedRows,
    UnprovedDense,
    ExpiredView,
    CloneKeepsLease,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("actual_publication_reuses_same_physical_view"),
        word: Word::Succeeds,
        input: || Input::Reuse,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("another_owner_cannot_adopt_source_admission"),
        word: Word::Refuses,
        input: || Input::ForeignOwner,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("source_admission_expires_with_its_generation"),
        word: Word::Refuses,
        input: || Input::ExpiredAdmission,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("equivalent_descriptor_does_not_retarget_capability"),
        word: Word::Refuses,
        input: || Input::ForeignDescriptor,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("equal_values_do_not_substitute_physical_source"),
        word: Word::Refuses,
        input: || Input::SubstitutedRows,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("dense_domain_requires_actual_parent_graph"),
        word: Word::Refuses,
        input: || Input::UnprovedDense,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("held_payload_remains_immutable_under_its_required_lease"),
        word: Word::Succeeds,
        input: || Input::ExpiredView,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("acquired_view_clone_keeps_the_required_lease_until_the_last_drops"),
        word: Word::Succeeds,
        input: || Input::CloneKeepsLease,
        expect: Expect::Value(()),
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
fn subject(input: Input) -> Result<(), Failure> {
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        program_derived_port(),
    );
    let publication = owner.computed(table())?;
    let rows = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let proof = owner.admit(publication.as_ref(), RepresentationInput::from(&rows))?;
    let request = match input {
        Input::UnprovedDense => ViewRequest::Membership {
            column: ColumnIndex(0),
        },
        _ => ViewRequest::PartitionOrder {
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
        Input::ForeignOwner => {
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
                .map(|_| ())
        }
        Input::ExpiredAdmission => {
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
                .map(|_| ())
        }
        Input::ForeignDescriptor => owner
            .derived()
            .acquire_stored(
                &Arc::new((*plan).clone()),
                &admitted,
                Default::default(),
                DerivedInputs::Rows(&original),
                &mut scratch,
                None,
            )
            .map(|_| ()),
        Input::SubstitutedRows => owner
            .derived()
            .acquire_stored(
                &plan,
                &admitted,
                Default::default(),
                DerivedInputs::Rows(&[RepresentationInput::Computed(&table())]),
                &mut scratch,
                None,
            )
            .map(|_| ()),
        Input::UnprovedDense => owner
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
            .map(|_| ()),
        Input::CloneKeepsLease => {
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
            assert!(held > before, "an acquired view holds a required lease");
            let clone = first.clone();
            drop(first);
            assert_eq!(required(), held, "a clone alone keeps the lease");
            clone.view()?;
            drop(clone);
            assert_eq!(required(), before, "the last drop releases the lease");
            Ok(())
        }
        Input::ExpiredView | Input::Reuse => {
            let first: AcquiredView = owner.derived().acquire_stored(
                &plan,
                &admitted,
                Default::default(),
                DerivedInputs::Rows(&original),
                &mut scratch,
                None,
            )?;
            if matches!(input, Input::ExpiredView) {
                owner.release_population();
                return first.view().map(|_| ());
            }
            let second = owner.derived().acquire_stored(
                &plan,
                &admitted,
                Default::default(),
                DerivedInputs::Rows(&original),
                &mut scratch,
                None,
            )?;
            assert_eq!(first.storage_identity(), second.storage_identity());
            assert_eq!(second.construction(), Construction::Reused);
            let ryo_derived::domain::contract::types::DerivedView::PartitionOrder(order) =
                first.view()?
            else {
                return Err(OperatingError::WrongOperation);
            };
            // Keys 2, 1, 2: key 2 hashes below key 1 under the derived key hash (one `FxHasher` step), so its rows come first.
            assert_eq!(order.iter().map(|row| row.0).collect::<Vec<_>>(), [0, 2, 1]);
            Ok(())
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
