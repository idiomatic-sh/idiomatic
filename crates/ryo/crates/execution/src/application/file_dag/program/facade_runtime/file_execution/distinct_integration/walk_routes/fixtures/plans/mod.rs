//! Authored plans declare expected barriers without consulting the prepared walk.
use super::super::super::*;
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBinding, ObservationBindingId, ObservationBindingName,
    ObservationColumn, ObservationColumnName, ObservationName, ObservationProducer,
    ObservationReadinessLayout, PlanObservation,
};
use ryo_values::domain::plan_parts::types::{Comparison, Operand, QueryLiteral, Test};
use ryo_values::domain::rule_requests::types::RuleSpec;

pub(in super::super) struct Authored {
    pub(in super::super) plans: Vec<Plan>,
    pub(in super::super) rules: Vec<RuleSpec>,
    pub(in super::super) intents: Vec<FileRuleIntent>,
    pub(in super::super) observations: ObservationIdentity,
}
fn rules(plans: &[Plan]) -> Vec<RuleSpec> {
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
fn plan(steps: Vec<PlanStep>) -> Plan {
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
fn comparison(test: Test, number: Integer) -> Comparison {
    Comparison {
        test,
        left: Operand::Column(ColumnIndex(1)),
        right: Operand::Literal(QueryLiteral::Number(number)),
        origin: None,
    }
}
fn head(kept: Vec<ColumnIndex>, comparisons: Vec<Comparison>) -> Plan {
    plan(vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: PlanInput::Relation(Relation::LanguageTarget),
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
fn collected() -> FileRuleIntent {
    FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }
}
impl Authored {
    pub(in super::super) fn continuations() -> Self {
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
            observations: ObservationIdentity::Independent,
        }
    }
    pub(in super::super) fn named() -> Self {
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
                comparisons: vec![Comparison {
                    test: Test::GreaterThan,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Literal(QueryLiteral::Number(Integer(0))),
                    origin: None,
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
            observations: ObservationIdentity::Named(ObservationReadinessLayout(Digest([93; 16]))),
        }
    }
    pub(in super::super) fn distinct() -> Self {
        let plans = vec![plan(vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: PlanInput::Relation(Relation::LanguageTarget),
                columns: vec![ColumnIndex(1)],
                distinct: RepeatedRows::Dropped,
            },
        }])];
        Self {
            rules: rules(&plans),
            plans,
            intents: vec![collected()],
            observations: ObservationIdentity::Independent,
        }
    }
}
