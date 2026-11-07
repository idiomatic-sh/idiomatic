//! The plans, rules and request graph the `file_program` tests share.
use super::service::compile_prepared;
use super::types::RequestGraph;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
    ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
};
use ryo_values::domain::plan_parts::types::Input as InputSource;
use ryo_values::domain::plan_parts::types::*;
use ryo_values::domain::plans::types::*;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId};
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use std::sync::Arc;

/// A rule's empty query text and parameter spelling.
const NO_TEXT: &str = "";

/// The name every producer exports and every consumer binds.
const OUTPUT: &str = "out";

/// A function selection followed by a projection of it.
pub(super) fn plan() -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        steps: vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: InputSource::Relation(Relation::Function),
                    kept: vec![ColumnIndex(0), ColumnIndex(2)],
                    comparisons: vec![PlanComparison {
                        test: Test::Equal,
                        left: Operand::Column(ColumnIndex(1)),
                        right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
                        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                    }],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: InputSource::Step(StepId(0)),
                    columns: vec![ColumnIndex(1)],
                    distinct: RepeatedRows::Kept,
                },
            },
        ],
        emit: Emit {
            step: StepId(1),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
}
/// The request graph of `plans` over the operators the application prepares for them.
pub(super) fn graph(
    plans: &[Plan],
    bindings: &[Vec<ParameterBinding>],
) -> RequestGraph {
    let operators = crate::application::file_dag::request::prepare_with_schemas(
        plans,
        bindings,
        &[],
        &ryo_relations::domain::relations::service::kinds,
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    )
    .expect("the application prepares operators for the fixture plans");
    compile_prepared(plans, &operators)
}
/// A rule that reads no named root.
pub(super) fn rule(id: &str) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(id.into()),
                text: NO_TEXT.into(),
            },
            modules: Default::default(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters(NO_TEXT.into()),
        include: vec![],
        exclude: vec![],
    }
}

/// The one column a named root of these fixtures holds.
pub(super) fn column() -> ObservationColumn {
    ObservationColumn {
        name: ObservationColumnName("c".into()),
        kind: CellKind::Number,
    }
}

/// The binding of `name` to the given producers, over the one column.
pub(super) fn binding(name: &str, producers: Vec<ObservationProducer>) -> ObservationBinding {
    ObservationBinding {
        name: ObservationBindingName(name.into()),
        columns: vec![column()],
        producers,
    }
}

/// The export of `rule` that every consumer of these fixtures names.
pub(super) fn export(rule: &str) -> ObservationProducer {
    ObservationProducer::Output {
        rule: RuleId(rule.into()),
        output: ObservationName(OUTPUT.into()),
    }
}

/// The rule `id` reading the given bindings.
pub(super) fn observing(id: &str, observations: Vec<ObservationBinding>) -> RuleSpec {
    RuleSpec {
        observations,
        ..rule(id)
    }
}

/// The plan of a producer that exports `OUTPUT` over the given columns.
pub(super) fn exporting(columns: Vec<ObservationColumn>) -> Plan {
    let mut exporting = plan();
    exporting.observations.push(PlanObservation {
        name: ObservationName(OUTPUT.into()),
        source: InputSource::Step(StepId(0)),
        columns,
        scope: StepScope::PerFile,
    });
    exporting
}

/// The input a plan declares for a binding over the one column.
pub(super) fn bound_input(name: &str) -> BoundObservationInput {
    BoundObservationInput {
        name: ObservationBindingName(name.into()),
        columns: vec![column()],
        scope: StepScope::PerFile,
    }
}
