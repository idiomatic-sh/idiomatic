//! The exact lease getter must consume the registered borrowed view without copying storage.
use crate::application::file_dag::invocation::service::InvocationResources;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::contract::types::DerivedView;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;
use {
    ryo_execution_domain::domain::derived_views::types::*,
    ryo_execution_domain::domain::operation_schema::types::InputShape,
    ryo_values::domain::rows::service::push_row,
};
fn production_cycle_lease_borrows_actual_trait_output_and_refuses_foreign_storage() {
    let mut table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Default::default()
    };
    push_row(
        &mut table,
        &[Cell::Number(Integer(1)), Cell::Number(Integer(2))],
    );
    let plan = Arc::new(
        ryo_execution_store::application::file_dag::derived::prepare::service::view(
            (Arc::new(ryo_execution_store::application::file_dag::fixtures::derived())
                as Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort>),
            &ViewRequest::CycleAdjacency {
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
            },
            &[InputShape {
                kinds: &table.kinds,
                tree: None,
            }],
        )
        .unwrap(),
    );
    let owner = ryo_execution_store::application::file_dag::relations::service::RelationOwner::fixture(
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        (Arc::new(ryo_execution_store::application::file_dag::fixtures::derived())
            as Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort>),
    );
    let publication = owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Computed(table.clone())).unwrap();
    let held = owner
        .rows(&publication, &|_| Err(OperatingError::NotRead))
        .unwrap();
    let rows = RepresentationInput::from(&held);
    let proof = owner
        .admit(&publication, rows)
        .and_then(|admitted| admitted)
        .unwrap();
    let acquired = owner
        .derived()
        .acquire_stored(
            &plan,
            &owner
                .derived()
                .admit_view(&plan, &owner, &[&proof])
                .unwrap(),
            ryo_execution_store::application::file_dag::derived::owner::service::ViewContext::default(),
            DerivedInputs::Rows(&[rows]),
            &mut DerivedScratch::default(),
            None,
        )
        .unwrap();
    let DerivedView::CycleAdjacency(built) = acquired.view().unwrap() else {
        panic!("actual cycle builder");
    };
    let pointer = built.values.as_ptr();
    let graph = InvocationResources::cycles_view(Some(&acquired))
        .unwrap()
        .unwrap();
    assert_eq!(
        graph.values,
        [Cell::Number(Integer(1)), Cell::Number(Integer(2))]
    );
    assert_eq!(
        pointer,
        graph.values.as_ptr(),
        "the semantic lease borrows the owned graph"
    );
    let equality = Arc::new(
        ryo_execution_store::application::file_dag::derived::prepare::service::view(
            (Arc::new(ryo_execution_store::application::file_dag::fixtures::derived())
                as Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort>),
            &ViewRequest::Equality {
                columns: vec![ColumnIndex(0)],
            },
            &[InputShape {
                kinds: &table.kinds,
                tree: None,
            }],
        )
        .unwrap(),
    );
    let wrong = owner
        .derived()
        .acquire_stored(
            &equality,
            &owner
                .derived()
                .admit_view(&equality, &owner, &[&proof])
                .unwrap(),
            ryo_execution_store::application::file_dag::derived::owner::service::ViewContext::default(),
            DerivedInputs::Rows(&[rows]),
            &mut DerivedScratch::default(),
            None,
        )
        .unwrap();
    assert!(matches!(
        InvocationResources::cycles_view(Some(&wrong)),
        Err(OperatingError::WrongOperation)
    ));
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ProductionCycleLeaseBorrowsActualTraitOutputAndRefusesForeignStorage,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName(
        "production_cycle_lease_borrows_actual_trait_output_and_refuses_foreign_storage",
    ),
    word: Word::Succeeds,
    input: || Scenario::ProductionCycleLeaseBorrowsActualTraitOutputAndRefusesForeignStorage,
    expect: Expect::Value(()),
}];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ProductionCycleLeaseBorrowsActualTraitOutputAndRefusesForeignStorage => {
            production_cycle_lease_borrows_actual_trait_output_and_refuses_foreign_storage();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
