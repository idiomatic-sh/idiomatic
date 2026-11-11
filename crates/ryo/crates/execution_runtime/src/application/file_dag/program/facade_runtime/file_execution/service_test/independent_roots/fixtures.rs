//! The support of the `independent_roots` cases.
use super::super::super::service::*;
use super::super::fixtures::{Root, Scenario, function_rows, plan, rule};
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationName,
    ObservationProducer, PlanObservation,
};
use ryo_values::domain::observations::types::{ObservationColumn, ObservationColumnName};
use ryo_values::domain::plan_parts::types::ObservationBindingId;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::records::types::ObservationReadinessLayout;
use ryo_values::domain::records::types::ProducerReadiness;
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, StepId, StepScope},
    plans::types::PlanStep,
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct RefuseCount;

impl crate::outbound::operators::interface::OperatorsPort for RefuseCount {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if matches!(
            &call,
            OperatorCall::ReduceCount { .. }
                | OperatorCall::Evaluate {
                    operation: Operation::GroupCount { .. },
                    ..
                }
        ) {
            return Err(PortableOperatorFault::WrongOperation);
        }
        crate::application::file_dag::fixtures::operators().operators(call)
    }
}

pub(super) fn columns(failed_emit: bool) -> Vec<ObservationColumn> {
    let mut columns = vec![ObservationColumn {
        name: ObservationColumnName("subject".into()),
        kind: CellKind::Node,
    }];
    if !failed_emit {
        columns.push(ObservationColumn {
            name: ObservationColumnName("count".into()),
            kind: CellKind::Number,
        });
    }
    columns
}

/// Which of the producer's two roots the engine refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Failing {
    /// The diagnostic root counts, which the engine refuses.
    Emit,
    /// The named observation root counts, which the engine refuses.
    Observation,
}

/// A producer's readiness, with its reason as text.
#[derive(Debug, PartialEq)]
pub(super) enum Readiness {
    Complete,
    Unknown(std::borrow::Cow<'static, str>),
    Skipped,
    NoQuery,
}

impl From<ProducerReadiness> for Readiness {
    fn from(readiness: ProducerReadiness) -> Self {
        match readiness {
            ProducerReadiness::Complete => Self::Complete,
            ProducerReadiness::Unknown(reason) => {
                Self::Unknown(std::borrow::Cow::Owned(reason.0.into()))
            }
            ProducerReadiness::Skipped => Self::Skipped,
            ProducerReadiness::NoQuery => Self::NoQuery,
        }
    }
}

/// What the producer rule answered on the file.
#[derive(Debug, PartialEq)]
pub(super) struct Observed {
    /// The diagnostic root.
    pub(super) emit: Root,
    /// The one named root's output index and outcome.
    pub(super) observation: [(ObservationIndex, Root); 1],
    /// The readiness the producer's first root recorded.
    pub(super) readiness: Option<Readiness>,
    /// Whether the producer's first root holds a publication.
    pub(super) published: bool,
}

/// Runs both rules on one file with the refused root `failing`; answers what the producer published.
pub(super) fn exercise(failing: Failing) -> Observed {
    {
        let failed_emit = failing == Failing::Emit;
        let output_columns = columns(failed_emit);
        let mut producer = plan(Scenario::ProjectRetry);
        producer.steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::Function),
                    comparisons: vec![],
                    kept: vec![ColumnIndex(0)],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::GroupCount {
                    from: PlanInput::Step(StepId(0)),
                    keys: vec![ColumnIndex(0)],
                    distinct: None,
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: PlanInput::Step(StepId(0)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Dropped,
                },
            },
        ];
        producer.emit.step = StepId(if failed_emit { 1 } else { 2 });
        producer.observations.push(PlanObservation {
            name: ObservationName("independent".into()),
            source: PlanInput::Step(StepId(if failed_emit { 2 } else { 1 })),
            columns: output_columns.clone(),
            scope: StepScope::PerFile,
        });
        let mut consumer = plan(Scenario::ProjectRetry);
        consumer.steps.truncate(1);
        consumer.emit.step = StepId(0);
        consumer.steps[0].operation = Operation::Select {
            from: PlanInput::BoundObservation(ObservationBindingId(0)),
            comparisons: vec![],
            kept: vec![],
        };
        consumer.observation_inputs.push(BoundObservationInput {
            name: ObservationBindingName("input".into()),
            columns: output_columns.clone(),
            scope: StepScope::PerFile,
        });
        let producer_rule = rule();
        let mut consumer_rule = rule();
        consumer_rule.id = RuleId("independent-consumer".into());
        consumer_rule.observations.push(ObservationBinding {
            name: ObservationBindingName("input".into()),
            columns: output_columns,
            producers: vec![ObservationProducer::Output {
                rule: producer_rule.id.clone(),
                output: ObservationName("independent".into()),
            }],
        });
        let rules = [producer_rule, consumer_rule];
        let compiled = ryo_execution_domain::domain::prepared_request::service::assemble(
            vec![producer, consumer],
            vec![vec![], vec![]],
            &rules,
        )
        .unwrap();
        let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
        let prepared = crate::application::file_dag::program::preparation::service::prepare(
            compiled,
            &rules,
            &[Digest([0; 16]); 2],
            &[],
            Arc::clone(&derived),
            ObservationIdentity::Named(ObservationReadinessLayout(Digest([6; 16]))),
        )
        .unwrap();
        let services = Arc::new(ProgramServices::new(
            Arc::new(CommonOperatorEngine::new(Arc::new(RefuseCount))),
            derived,
            &[CorpusInput::Available(CorpusEntry {
                executable: ExecutablePermission::Unknown,
                path: FilePath("independent-roots.py".into()),
                size: FileSize(0),
                modified: Nanoseconds(0),
            })],
        ));
        let mut symbols = Symbols::default();
        let mut file = open_file(
            &prepared,
            FileId(0),
            ScopePartials::File {
                evaluation: &[],
                publication: &[],
            },
            &[
                FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::Required,
                    capture: CaptureIntent::Disabled,
                },
                FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::Required,
                    capture: CaptureIntent::Disabled,
                },
            ],
            services,
            ryo_values::domain::files::types::SourceView(b""),
            ReuseEligibility::Eligible,
            &mut symbols,
            None,
            None,
        )
        .unwrap();
        let table = function_rows(&[0]);
        let read = |relation| {
            if relation == Relation::Function {
                Ok(Rows::Computed(&table))
            } else {
                Err(OperatingError::NotRead)
            }
        };
        let outcome = file
            .rule(
                RuleIndex(0),
                RuleDemand {
                    emit: EmitDemand::Enabled,
                    observations: ObservationDemand::Required,
                },
                &read,
                &mut symbols,
            )
            .unwrap();
        let observed = Observed {
            emit: Root::from(outcome.emit),
            observation: outcome
                .observations
                .into_iter()
                .map(|observation| (observation.output, Root::from(observation.outcome)))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            readiness: file.roots[&RuleIndex(0)][0]
                .readiness
                .clone()
                .map(Readiness::from),
            published: file.roots[&RuleIndex(0)][0].publication.is_some(),
        };
        file.finish().unwrap();
        observed
    }
}
