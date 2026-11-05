//! Pure derived-owner fixtures; no operator evaluator or legacy transport.
use super::*;
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
    pub builds: u64,
    pub reuses: u64,
    pub entries: usize,
    pub graph_builds: u64,
    pub graph_reuses: u64,
    pub graph_early_stops: u64,
}
/// Compile actual plans/declarations before installing the production file descriptors.
pub(crate) fn prepared_sources(plans: &[Plan]) -> super::super::prepared::PreparedDerived {
    use crate::domain::{
        operator::contract::types::RequestOperators,
        source_partitions::types::source_partitions::value::SourcePartitions,
    };
    use ryo_values::domain::{
        program::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
        rule_requests::types::{RuleParameters, RuleSpec},
        runs::types::{RuleApplicability, RuleId},
    };
    let rules = plans
        .iter()
        .enumerate()
        .map(|(i, _)| RuleSpec {
            id: RuleId(format!("derived-owner-{i}").into()),
            query: QueryBundle {
                root: QueryDocumentSource {
                    id: QuerySourceId(format!("derived-owner-{i}").into()),
                    text: "".into(),
                },
                modules: Default::default(),
            },
            applicability: RuleApplicability::Always,
            given: vec![],
            observations: vec![],
            spelled: RuleParameters("".into()),
            include: vec![],
            exclude: vec![],
        })
        .collect::<Vec<_>>();
    let bindings = vec![vec![]; plans.len()];
    let semantic = crate::application::file_dag::testing::program_derived_port();
    let request = RequestOperators::prepare(plans, &bindings, &[], semantic).unwrap();
    let borrowed = plans.iter().map(Ok).collect::<Vec<_>>();
    let partitions = SourcePartitions::new(
        &rules,
        &borrowed,
        &bindings,
        &[],
        &vec![vec![]; plans.len()],
    )
    .unwrap();
    let graph = crate::domain::file_program::service::compile_prepared(plans, &request);
    let program =
        crate::domain::executable_program::service::compile(plans, graph, &partitions, &request)
            .unwrap();
    super::super::prepared::PreparedDerived::compile(&program, &request).unwrap()
}
pub(crate) fn source_plan(relation: Relation) -> Plan {
    use ryo_values::domain::{
        plan_parts::types::{Input, StepId},
        plans::types::{Emit, Operation, PlanStep, StepScope},
    };
    Plan {
        parameters: vec![],
        observation_inputs: vec![],
        observations: vec![],
        frontier: vec![],
        comparison_origins: vec![],
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(relation),
                comparisons: vec![],
                kept: vec![],
            },
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
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
        let plans = wanted.iter().copied().map(source_plan).collect::<Vec<_>>();
        owner
            .derived
            .prepare_program_file(
                &prepared_sources(&plans),
                &vec![true; plans.len()],
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
    ) -> Result<super::super::view::AcquiredView, OperatingError> {
        let plan = super::graph_fixture::GraphFixture::prepare(
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
/// Observe backing lifetime without retaining a lease or exposing mutable storage.
pub(crate) fn weak_storage(
    view: &super::super::view::AcquiredView,
) -> Weak<super::super::storage::DerivedStorage> {
    Arc::downgrade(&view.storage)
}
