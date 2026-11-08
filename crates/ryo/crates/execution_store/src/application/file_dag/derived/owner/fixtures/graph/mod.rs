//! Test setup uses the production owner, original-row admission and physical dispatch.
use ryo_derived::domain::vertices::types::VertexId;
use ryo_values::domain::plan_parts::types::InputNumber;
use crate::application::file_dag::derived::owner::graphs::{
    service::slot,
    types::{
        handles::service::{DomainHandle, GraphHandle, RootAnswer},
        roots::value::DemandAnswers,
    },
};
use crate::application::file_dag::derived::owner::*;
use crate::application::file_dag::{
    relations::{RelationOwner, handle::service::PublishedInput},
    workspace::InvocationWorkspace,
};
use ryo_execution_domain::domain::{
    coverage::types::Partial,
    stored_graph::types::{GraphWalk, Orientation},
    table_model::types::Rows,
};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::{Relation, Value};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Direction, Input, StepId, StepScope},
    plans::types::{Emit, Operation, Plan, PlanStep},
};
use std::rc::Rc;

/// A derived owner over a fake port, with the graph acquisitions a proof drives.
pub struct GraphFixture {
    pub(crate) owner: Rc<RelationOwner>,
    wanted: Vec<Relation>,
    partials: Vec<Partial>,
    limit: usize,
}
impl GraphFixture {
    pub(crate) fn new(
        wanted: &[Relation],
        partials: &[Partial],
        limit: usize,
    ) -> Self {
        Self::with_identity(wanted, partials, limit, true, 1024)
    }
    pub(crate) fn with_identity(
        wanted: &[Relation],
        partials: &[Partial],
        limit: usize,
        stable: bool,
        max_views: usize,
    ) -> Self {
        let fixture = Self {
            owner: Rc::new(RelationOwner::for_request(
                if stable {
                    crate::application::file_dag::relations::types::ReuseEligibility::Eligible
                } else {
                    crate::application::file_dag::relations::types::ReuseEligibility::Ineligible
                },
                crate::application::file_dag::relations::types::PinnedViewCount(max_views),
                (std::sync::Arc::new(
                    crate::application::file_dag::fixtures::derived(),
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
                    child: ColumnIndex(0),
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
        let mut plans = self
            .wanted
            .iter()
            .copied()
            .map(crate::application::file_dag::derived::owner::fixtures::source_plan)
            .collect::<Vec<_>>();
        if !self.wanted.is_empty() {
            plans.push(self.request());
        }
        let prepared = crate::application::file_dag::derived::owner::fixtures::prepared_sources(
            &plans,
            self.owner.derived().semantic.clone(),
        );
        self.owner
            .derived()
            .prepare_program_file(
                &prepared,
                &(0..plans.len())
                    .map(ryo_execution_domain::domain::coverage::types::RuleIndex)
                    .collect::<Vec<_>>(),
                &self.partials,
            )
            .unwrap();
        self.owner
            .derived()
            .set_optional_limit(
                crate::application::file_dag::derived::owner::index::types::ByteLimit(self.limit),
            )
            .unwrap();
    }
    fn workspace(
        &self,
        input: Input,
        rows: &Rows<'_>,
    ) -> Result<(InvocationWorkspace, PublishedInput), OperatingError> {
        // A computed population is a real owned publication, never a stable
        // relation alias. Resident/selected rows retain their exact source witness.
        let publication = match rows {
            Rows::Computed(table) => self.owner.computed((*table).clone())?,
            Rows::Compact(table) => {
                let publication = self.owner.source_compact(Arc::new((*table).clone()))?;
                let supplied = self
                    .owner
                    .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
                self.owner.admit(
                    publication.as_ref(),
                    RepresentationInput::from(&supplied),
                )?;
                publication
            }
            _ => match input {
                Input::Relation(relation) => self.owner.source(relation, rows)?,
                Input::Step(_) | Input::BoundObservation(_) => return Err(OperatingError::NotRun),
            },
        };
        let mut workspace = InvocationWorkspace::new(Rc::clone(&self.owner));
        workspace.capture_admitted(vec![publication.clone()]);
        Ok((workspace, publication))
    }
    /// Acquires the graph of `rows` between `endpoints`, keyed by `keys`.
    pub fn acquire(
        &self,
        input: Input,
        rows: &Rows<'_>,
        endpoints: [ColumnIndex; 2],
        keys: &[ColumnIndex],
    ) -> Result<Option<GraphHandle>, OperatingError> {
        let plan = crate::application::file_dag::derived::owner::fixtures::prepare_view(
            self.owner.derived().semantic.clone(),
            ViewRequest::Adjacency {
                endpoints,
                keys: keys.to_vec(),
            },
            rows,
        );
        self.acquire_prepared(&plan, input, rows)
    }
    /// A demand whose plan belongs to another family: the production entry must refuse it.
    pub(crate) fn acquire_wrong(
        &self,
        input: Input,
        rows: &Rows<'_>,
        request: ViewRequest,
    ) -> Result<Option<GraphHandle>, OperatingError> {
        let plan = crate::application::file_dag::derived::owner::fixtures::prepare_view(
            self.owner.derived().semantic.clone(),
            request,
            rows,
        );
        self.acquire_prepared(&plan, input, rows)
    }
    pub(crate) fn acquire_prepared(
        &self,
        plan: &Arc<DerivedContract>,
        input: Input,
        rows: &Rows<'_>,
    ) -> Result<Option<GraphHandle>, OperatingError> {
        let (workspace, publication) = self.workspace(input, rows)?;
        let admitted_rows = self
            .owner
            .resolve(publication.as_ref(), &|relation| match input {
                Input::Relation(expected) if expected == relation => Ok(rows.clone()),
                _ => Err(OperatingError::NotRead),
            })?;
        let Some(source) = workspace.graph_input(InputNumber(0), &admitted_rows)? else {
            return Ok(None);
        };
        let mut cache = self.owner.derived().graph_cache()?;
        self.owner.derived().acquire_graph(plan, &source, &mut cache)
    }
    /// Required bytes the owner keeps for the admitted source of `rows` until the generation
    /// retires: what a demand's exit returns to, since the exit does not retire the source.
    pub(crate) fn source_floor(
        &self,
        input: Input,
        rows: &Rows<'_>,
    ) -> Result<usize, OperatingError> {
        let (workspace, publication) = self.workspace(input, rows)?;
        let admitted_rows = self
            .owner
            .resolve(publication.as_ref(), &|relation| match input {
                Input::Relation(expected) if expected == relation => Ok(rows.clone()),
                _ => Err(OperatingError::NotRead),
            })?;
        drop(workspace.graph_input(InputNumber(0), &admitted_rows)?);
        drop(admitted_rows);
        drop(workspace);
        drop(publication);
        Ok(self.owner.derived().publication_ownership_bytes().0)
    }
    pub(crate) fn domain(
        &self,
        graph: &GraphHandle,
        input: Input,
        rows: &Rows<'_>,
        column: ColumnIndex,
    ) -> Result<Option<DomainHandle>, OperatingError> {
        let plan = crate::application::file_dag::derived::owner::fixtures::prepare_view(
            self.owner.derived().semantic.clone(),
            ViewRequest::Membership { column },
            rows,
        );
        let (workspace, publication) = self.workspace(input, rows)?;
        let admitted_rows = self
            .owner
            .resolve(publication.as_ref(), &|relation| match input {
                Input::Relation(expected) if expected == relation => Ok(rows.clone()),
                _ => Err(OperatingError::NotRead),
            })?;
        let Some(source) = workspace.graph_input(InputNumber(0), &admitted_rows)? else {
            return Ok(None);
        };
        let mut cache = self.owner.derived().graph_cache()?;
        self.owner
            .derived()
            .acquire_graph_domain(&plan, graph, &source, &mut cache)
            .map(Some)
    }
    /// Reaches the fixture graph from its roots.
    pub fn reach(
        &self,
        access: &GraphHandle,
        partition: &[Value],
        root: Value,
        walk: &mut GraphWalk,
    ) -> Result<Option<RootAnswer>, OperatingError> {
        let mut cache = self.owner.derived().graph_cache()?;
        DemandAnswers::new(self.owner.derived(), &mut cache).reach(access, partition, root, walk)
    }
    /// The nearest vertex from `root` in `orientation` over an acquired graph.
    pub fn nearest(
        &self,
        access: &GraphHandle,
        domain: &DomainHandle,
        root: u32,
        orientation: Orientation,
        local: &mut Vec<VertexId>,
        walk: &mut GraphWalk,
    ) -> Result<Option<u32>, OperatingError> {
        let mut cache = self.owner.derived().graph_cache()?;
        let mut answers = std::mem::take(local);
        let found = DemandAnswers::new(self.owner.derived(), &mut cache)
            .nearest(access, domain, VertexId(root), orientation, &mut answers, walk);
        *local = answers;
        found.map(|found| found.map(|vertex| vertex.0))
    }
    /// Acquires the key index of `rows` over `columns`.
    pub fn index(
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
    /// Sets the owner's retained-byte limit.
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        self.owner
            .derived()
            .set_optional_limit(
                crate::application::file_dag::derived::owner::index::types::ByteLimit(limit),
            )
            .unwrap();
    }
    /// Whether a graph demand can reach the cache of the owner now.
    pub fn cache_ready(&self) -> Result<(), OperatingError> {
        self.owner.derived().graph_cache().map(|_| ())
    }
    /// Retire the generation: the owner drops its cache until the file is prepared again.
    pub fn retire(&self) {
        self.owner.derived().clear();
    }
    pub(crate) fn clear(&self) {
        self.owner.release_population();
        self.setup();
    }
    /// Whether reclaiming `cost` bytes succeeds.
    pub fn reclaim(&self, cost: usize) -> bool {
        let mut backend = self.owner.derived().backend.borrow_mut();
        self.owner
            .derived()
            .prepare_graph_room(
                backend.as_mut().unwrap(),
                crate::application::file_dag::derived::ledger::types::ByteCount(cost),
                &[],
            )
            .unwrap()
            == Room::Fits
    }
    /// The bytes the graph retains.
    pub fn graph_cost(&self, graph: &GraphHandle) -> usize {
        let backend = self.owner.derived().backend.borrow();
        let cache = backend.as_ref().unwrap();
        let metadata = slot(&cache.graphs).unwrap().metadata.0;
        // Measure a cold admission: this probe already owns optional leases,
        // so its ledger incremental cost would exclude the graph payload.
        graph
            .backings()
            .iter()
            .map(|backing| backing.bytes.0)
            .sum::<usize>()
            + metadata
    }
}
