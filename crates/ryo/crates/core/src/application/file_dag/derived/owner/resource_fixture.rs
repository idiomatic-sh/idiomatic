//! Read-only observations around actual registered collected execution and its owner.
use super::*;
use crate::application::file_dag::{execution::CommonOperatorEngine, relations::RelationOwner};
use crate::domain::{
    coverage::types::Partial,
    operating::types::OperatorContext,
    operation_schema::types::InputShape,
    operator::{service::registered, types::PreparedPool},
    table_model::{service::kinds_of, types::Rows},
};
use ryo_derived::domain::contract::types::KeyIndex;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, StepId},
    plans::types::{Emit, Operation, Plan, PlanStep, StepScope},
};
use std::rc::Rc;
#[derive(Clone, Copy, Debug)]
pub(crate) struct Snapshot {
    pub builds: u64,
    pub reuses: u64,
    pub entries: usize,
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
            crate::outbound::derived::factory::test_port(),
        ));
        owner.derived.prepare_file(
            true,
            wanted,
            partials,
            &[],
            &[],
            &PreparedPool::default(),
            |_| true,
        );
        if !owner.derived.index_observation().available {
            return None;
        }
        owner
            .derived
            .set_optional_limit(limit)
            .expect("installed fixture backend accepts configured budget");
        Some(Self { owner })
    }
    pub(crate) fn owner(&self) -> &Rc<RelationOwner> {
        &self.owner
    }
    pub(crate) fn collect(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        self.collect_with(
            crate::outbound::operators::factory::test_port(),
            operation,
            inputs,
            context,
            out,
        )
    }
    pub(crate) fn collect_with(
        &self,
        operators: &dyn crate::outbound::operators::interface::OperatorsPort,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let plan = Plan {
            observation_inputs: Vec::new(),
            observations: Vec::new(),
            parameters: vec![],
            steps: vec![PlanStep {
                operation: operation.clone(),
                scope: StepScope::PerFile,
            }],
            emit: Emit {
                step: StepId(0),
                subject: ColumnIndex(0),
                key: vec![],
                headings: vec![],
            },
            frontier: vec![],
            comparison_origins: vec![],
        };
        if let Some(cache) = self.owner.derived.backend.borrow_mut().as_mut() {
            super::super::graph_cache::service::reserve(
                cache,
                &[plan],
                &PreparedPool::default(),
                |_| true,
            );
        }
        let schemas = inputs
            .iter()
            .map(|rows| InputShape {
                kinds: kinds_of(rows),
                tree: if let Rows::Tree(tree) = rows {
                    Some(tree.form)
                } else {
                    None
                },
            })
            .collect::<Vec<_>>();
        let contract = super::super::prepare::operator(
            registered(crate::domain::features::service::kind_of(operation)),
            operation,
            &schemas,
            context.bindings,
        )?;
        let engine = CommonOperatorEngine::new(operators, self.owner.derived.semantic_port());
        crate::application::file_dag::executor::ScopedExecutor::new(
            &engine,
            Rc::clone(&self.owner),
        )?
        .collect(&contract, inputs, context, out)
    }
    pub(crate) fn snapshot(&self) -> Snapshot {
        let cache = self.owner.derived.backend.borrow();
        let cache = cache.as_ref().expect("fixture installed actual backend");
        Snapshot {
            builds: cache.builds,
            reuses: cache.reuses,
            entries: self.owner.derived.retained_index_count(),
            graph_builds: cache.graph_counts.builds,
            graph_reuses: cache.graph_counts.reuses,
            graph_early_stops: cache.graph_counts.early_stops,
        }
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
    pub(crate) fn first_index_owners(&self) -> Option<usize> {
        self.owner.derived.first_index_owners()
    }
}

struct OwnedFeed<'a, 'f> {
    inner: &'a mut dyn crate::application::file_dag::feed::Feed<'f>,
    inputs: &'a [Rows<'f>],
    logical: Vec<ryo_values::domain::plan_parts::types::Input>,
    owner: Rc<RelationOwner>,
}
impl<'f> crate::application::file_dag::feed::Feed<'f> for OwnedFeed<'_, 'f> {
    fn relation_owner(&self) -> Rc<RelationOwner> {
        Rc::clone(&self.owner)
    }
    fn publication(
        &self,
        place: crate::domain::operating::types::InputPlace,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, OperatingError>
    {
        self.inner.publication(place)
    }
    fn origin(
        &self,
        place: crate::domain::operating::types::InputPlace,
    ) -> Result<crate::application::file_dag::feed::InputOrigin, OperatingError> {
        use crate::application::file_dag::feed::InputOrigin;
        let rows = self
            .inputs
            .get(place.0)
            .ok_or(OperatingError::MissingInput {
                position: place.0 + 1,
            })?;
        Ok(match (self.logical.get(place.0), rows) {
            (
                Some(ryo_values::domain::plan_parts::types::Input::Relation(relation)),
                Rows::Stored(_) | Rows::Selected(_) | Rows::Tree(_),
            ) => InputOrigin::Relation(*relation),
            _ => return self.inner.origin(place),
        })
    }
    fn kinds(
        &self,
        place: crate::domain::operating::types::InputPlace,
    ) -> Result<&[ryo_relations::domain::relations::types::CellKind], OperatingError> {
        self.inner.kinds(place)
    }
    fn streams(
        &self,
        place: crate::domain::operating::types::InputPlace,
    ) -> Result<crate::domain::operating::types::Supply, OperatingError> {
        self.inner.streams(place)
    }
    fn native_join_right_first(&self) -> bool {
        self.inner.native_join_right_first()
    }
    fn found(
        &mut self,
        place: crate::domain::operating::types::InputPlace,
        rows: ryo_values::domain::rows::types::RowCount,
    ) {
        self.inner.found(place, rows);
    }
    fn stream(
        &mut self,
        place: crate::domain::operating::types::InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn crate::domain::operating::types::Emit,
    ) -> Result<(), OperatingError> {
        self.inner.stream(place, context, emit)
    }
    fn hold<'t>(
        &mut self,
        place: crate::domain::operating::types::InputPlace,
        context: &mut OperatorContext<'_>,
        table: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'f: 't,
    {
        self.inner.hold(place, context, table)
    }
}
impl ResourceFixture {
    pub(crate) fn stream<'f>(
        &self,
        operation: &Operation,
        inputs: &[Rows<'f>],
        feed: &mut dyn crate::application::file_dag::feed::Feed<'f>,
        context: &mut OperatorContext<'_>,
        room: &mut crate::domain::operating::types::Room,
        emit: &mut dyn crate::domain::operating::types::Emit,
    ) -> Result<(), OperatingError> {
        let schemas = inputs
            .iter()
            .map(|rows| InputShape {
                kinds: kinds_of(rows),
                tree: if let Rows::Tree(tree) = rows {
                    Some(tree.form)
                } else {
                    None
                },
            })
            .collect::<Vec<_>>();
        let contract = super::super::prepare::operator(
            registered(crate::domain::features::service::kind_of(operation)),
            operation,
            &schemas,
            context.bindings,
        )?;
        self.stream_prepared(operation, &contract, inputs, feed, context, room, emit)
    }
    pub(crate) fn stream_prepared<'f>(
        &self,
        operation: &Operation,
        contract: &crate::domain::operator::contract::types::OperatorContract,
        inputs: &[Rows<'f>],
        feed: &mut dyn crate::application::file_dag::feed::Feed<'f>,
        context: &mut OperatorContext<'_>,
        room: &mut crate::domain::operating::types::Room,
        emit: &mut dyn crate::domain::operating::types::Emit,
    ) -> Result<(), OperatingError> {
        let plan = Plan {
            observation_inputs: Vec::new(),
            observations: Vec::new(),
            parameters: vec![],
            steps: vec![PlanStep {
                operation: operation.clone(),
                scope: StepScope::PerFile,
            }],
            emit: Emit {
                step: StepId(0),
                subject: ColumnIndex(0),
                key: vec![],
                headings: vec![],
            },
            frontier: vec![],
            comparison_origins: vec![],
        };
        if let Some(cache) = self.owner.derived.backend.borrow_mut().as_mut() {
            super::super::graph_cache::service::reserve(
                cache,
                &[plan],
                &PreparedPool::default(),
                |_| true,
            );
        }
        if !Rc::ptr_eq(&self.owner, &feed.relation_owner()) {
            return Err(OperatingError::NotRead);
        }
        let mut feed = OwnedFeed {
            inner: feed,
            inputs,
            logical: ryo_values::domain::plans::service::named(&contract.inputs).to_vec(),
            owner: Rc::clone(&self.owner),
        };
        let engine = CommonOperatorEngine::new(
            crate::outbound::operators::factory::test_port(),
            self.owner.derived.semantic_port(),
        );
        crate::application::file_dag::executor::ScopedExecutor::new(
            &engine,
            Rc::clone(&self.owner),
        )?
        .stream(contract, &mut feed, context, room, emit)
    }
}

/// Prepare the actual request graph and its required file execution owner.
pub(crate) fn prepared_scope<'a>(
    plans: &[Plan],
    request: &crate::domain::operator::contract::types::RequestOperators,
    prepared: &PreparedPool,
    wanted: &[Relation],
    partials: &[Partial],
    executor: &'a dyn crate::application::file_dag::executor::PrimitiveExecutor,
) -> Result<crate::application::file_dag::testing::FileScope<'a>, OperatingError> {
    let graph = crate::domain::file_program::service::compile_prepared(plans, request);
    let scope = crate::application::file_dag::testing::FileScope::new(
        Arc::new(graph),
        executor,
        |_| true,
        wanted,
        partials,
    )?;
    scope.execution.borrow().prepare_resources(
        true,
        wanted,
        partials,
        wanted,
        plans,
        prepared,
        |_| true,
    );
    Ok(scope)
}

use ryo_values::domain::rows::types::Table;

/// Observe backing lifetime without retaining its lease or exposing mutable storage.
pub(crate) fn weak_storage(
    view: &super::super::view::AcquiredView,
) -> Weak<super::super::storage::DerivedStorage> {
    Arc::downgrade(&view.storage)
}
