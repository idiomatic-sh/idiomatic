//! Authored plans declare expected barriers without consulting the prepared walk.
use super::super::super::fixtures::*;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use ryo_relations::domain::relations::types::{CellKind, Integer, Relation};
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
    ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
};
use ryo_values::domain::plan_parts::types::ObservationBindingId;
use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, QueryLiteral, Test};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::records::types::ObservationReadinessLayout;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, StepId, StepScope},
    plans::types::{Emit, Operation, Plan, PlanStep},
    runs::types::RuleId,
    stored_facts::types::Digest,
};

pub(crate) struct Authored {
    pub(crate) plans: Vec<Plan>,
    pub(crate) rules: Vec<RuleSpec>,
    pub(crate) intents: Vec<FileRuleIntent>,
    pub(crate) observations: ObservationIdentity,
    /// Parameter bindings of each rule; empty when no rule binds a parameter.
    pub(crate) bindings: Vec<Vec<ParameterBinding>>,
    /// Host relations the request declares.
    pub(crate) hosts: Vec<HostRelation>,
}
pub(super) fn rules(plans: &[Plan]) -> Vec<RuleSpec> {
    plans
        .iter()
        .enumerate()
        .map(|(at, _)| {
            let mut rule = rule();
            rule.id = RuleId(format!("walk-route-{at}").into());
            rule
        })
        .collect()
}
pub(super) fn plan(steps: Vec<PlanStep>) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        emit: Emit {
            step: StepId((steps.len() - 1) as u32),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps,
        frontier: vec![],
        comparison_origins: vec![],
    }
}
pub(super) fn comparison(test: Test, number: Integer) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(1)),
        right: Operand::Literal(QueryLiteral::Number(number)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}
fn head(kept: Vec<ColumnIndex>, comparisons: Vec<PlanComparison>) -> Plan {
    head_of(Relation::LanguageTarget, kept, comparisons)
}
fn head_of(relation: Relation, kept: Vec<ColumnIndex>, comparisons: Vec<PlanComparison>) -> Plan {
    plan(vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: PlanInput::Relation(relation),
            comparisons,
            kept,
        },
    }])
}
fn extended(kept: ColumnIndex, suffix: Integer) -> Plan {
    let mut plan = head(vec![kept], vec![comparison(Test::GreaterThan, Integer(1))]);
    plan.steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Extend {
            from: PlanInput::Step(StepId(0)),
            values: vec![Operand::Literal(QueryLiteral::Number(suffix))],
        },
    });
    plan.emit.step = StepId(1);
    plan
}
pub(super) fn collected() -> FileRuleIntent {
    FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }
}
/// The production intent shape of the one core caller: collected rows, required
/// observations and frontier capture, so a rule owes two entry-point deliveries.
fn captured() -> FileRuleIntent {
    FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::Required,
        capture: CaptureIntent::Observations,
    }
}
impl Authored {
    /// Two rules sharing one class under the production intent shape.
    pub(crate) fn captured() -> Self {
        let plans = vec![
            extended(ColumnIndex(2), Integer(7)),
            extended(ColumnIndex(2), Integer(7)),
        ];
        Self {
            rules: rules(&plans),
            intents: vec![captured(); plans.len()],
            plans,
            bindings: vec![],
            hosts: vec![],
            observations: ObservationIdentity::Independent,
        }
    }
    /// Two relations, two classes on the first (`> 1` and `> 2`) and one class on the
    /// second (`> 1`) consumed by two rules: rules 0 and 1 read `LanguageTarget`,
    /// rules 2 and 3 read `FilesystemLineCount`.
    pub(crate) fn relations() -> Self {
        let on = |relation, number| {
            head_of(
                relation,
                vec![ColumnIndex(1)],
                vec![comparison(Test::GreaterThan, Integer(number))],
            )
        };
        let plans = vec![
            on(Relation::LanguageTarget, 1),
            on(Relation::LanguageTarget, 2),
            on(Relation::FilesystemLineCount, 1),
            on(Relation::FilesystemLineCount, 1),
        ];
        Self {
            rules: rules(&plans),
            intents: vec![collected(); plans.len()],
            plans,
            bindings: vec![],
            hosts: vec![],
            observations: ObservationIdentity::Independent,
        }
    }
    /// One admitted sharing class in `scope`: rules 0 and 1 author the same `head` over
    /// `LanguageTarget`, and rule 2 a selection of the same relation with another predicate
    /// (`> 1`), so the request holds two distinct classes over one source.
    pub(crate) fn sharing(scope: StepScope, head: Operation) -> Self {
        let mut plans = vec![
            plan(vec![PlanStep {
                scope,
                operation: head.clone(),
            }]),
            plan(vec![PlanStep {
                scope,
                operation: head,
            }]),
            plan(vec![PlanStep {
                scope,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::LanguageTarget),
                    comparisons: vec![comparison(Test::GreaterThan, Integer(1))],
                    kept: vec![ColumnIndex(1)],
                },
            }]),
        ];
        if scope == StepScope::CrossFile {
            for plan in &mut plans {
                plan.frontier = vec![PlanInput::Relation(Relation::LanguageTarget)];
            }
        }
        Self {
            rules: rules(&plans),
            intents: vec![collected(); plans.len()],
            plans,
            bindings: vec![],
            hosts: vec![],
            observations: ObservationIdentity::Independent,
        }
    }
    /// A shared head beside a projection that drops repeated rows: the walk declines that
    /// representation.
    pub(crate) fn declined() -> Self {
        let plans = vec![
            head(
                vec![ColumnIndex(1)],
                vec![comparison(Test::GreaterThan, Integer(1))],
            ),
            plan(vec![PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: PlanInput::Relation(Relation::LanguageTarget),
                    columns: vec![ColumnIndex(1)],
                    distinct: RepeatedRows::Dropped,
                },
            }]),
        ];
        Self {
            rules: rules(&plans),
            intents: vec![collected(); plans.len()],
            plans,
            bindings: vec![],
            hosts: vec![],
            observations: ObservationIdentity::Independent,
        }
    }
    pub(crate) fn continuations() -> Self {
        let good = extended(ColumnIndex(2), Integer(7));
        let plans = vec![
            extended(ColumnIndex(1), Integer(99)),
            good.clone(),
            good,
            head(
                vec![ColumnIndex(1), ColumnIndex(1)],
                vec![comparison(Test::GreaterThan, Integer(1))],
            ),
        ];
        let mut intents = vec![collected(); plans.len()];
        intents[2] = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Forwarded,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        };
        Self {
            rules: rules(&plans),
            plans,
            intents,
            bindings: vec![],
            hosts: vec![],
            observations: ObservationIdentity::Independent,
        }
    }
    /// A derived operation after a shared head, beside a plain head of the same class.
    pub(crate) fn grouped() -> Self {
        let mut grouped = head(
            vec![ColumnIndex(1)],
            vec![comparison(Test::GreaterThan, Integer(1))],
        );
        grouped.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::GroupCount {
                from: PlanInput::Step(StepId(0)),
                keys: vec![ColumnIndex(0)],
                distinct: None,
            },
        });
        grouped.emit.step = StepId(1);
        let plans = vec![
            grouped,
            head(
                vec![ColumnIndex(1)],
                vec![comparison(Test::GreaterThan, Integer(1))],
            ),
        ];
        Self {
            rules: rules(&plans),
            intents: vec![collected(); plans.len()],
            plans,
            bindings: vec![],
            hosts: vec![],
            observations: ObservationIdentity::Independent,
        }
    }
    pub(crate) fn named() -> Self {
        let columns = vec![ObservationColumn {
            name: ObservationColumnName("number".into()),
            kind: CellKind::Number,
        }];
        let mut plans = [Integer(1), Integer(3)]
            .into_iter()
            .map(|number| {
                let mut plan = head(vec![ColumnIndex(1)], vec![comparison(Test::Equal, number)]);
                plan.observations.push(PlanObservation {
                    name: ObservationName("picked".into()),
                    source: PlanInput::Step(StepId(0)),
                    columns: columns.clone(),
                    scope: StepScope::PerFile,
                });
                plan
            })
            .collect::<Vec<_>>();
        let mut observed = plan(vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::BoundObservation(ObservationBindingId(0)),
                comparisons: vec![PlanComparison {
                    test: Test::GreaterThan,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Literal(QueryLiteral::Number(Integer(0))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: vec![],
            },
        }]);
        observed.observation_inputs.push(BoundObservationInput {
            name: ObservationBindingName("picked".into()),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        });
        plans.push(observed);
        plans.push(head(
            vec![ColumnIndex(1)],
            vec![comparison(Test::GreaterThan, Integer(0))],
        ));
        let mut rules = rules(&plans);
        rules[2].observations = vec![ObservationBinding {
            name: ObservationBindingName("picked".into()),
            columns,
            producers: rules[..2]
                .iter()
                .map(|rule| ObservationProducer::Output {
                    rule: rule.id.clone(),
                    output: ObservationName("picked".into()),
                })
                .collect(),
        }];
        let mut intents = vec![collected(); plans.len()];
        for intent in &mut intents[..2] {
            *intent = FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Disabled,
                observations: ObservationDemand::Required,
                capture: CaptureIntent::Disabled,
            };
        }
        intents[3] = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Forwarded,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        };
        Self {
            plans,
            rules,
            intents,
            bindings: vec![],
            hosts: vec![],
            observations: ObservationIdentity::Named(ObservationReadinessLayout(Digest([93; 16]))),
        }
    }
}
