//! Test setup uses the production owner, original-row admission and physical dispatch.
use super::super::graph_cache::{
    service::slot,
    types::{
        handles::{DomainHandle, GraphHandle, RootAnswer},
        roots::value::DemandAnswers,
    },
};
use super::*;
use crate::application::file_dag::{relations::RelationOwner, workspace::InvocationWorkspace};
use crate::domain::{
    coverage::types::Partial,
    operation_schema::types::InputShape,
    operator::contract::views::BorrowedRows,
    stored_graph::types::{GraphCounts, GraphWalk, Orientation},
    table_model::{service::kinds_of, types::Rows},
};
use ryo_relations::domain::relations::types::{Relation, Value};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Direction, Input, StepId},
    plans::types::{Emit, Operation, Plan, PlanStep, StepScope},
};
use std::rc::Rc;

pub(in crate::application::file_dag) struct GraphFixture {
    owner: Rc<RelationOwner>,
    wanted: Vec<Relation>,
    partials: Vec<Partial>,
    limit: usize,
}
pub(in crate::application::file_dag) struct Snapshot {
    pub counts: GraphCounts,
    pub bytes: usize,
    pub limit: usize,
    pub memo_bytes: usize,
    pub graphs: usize,
    pub domains: usize,
    pub nearest: usize,
    pub indexes: usize,
    pub owner_entries_capacity: usize,
    pub index_payloads: Vec<usize>,
    pub index_columns: Vec<Vec<ColumnIndex>>,
    pub required_bytes: usize,
    pub optional_bytes: usize,
    pub unique_bytes: usize,
    pub overlap_bytes: usize,
    pub graph_backings: usize,
    pub retained_view_bytes: usize,
    pub selections: usize,
    pub retained_positions: usize,
}
impl GraphFixture {
    pub(in crate::application::file_dag) fn new(
        wanted: &[Relation],
        partials: &[Partial],
        limit: usize,
    ) -> Self {
        Self::with_identity(wanted, partials, limit, true, 1024)
    }
    pub(in crate::application::file_dag) fn with_identity(
        wanted: &[Relation],
        partials: &[Partial],
        limit: usize,
        stable: bool,
        max_views: usize,
    ) -> Self {
        let fixture = Self {
            owner: Rc::new(RelationOwner::for_request(
                stable,
                max_views,
                (std::sync::Arc::new(
                    ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
                )
                    as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
            )),
            wanted: wanted.to_vec(),
            partials: partials.to_vec(),
            limit,
        };
        fixture.setup();
        fixture
    }
    fn request(&self) -> Plan {
        let relation = self.wanted.first().copied().unwrap_or(Relation::Function);
        // Real request eligibility, rather than directly toggling graph_enabled.
        Plan {
            observation_inputs: Vec::new(),
            observations: Vec::new(),
            parameters: vec![],
            steps: vec![PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Axis {
                    from: Input::Relation(relation),
                    edges: Input::Relation(relation),
                    domain: Input::Relation(relation),
                    direction: Direction::Descendant,
                    id: ColumnIndex(0),
                    parent: ColumnIndex(0),
                    child: ColumnIndex(1),
                    domain_id: ColumnIndex(0),
                    domain_filter: vec![],
                    domain_view: vec![],
                    output: vec![],
                },
            }],
            emit: Emit {
                step: StepId(0),
                subject: ColumnIndex(0),
                key: vec![],
                headings: vec![],
            },
            frontier: vec![],
            comparison_origins: vec![],
        }
    }
    fn setup(&self) {
        let plan = self.request();
        self.owner.derived.prepare_file(
            true,
            &self.wanted,
            &self.partials,
            &[],
            &[plan],
            &crate::domain::operator::types::PreparedPool::default(),
            |_| true,
        );
        self.owner.derived.set_optional_limit(self.limit).unwrap();
    }
    /// Attach observation/policy access to an existing real selection owner;
    /// request declaration enables graph demand without replacing its backend.
    pub(in crate::application::file_dag) fn attach(
        owner: Rc<RelationOwner>,
        wanted: &[Relation],
    ) -> Self {
        let limit = owner.derived.backend.borrow().as_ref().unwrap().limit;
        let fixture = Self {
            owner,
            wanted: wanted.to_vec(),
            partials: vec![],
            limit,
        };
        let plan = fixture.request();
        super::super::graph_cache::service::reserve(
            fixture.owner.derived.backend.borrow_mut().as_mut().unwrap(),
            &[plan],
            &crate::domain::operator::types::PreparedPool::default(),
            |_| true,
        );
        fixture
    }
    fn workspace(&self, input: Input, rows: &Rows<'_>) -> InvocationWorkspace {
        let mut workspace = InvocationWorkspace::new(Rc::clone(&self.owner));
        workspace.capture_inputs(&[input], &[rows.clone()]).unwrap();
        workspace
    }
    pub(in crate::application::file_dag) fn prepare(
        request: ViewRequest,
        rows: &Rows<'_>,
    ) -> Arc<DerivedContract> {
        Arc::new(
            super::super::prepare::view(
                (std::sync::Arc::new(
                    ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
                )
                    as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
                &request,
                &[InputShape {
                    kinds: kinds_of(rows),
                    tree: None,
                }],
            )
            .unwrap(),
        )
    }
    pub(in crate::application::file_dag) fn acquire(
        &self,
        input: Input,
        rows: &Rows<'_>,
        endpoints: [ColumnIndex; 2],
        keys: &[ColumnIndex],
    ) -> Result<Option<GraphHandle>, OperatingError> {
        let plan = Self::prepare(
            ViewRequest::Adjacency {
                endpoints,
                keys: keys.to_vec(),
            },
            rows,
        );
        self.acquire_prepared(&plan, input, rows)
    }
    pub(in crate::application::file_dag) fn acquire_prepared(
        &self,
        plan: &Arc<DerivedContract>,
        input: Input,
        rows: &Rows<'_>,
    ) -> Result<Option<GraphHandle>, OperatingError> {
        let workspace = self.workspace(input, rows);
        let Some(source) = workspace.graph_input(0, rows)? else {
            return Ok(None);
        };
        self.owner.derived.acquire_graph(
            plan,
            &source,
            self.owner.derived.backend.borrow_mut().as_mut(),
        )
    }
    pub(in crate::application::file_dag) fn domain(
        &self,
        graph: &GraphHandle,
        input: Input,
        rows: &Rows<'_>,
        column: ColumnIndex,
    ) -> Result<Option<DomainHandle>, OperatingError> {
        let plan = Self::prepare(ViewRequest::Membership { column }, rows);
        let workspace = self.workspace(input, rows);
        let Some(source) = workspace.graph_input(0, rows)? else {
            return Ok(None);
        };
        self.owner
            .derived
            .acquire_graph_domain(
                &plan,
                graph,
                &source,
                self.owner.derived.backend.borrow_mut().as_mut(),
            )
            .map(Some)
    }
    pub(in crate::application::file_dag) fn reach(
        &self,
        access: &GraphHandle,
        partition: &[Value],
        root: Value,
        walk: &mut GraphWalk,
    ) -> Result<Option<RootAnswer>, OperatingError> {
        DemandAnswers::new(
            &self.owner.derived,
            self.owner.derived.backend.borrow_mut().as_mut(),
        )
        .reach(access, partition, root, walk)
    }
    pub(in crate::application::file_dag) fn nearest(
        &self,
        access: &GraphHandle,
        domain: &DomainHandle,
        root: u32,
        orientation: Orientation,
        local: &mut Vec<u32>,
        walk: &mut GraphWalk,
    ) -> Result<Option<u32>, OperatingError> {
        DemandAnswers::new(
            &self.owner.derived,
            self.owner.derived.backend.borrow_mut().as_mut(),
        )
        .nearest(access, domain, root, orientation, local, walk)
    }
    pub(in crate::application::file_dag) fn index(
        &self,
        relation: Relation,
        rows: &Rows<'_>,
        columns: &[ColumnIndex],
    ) -> Result<super::super::view::AcquiredView, OperatingError> {
        let plan = Self::prepare(
            ViewRequest::Equality {
                columns: columns.to_vec(),
            },
            rows,
        );
        let proof = self
            .owner
            .admit_relation(relation, BorrowedRows::from(rows))?;
        let (population, _) = proof.identity(&self.owner)?;
        let Rows::Stored(source) = rows else {
            return Err(OperatingError::WrongOperation);
        };
        self.owner.derived.acquire_stored(
            &plan,
            &[population],
            ViewContext::default(),
            DerivedInputs::Rows(&[proof.rows()]),
            &mut DerivedScratch::default(),
            Some((relation, *source)),
        )
    }
    pub(in crate::application::file_dag) fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        self.owner.derived.set_optional_limit(limit).unwrap();
    }
    pub(in crate::application::file_dag) fn clear(&self) {
        self.owner.release_population();
        self.setup();
    }
    pub(in crate::application::file_dag) fn reclaim(&self, cost: usize) -> bool {
        let mut backend = self.owner.derived.backend.borrow_mut();
        self.owner
            .derived
            .prepare_graph_room(backend.as_mut().unwrap(), cost, &[])
            .unwrap()
    }
    pub(in crate::application::file_dag) fn graph_cost(&self, graph: &GraphHandle) -> usize {
        let backend = self.owner.derived.backend.borrow();
        let cache = backend.as_ref().unwrap();
        let (_, metadata) = slot(&cache.graphs).unwrap();
        // Measure a cold admission: this probe already owns optional leases,
        // so its ledger incremental cost would exclude the graph payload.
        graph
            .backings
            .iter()
            .map(|backing| backing.bytes)
            .sum::<usize>()
            + metadata
    }
    pub(in crate::application::file_dag) fn snapshot(&self) -> Snapshot {
        let backend = self.owner.derived.backend.borrow();
        let cache = backend.as_ref().unwrap();
        let c = &cache.graph_counts;
        let ledger = self.owner.derived.ledger.borrow();
        let s = ledger.snapshot();
        let state = self.owner.derived.state.borrow();
        let mut indexes: Vec<_> = state
            .entries
            .values()
            .filter(|entry| {
                entry.retained.is_some()
                    && matches!(entry._plan.request, ViewRequest::Equality { .. })
            })
            .collect();
        indexes.sort_by_key(|entry| entry.admission);
        Snapshot {
            counts: GraphCounts {
                reclaimed_indexes: c.reclaimed_indexes,
                reclaimed_selections: c.reclaimed_selections,
                reclaimed_bytes: c.reclaimed_bytes,
                pinned_refusals: c.pinned_refusals,
                peak_bytes: c.peak_bytes,
                builds: c.builds,
                reuses: c.reuses,
                refusals: c.refusals,
                roots: c.roots,
                traversals: c.traversals,
                memo_hits: c.memo_hits,
                expanded: c.expanded,
                examined: c.examined,
                early_stops: c.early_stops,
                forest_hits: c.forest_hits,
            },
            bytes: cache.bytes,
            limit: cache.limit,
            memo_bytes: cache.memo_bytes,
            graphs: cache.graphs.len(),
            domains: cache.graph_domains.len(),
            nearest: cache.nearest.len(),
            indexes: indexes.len(),
            owner_entries_capacity: state.entries.capacity(),
            index_payloads: indexes.iter().map(|entry| entry.bytes).collect(),
            index_columns: indexes
                .iter()
                .map(|entry| match &entry._plan.request {
                    ViewRequest::Equality { columns } => columns.clone(),
                    _ => unreachable!("filtered equality entry"),
                })
                .collect(),
            required_bytes: s.required_bytes,
            optional_bytes: s.view_optional_bytes,
            unique_bytes: s.unique_bytes,
            overlap_bytes: s.overlap_bytes,
            graph_backings: ledger.backing_count(9),
            retained_view_bytes: self.owner.derived.state.borrow().retained_bytes,
            selections: self.owner.derived.state.borrow().retained_predicates,
            retained_positions: s.optional_units,
        }
    }
}
