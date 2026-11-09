//! Pure derived-owner fixtures; no operator evaluator or legacy transport.
#[cfg(test)]
use crate::application::file_dag::derived::owner::*;
#[cfg(test)]
use crate::application::file_dag::relations::RelationOwner;
use crate::outbound::derived::interface::DerivedPort;
#[cfg(test)]
use ryo_derived::domain::equality::types::KeyIndex;
#[cfg(test)]
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
#[cfg(test)]
use ryo_execution_domain::domain::{
    coverage::types::Partial, operating::errors::OperatingError, table_model::types::Rows,
};
#[cfg(test)]
use ryo_relations::domain::relations::types::Relation;
#[cfg(test)]
use ryo_values::domain::plan_parts::types::ColumnIndex;
#[cfg(test)]
use std::rc::Rc;
/// What a resource fixture's owner holds and counted.
#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub graph_builds: u64,
    pub graph_reuses: u64,
    pub graph_early_stops: u64,
}
/// A relation owner over a recording port, with the resource acquisitions a proof drives.
#[cfg(test)]
pub(crate) struct ResourceFixture {
    owner: Rc<RelationOwner>,
}
#[cfg(test)]
impl ResourceFixture {
    /// A fixture over the wanted relations and partials, with the retained-byte limit; `None` when the owner declines.
    pub(crate) fn new(wanted: &[Relation], partials: &[Partial], limit: usize) -> Option<Self> {
        let owner = Rc::new(RelationOwner::new(
            crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
            crate::application::file_dag::fixtures::program_derived_port(),
        ));
        let needs = wanted
            .iter()
            .copied()
            .map(crate::application::file_dag::derived::owner::fixtures::reads)
            .collect::<Box<[_]>>();
        let rules = needs.len();
        owner
            .derived()
            .prepare_program_file(
                &crate::application::file_dag::derived::prepared::RequestDerived::new(needs),
                &(0..rules)
                    .map(ryo_execution_domain::domain::coverage::types::RuleIndex)
                    .collect::<Vec<_>>(),
                partials,
            )
            .unwrap();
        if !owner.derived().index_observation().available {
            return None;
        }
        owner
            .derived()
            .set_optional_limit(
                crate::application::file_dag::derived::owner::index::types::ByteLimit(limit),
            )
            .unwrap();
        Some(Self { owner })
    }
    /// The fixture's relation owner.
    pub(crate) fn owner(&self) -> &Rc<RelationOwner> {
        &self.owner
    }
    /// Acquires the key index of `rows` over `columns`.
    pub(crate) fn index(
        &self,
        relation: Relation,
        rows: &Rows<'_>,
        columns: &[ColumnIndex],
    ) -> Result<
        crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
        OperatingError,
    > {
        let plan = crate::application::file_dag::derived::owner::fixtures::prepare_view(
            self.owner.derived().semantic.clone(),
            ViewRequest::Equality {
                columns: columns.to_vec(),
            },
            rows,
        );
        let proof = self
            .owner
            .admit_relation(relation, RepresentationInput::from(rows))?;
        let Rows::Stored(source) = rows else {
            return Err(OperatingError::WrongOperation);
        };
        self.owner.derived().acquire_stored(
            &plan,
            &self
                .owner
                .derived()
                .admit_view(&plan, &self.owner, &[&proof])
                .unwrap(),
            ViewContext::default(),
            DerivedInputs::Rows(&[proof.rows()]),
            &mut DerivedScratch::default(),
            Some(crate::application::file_dag::derived::owner::acquisition::population::types::StoredColumns {
                relation,
            }),
        )
    }
    /// Whether no invocation holds an index handle.
    pub(crate) fn no_invocation_index_handles(&self) -> bool {
        self.owner.derived().no_invocation_index_handles()
    }
    /// How many index backings are shared.
    pub(crate) fn shared_index_backings(&self) -> usize {
        self.owner.derived().shared_index_backings()
    }
    /// A weak reference to the first retained index, to observe its release.
    pub(crate) fn first_index_weak(&self) -> Option<Weak<KeyIndex>> {
        self.owner.derived().first_index_weak()
    }
}
/// Observe backing lifetime without retaining a lease or exposing mutable storage.
#[cfg(test)]
pub(crate) fn weak_storage(
    view: &crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
) -> Weak<
    crate::application::file_dag::derived::owner::acquisition::semantics::storage::DerivedStorage,
> {
    Arc::downgrade(view.storage())
}

/// Passive observation of actual portable builds; every call delegates to the
/// production derived use case. No evaluator, scheduling or owner lives here.
#[derive(Debug, Default)]
pub struct RecordedDerived {
    builds: std::sync::Mutex<Vec<ryo_derived::domain::request::types::DerivedRequest>>,
}
impl RecordedDerived {
    /// The requests the port was asked to build, in order.
    pub fn builds(&self) -> Vec<ryo_derived::domain::request::types::DerivedRequest> {
        self.builds.lock().expect("recorded semantic calls").clone()
    }
}
impl DerivedPort for RecordedDerived {
    fn prepare(
        &self,
        request: ryo_derived::domain::request::types::DerivedRequest,
        schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>,
    ) -> ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived {
        crate::application::file_dag::fixtures::derived().prepare(request, schemas)
    }
    fn build(
        &self,
        prepared: &ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived,
        inputs: ryo_derived::domain::contract::types::BuildInputs<'_>,
        room: &mut ryo_derived::domain::contract::types::BuildWorkspace,
        recycled: Option<
            ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload,
        >,
    ) -> Result<
        ryo_derived::domain::contract::types::BuildOutcome,
        ryo_derived::domain::fault::types::DerivedFault,
    > {
        self.builds
            .lock()
            .expect("recorded semantic calls")
            .push(prepared.request().clone());
        crate::application::file_dag::fixtures::derived().build(prepared, inputs, room, recycled)
    }
    fn view<'a>(
        &self,
        prepared: &ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived,
        payload: &'a ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload,
    ) -> Result<
        ryo_derived::domain::contract::types::DerivedView<'a>,
        ryo_derived::domain::fault::types::DerivedFault,
    > {
        crate::application::file_dag::fixtures::derived().view(prepared, payload)
    }
}
