//! Shared real request and semantic-descriptor preparation data for owner controls.
use ryo_execution_domain::domain::{
    derived_views::types::{contract::value::DerivedContract, ViewRequest},
    operation_schema::types::InputShape,
    table_model::{service::kinds_of, types::Rows},
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::{plan_parts::types::ColumnIndex, plans::types::Plan};
use std::sync::Arc;
/// Compile actual plans/declarations before installing the production file descriptors.
pub(crate) fn prepared_sources(
    plans: &[Plan],
    semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
) -> super::super::prepared::RequestDerived {
    use ryo_execution_domain::domain::{
        source_partitions::types::source_partitions::value::SourcePartitions,
    };
    use ryo_values::domain::{
        query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
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
    let request = crate::application::file_dag::service::prepare(plans, &bindings, &[], semantic).unwrap();
    let borrowed = plans.iter().map(Ok).collect::<Vec<_>>();
    let partitions = SourcePartitions::new(
        &rules,
        &borrowed,
        &bindings,
        &[],
        &vec![vec![]; plans.len()],
    )
    .unwrap();
    let graph = ryo_execution_domain::domain::file_program::service::compile_prepared_observations(
        plans,
        &request,
        ryo_execution_domain::domain::file_program::types::ObservationRequest::Independent,
    );
    let program =
        ryo_execution_domain::domain::executable_program::service::compile(plans, graph, &partitions, &request)
            .unwrap();
    super::super::prepared::RequestDerived::compile(&program).unwrap()
}
pub(crate) fn source_plan(relation: Relation) -> Plan {
    use ryo_values::domain::{
        plan_parts::types::{Input, StepId, StepScope},
        plans::types::{Emit, Operation, PlanStep},
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

pub(in crate::application::file_dag) fn prepare_view(
    semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
    request: ViewRequest,
    rows: &Rows<'_>,
) -> Arc<DerivedContract> {
    Arc::new(
        super::super::prepare::view(
            semantic,
            &request,
            &[InputShape {
                kinds: kinds_of(rows),
                tree: None,
            }],
        )
        .unwrap(),
    )
}

pub(in crate::application::file_dag) mod graph;
mod graph_snapshot;
mod observations;
pub(in crate::application::file_dag) mod resources;
