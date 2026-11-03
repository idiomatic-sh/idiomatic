//! Actual request topology, file activation differences, and independent consumer lifetimes.
use super::types::ActivationCache;
use crate::application::file_dag::{request::compile, types::FileDagExecution};
use crate::domain::file_program::types::RequestGraph;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, RepeatedRows, StepId};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};
use std::{rc::Rc, sync::Arc};

fn request_graph() -> Arc<RequestGraph> {
    let plan = Plan {
        parameters: vec![],
        steps: (0..3)
            .map(|step| PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: if step == 0 {
                        Input::Relation(Relation::LanguageTarget)
                    } else {
                        Input::Step(StepId(step - 1))
                    },
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                },
            })
            .collect(),
        emit: Emit {
            step: StepId(2),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    };
    Arc::new(compile(&[plan.clone(), plan]))
}

#[test]
fn same_request_and_complete_set_share_one_immutable_activation() {
    let graph = request_graph();
    let mut cache = ActivationCache::default();
    let first = cache.activate(
        &graph,
        |_| true,
        &[Relation::LanguageTarget, Relation::Function],
    );
    let again = cache.activate(
        &graph,
        |_| true,
        &[
            Relation::Function,
            Relation::LanguageTarget,
            Relation::Function,
        ],
    );
    assert!(Arc::ptr_eq(&first, &again));
    assert_eq!(first.active, [true, true]);
    let replacement = request_graph();
    let changed_request = cache.activate(
        &replacement,
        |_| true,
        &[Relation::LanguageTarget, Relation::Function],
    );
    assert!(!Arc::ptr_eq(&first, &changed_request));
    assert_eq!(first.remaining, changed_request.remaining);
}

#[test]
fn changed_governing_or_partial_population_rebuilds_exact_activation() {
    let graph = request_graph();
    let root = graph.boundaries[0][0].1;
    let mut cache = ActivationCache::default();
    let both = cache.activate(&graph, |_| true, &[Relation::LanguageTarget]);
    let left = cache.activate(&graph, |rule| rule == 0, &[Relation::LanguageTarget]);
    assert!(!Arc::ptr_eq(&both, &left));
    assert_eq!(left.active, [true, false]);
    assert_eq!(both.remaining[root.0], 2);
    assert_eq!(left.remaining[root.0], 1);
    let partial = cache.activate(&graph, |_| true, &[]);
    assert_eq!(partial.active, [false, false]);
    assert!(partial.remaining.iter().all(|count| *count == 0));
    let restored = cache.activate(&graph, |_| true, &[Relation::LanguageTarget]);
    assert!(
        !Arc::ptr_eq(&both, &restored),
        "single-entry replacement does not retain old variants"
    );
    assert_eq!(restored.remaining, both.remaining);
}

#[test]
fn schema_ineligible_request_keeps_residual_error_gate() {
    let mut graph = request_graph();
    let root = graph.cuts[0].iter().flatten().next().copied().unwrap();
    Arc::get_mut(&mut graph).unwrap().nodes[root.0].schema_valid = false;
    let schedule =
        ActivationCache::default().activate(&graph, |_| true, &[Relation::LanguageTarget]);
    assert_eq!(schedule.active, [false, false]);
    assert!(schedule.remaining.iter().all(|count| *count == 0));
}

#[test]
fn shared_template_keeps_generation_owners_and_lazy_consumer_release_independent() {
    let graph = request_graph();
    let mut cache = ActivationCache::default();
    let schedule = cache.activate(&graph, |_| true, &[Relation::LanguageTarget]);
    let port = crate::outbound::derived::factory::test_port();
    let owner = || {
        Rc::new(
            crate::application::file_dag::relations::RelationOwner::for_request(
                true,
                graph.nodes.len() + 2,
                Arc::clone(&port),
            ),
        )
    };
    let mut first = FileDagExecution::new_for_schedule(&schedule, owner());
    let second = FileDagExecution::new_for_schedule(&schedule, owner());
    assert!(!Rc::ptr_eq(&first.relations, &second.relations));
    let before = second.remaining.clone();
    first.finish_rule(0);
    assert_eq!(second.remaining, before);
    first.finish_rule(1);
    assert!(
        first.remaining.iter().all(|count| *count == 0),
        "unrequested ancestors cancel through the actual consumer path"
    );
    assert_eq!(second.remaining, before);
    assert_eq!(schedule.remaining, before);
    let next = cache.activate(&graph, |_| true, &[Relation::LanguageTarget]);
    assert!(Arc::ptr_eq(&schedule, &next));
}
