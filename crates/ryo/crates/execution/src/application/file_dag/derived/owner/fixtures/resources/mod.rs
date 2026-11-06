//! Pure derived-owner fixtures; no operator evaluator or legacy transport.
use crate::application::file_dag::derived::owner::*;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::{
    coverage::types::Partial, operating::errors::OperatingError,
    operator::contract::views::BorrowedRows, table_model::types::Rows,
};
use ryo_derived::domain::contract::types::KeyIndex;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::{plan_parts::types::ColumnIndex, plans::types::Plan};
use std::rc::Rc;
#[derive(Clone, Copy, Debug)]
pub(crate) struct Snapshot {
    pub graph_builds: u64,
    pub graph_reuses: u64,
    pub graph_early_stops: u64,
}
pub(crate) struct ResourceFixture {
    owner: Rc<RelationOwner>,
}
impl ResourceFixture {
    pub(crate) fn new(wanted: &[Relation], partials: &[Partial], limit: usize) -> Option<Self> {
        let owner = Rc::new(RelationOwner::new(
            true,
            crate::application::file_dag::testing::program_derived_port(),
        ));
        let plans = wanted
            .iter()
            .copied()
            .map(crate::application::file_dag::derived::owner::fixtures::source_plan)
            .collect::<Vec<_>>();
        owner
            .derived
            .prepare_program_file(
                &crate::application::file_dag::derived::owner::fixtures::prepared_sources(
                    &plans,
                    Arc::clone(&owner.derived.semantic),
                ),
                &(0..plans.len())
                    .map(crate::domain::coverage::types::RuleIndex)
                    .collect::<Vec<_>>(),
                partials,
            )
            .unwrap();
        if !owner.derived.index_observation().available {
            return None;
        }
        owner.derived.set_optional_limit(limit).unwrap();
        Some(Self { owner })
    }
    pub(crate) fn owner(&self) -> &Rc<RelationOwner> {
        &self.owner
    }
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
            Arc::clone(&self.owner.derived.semantic),
            ViewRequest::Equality {
                columns: columns.to_vec(),
            },
            rows,
        );
        let proof = self
            .owner
            .admit_relation(relation, BorrowedRows::from(rows))?;
        let Rows::Stored(source) = rows else {
            return Err(OperatingError::WrongOperation);
        };
        self.owner.derived.acquire_stored(
            &plan,
            &self
                .owner
                .derived
                .admit_view(&plan, &self.owner, &[&proof])
                .unwrap(),
            ViewContext::default(),
            DerivedInputs::Rows(&[proof.rows()]),
            &mut DerivedScratch::default(),
            Some((relation, *source)),
        )
    }
    pub(crate) fn no_invocation_index_handles(&self) -> bool {
        self.owner.derived.no_invocation_index_handles()
    }
    pub(crate) fn shared_index_backings(&self) -> usize {
        self.owner.derived.shared_index_backings()
    }
    pub(crate) fn first_index_weak(&self) -> Option<Weak<KeyIndex>> {
        self.owner.derived.first_index_weak()
    }
}
/// Observe backing lifetime without retaining a lease or exposing mutable storage.
pub(crate) fn weak_storage(
    view: &crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
) -> Weak<
    crate::application::file_dag::derived::owner::acquisition::semantics::service::DerivedStorage,
> {
    Arc::downgrade(view.storage())
}

/// Passive observation of actual portable builds; every call delegates to the
/// production derived use case. No evaluator, scheduling or owner lives here.
#[derive(Debug, Default)]
pub(in crate::application::file_dag) struct RecordedDerived {
    builds: std::sync::Mutex<Vec<ryo_derived::domain::contract::types::ViewRequest>>,
    raw_failure: Option<ryo_derived::domain::contract::types::DerivedFault>,
}
impl RecordedDerived {
    pub(in crate::application::file_dag) fn refusing_raw(
        fault: ryo_derived::domain::contract::types::DerivedFault,
    ) -> Self {
        Self {
            raw_failure: Some(fault),
            ..Self::default()
        }
    }

    pub(in crate::application::file_dag) fn builds(
        &self,
    ) -> Vec<ryo_derived::domain::contract::types::ViewRequest> {
        self.builds.lock().expect("recorded semantic calls").clone()
    }
}
impl ryo_derived::inbound::use_cases::derived::interface::DerivedUseCase for RecordedDerived {
    fn prepare(
        &self,
        request: ryo_derived::domain::contract::types::ViewRequest,
        schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>,
    ) -> Arc<ryo_derived::domain::contract::types::PreparedDerived> {
        use ryo_derived::inbound::use_cases::derived::{
            interface::DerivedUseCase, use_case::DerivedUseCaseImpl,
        };
        DerivedUseCaseImpl::new().prepare(request, schemas)
    }
    fn build(
        &self,
        prepared: &ryo_derived::domain::contract::types::PreparedDerived,
        inputs: ryo_derived::domain::contract::types::DerivedInputs<'_>,
        room: &mut ryo_derived::domain::contract::types::BuildWorkspace,
        recycled: Option<ryo_derived::domain::contract::types::DerivedPayload>,
    ) -> Result<
        ryo_derived::domain::contract::types::BuildOutcome,
        ryo_derived::domain::contract::types::DerivedFault,
    > {
        use ryo_derived::inbound::use_cases::derived::{
            interface::DerivedUseCase, use_case::DerivedUseCaseImpl,
        };
        self.builds
            .lock()
            .expect("recorded semantic calls")
            .push(prepared.request().clone());
        if matches!(
            prepared.request(),
            ryo_derived::domain::contract::types::ViewRequest::RawKeyOrder { .. }
        ) && let Some(fault) = &self.raw_failure
        {
            return Err(fault.clone());
        }
        DerivedUseCaseImpl::new().build(prepared, inputs, room, recycled)
    }
    fn view<'a>(
        &self,
        prepared: &ryo_derived::domain::contract::types::PreparedDerived,
        payload: &'a ryo_derived::domain::contract::types::DerivedPayload,
    ) -> Result<
        ryo_derived::domain::contract::types::DerivedView<'a>,
        ryo_derived::domain::contract::types::DerivedFault,
    > {
        use ryo_derived::inbound::use_cases::derived::{
            interface::DerivedUseCase, use_case::DerivedUseCaseImpl,
        };
        DerivedUseCaseImpl::new().view(prepared, payload)
    }
}
