//! The exact lease getter must consume the registered borrowed view without copying storage.
use crate::application::file_dag::invocation::types::InvocationResources;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::contract::types::DerivedView;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;
use std::sync::Arc;
use {
    ryo_execution_domain::domain::derived_views::types::*,
    ryo_execution_domain::domain::operation_schema::types::InputShape,
    ryo_values::domain::rows::service::push_row,
};
/// What the cycle lease answered: its entity values, and whether it borrows the acquired view's storage.
#[derive(Debug, PartialEq)]
struct Lease {
    values: Cow<'static, [i64]>,
    borrowed: bool,
}

/// The request of the view acquired over the one-row edge table `(1, 2)`.
type Input = ViewRequest;
type Value = Lease;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "production_cycle_lease_borrows_actual_trait_output_and_refuses_foreign_storage",
        ),
        word: Word::Succeeds,
        input: || ViewRequest::CycleAdjacency {
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        expect: Expect::Value(Lease {
            values: Cow::Borrowed(&[1, 2]),
            borrowed: true,
        }),
    },
    Case {
        name: CaseName("a_cycle_lease_of_an_equality_view_is_refused"),
        word: Word::Refuses,
        input: || ViewRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        expect: Expect::Error(OperatingError::WrongOperation),
    },
];

/// Acquires the requested view over the edge table and takes the cycle lease of it.
fn subject(request: Input) -> Result<Value, Failure> {
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
            Arc::new(ryo_execution_store::application::file_dag::fixtures::derived()),
            &request,
            &[InputShape {
                kinds: &table.kinds,
                tree: None,
            }],
        )?,
    );
    let owner = ryo_execution_store::application::file_dag::relations::service::RelationOwner::fixture(
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        Arc::new(ryo_execution_store::application::file_dag::fixtures::derived()),
    );
    let publication = owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Computed(table.clone()))?;
    let held = owner.rows(&publication, &|_| Err(OperatingError::NotRead))?;
    let rows = RepresentationInput::from(&held);
    let proof = owner
        .admit(&publication, rows)
        .and_then(|admitted| admitted)?;
    let acquired = owner.derived().acquire_stored(
        &plan,
        &owner.derived().admit_view(&plan, &owner, &[&proof])?,
        ryo_execution_store::application::file_dag::derived::owner::service::ViewContext::default(),
        DerivedInputs::Rows(&[rows]),
        &mut DerivedScratch::default(),
        None,
    )?;
    let built = match acquired.view()? {
        DerivedView::CycleAdjacency(built) => Some(built.values.as_ptr()),
        _ => None,
    };
    let Some(graph) = InvocationResources::cycles_view(Some(&acquired))? else {
        return Err(OperatingError::NotRead);
    };
    Ok(Lease {
        values: graph
            .values
            .iter()
            .map(|cell| match cell {
                Cell::Number(Integer(value)) => *value,
                other => panic!("a number cell, not {other:?}"),
            })
            .collect(),
        borrowed: built == Some(graph.values.as_ptr()),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
