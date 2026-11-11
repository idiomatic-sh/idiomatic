//! The owner, its sources, plans and populations that the owner's case groups share.
use super::super::*;
use crate::application::file_dag::derived::owner::index::service as indexes;
use crate::application::file_dag::derived::owner::index::types::ByteLimit;
use crate::application::file_dag::relations::{RelationOwner, handle::service::PublishedInput};
use ryo_derived::domain::contract::types::DerivedView;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::Table;
use {
    ryo_execution_domain::domain::operation_schema::types::InputShape,
    ryo_values::domain::rows::service::push_row,
};

pub(super) fn source() -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    for value in [3, 1, 3] {
        push_row(&mut table, &[Value::Number(Integer(value))]);
    }
    table
}

pub(super) fn plan() -> Arc<DerivedContract> {
    Arc::new(
        crate::application::file_dag::derived::prepare::view(
            std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
            &ViewRequest::PartitionOrder {
                keys: vec![ColumnIndex(0)],
            },
            &[InputShape {
                kinds: &[CellKind::Number],
                tree: None,
            }],
        )
        .unwrap(),
    )
}

pub(super) fn owner(entries: EntryLimit) -> RelationOwner {
    let mut owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        crate::application::file_dag::fixtures::program_derived_port(),
    );
    owner.derived_mut().max_entries = entries;
    owner
}

pub(super) fn population(owner: &RelationOwner, publication: &PublishedInput) -> PopulationId {
    let held = owner
        .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    owner
        .admit(publication.as_ref(), RepresentationInput::from(&held))
        .unwrap()
        .identity(owner)
        .unwrap()
        .population
}

pub(super) fn pair_plan() -> Arc<DerivedContract> {
    Arc::new(
        crate::application::file_dag::derived::prepare::view(
            crate::application::file_dag::fixtures::program_derived_port(),
            &ViewRequest::OrderedKeys {
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                left_order: ColumnIndex(0),
                right_order: ColumnIndex(0),
            },
            &[
                InputShape {
                    kinds: &[CellKind::Number],
                    tree: None,
                },
                InputShape {
                    kinds: &[CellKind::Number],
                    tree: None,
                },
            ],
        )
        .unwrap(),
    )
}

pub(super) fn acquire_pair(
    owner: &RelationOwner,
    plan: &Arc<DerivedContract>,
    publications: &[PublishedInput; 2],
    scratch: &mut DerivedScratch,
) -> acquisition::admission::service::AcquiredView {
    let left = owner
        .resolve(publications[0].as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let right = owner
        .resolve(publications[1].as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let left = owner
        .admit(publications[0].as_ref(), RepresentationInput::from(&left))
        .unwrap();
    let right = owner
        .admit(publications[1].as_ref(), RepresentationInput::from(&right))
        .unwrap();
    let admitted = owner
        .derived()
        .admit_view(plan, owner, &[&left, &right])
        .unwrap();
    let symbols = ryo_language::domain::symbols::types::Symbols::default();
    let texts = ryo_execution_domain::domain::table_texts::types::Texts {
        symbols: &symbols,
        source: ryo_values::domain::files::types::SourceView(&[]),
        paths: &[],
        arenas: &[],
    };
    owner
        .derived()
        .acquire_stored(
            plan,
            &admitted,
            ViewContext::default(),
            DerivedInputs::Text {
                rows: &[left.rows(), right.rows()],
                texts: &texts,
            },
            scratch,
            None,
        )
        .unwrap()
}

pub(super) fn acquire(
    owner: &RelationOwner,
    plan: &Arc<DerivedContract>,
    publication: &PublishedInput,
    context: ViewContext,
    scratch: &mut DerivedScratch,
) -> acquisition::admission::service::AcquiredView {
    let held = owner
        .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let proof = owner
        .admit(publication.as_ref(), RepresentationInput::from(&held))
        .unwrap();
    let admitted = owner.derived().admit_view(plan, owner, &[&proof]).unwrap();
    owner
        .derived()
        .acquire_stored(
            plan,
            &admitted,
            context,
            DerivedInputs::Rows(&[proof.rows()]),
            scratch,
            None,
        )
        .unwrap()
}
