//! Actual compiled request and services for the external lifecycle proof.
use super::super::super::*;
use super::super::fixtures::{FinalConsumer, ProjectionOrder, shared_plan};
use crate::application::file_dag::program::preparation::types::PreparedProgram;

#[derive(Clone, Copy, Debug)]
pub(super) enum ExternalRootExit {
    Finish,
    RejectedDelivery,
    SemanticFailure,
    ProviderRefusal,
    SkippedAlias,
    UndemandedAlias,
    CaptureCallbackRefusal,
    CaptureCollectedFailure,
    CaptureCancelledAlias,
}

pub(super) struct PreparedCase {
    pub(super) prepared: Arc<PreparedProgram>,
    pub(super) services: Arc<ProgramServices>,
    pub(super) intents: [FileRuleIntent; 4],
    pub(super) capture_rule: Option<RuleIndex>,
}

pub(super) fn prepare_case(exit: ExternalRootExit) -> Result<PreparedCase, OperatingError> {
    let capture_rule = match exit {
        ExternalRootExit::CaptureCallbackRefusal | ExternalRootExit::CaptureCancelledAlias => {
            Some(RuleIndex(0))
        }
        ExternalRootExit::CaptureCollectedFailure => Some(RuleIndex(3)),
        _ => None,
    };
    let derived = crate::application::file_dag::testing::program_derived_port();
    let mut external = shared_plan(ProjectionOrder::Forward, FinalConsumer::Completes);
    external.steps.truncate(2);
    external.emit.step = StepId(1);
    let mut rules = vec![rule(); 4];
    for (at, rule) in rules.iter_mut().enumerate() {
        rule.id = RuleId(format!("external-root-{at}").into());
    }
    let mut plans = vec![
        external.clone(),
        external,
        shared_plan(ProjectionOrder::Forward, FinalConsumer::Completes),
        shared_plan(
            ProjectionOrder::Reversed,
            match exit {
                ExternalRootExit::SemanticFailure | ExternalRootExit::CaptureCollectedFailure => {
                    FinalConsumer::InvalidTerminal
                }
                ExternalRootExit::ProviderRefusal => FinalConsumer::RefusingProvider,
                _ => FinalConsumer::Completes,
            },
        ),
    ];
    if let Some(rule) = capture_rule {
        plans[rule.0].frontier = vec![PlanInput::Step(StepId(1))];
    }
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]; 4],
            plans,
            bindings: vec![vec![]; 4],
        },
        &rules,
        &vec![Digest([0; 16]); 4],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent, crate::domain::executable_program::types::RoutePolicy::default(),
    )?;
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::testing::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("external-roots.input".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let intent = FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    };
    let mut intents = [intent; 4];
    intents[0] = FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Forwarded,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    };
    match exit {
        ExternalRootExit::SkippedAlias | ExternalRootExit::CaptureCancelledAlias => {
            intents[1] = FileRuleIntent::Skipped(
                crate::domain::rule_intent::types::ApplicabilityReason::DiagnosticNotApplicable,
            );
        }
        ExternalRootExit::UndemandedAlias => intents[1] = FileRuleIntent::Undemanded,
        _ => {}
    }
    if let Some(rule) = capture_rule {
        let FileRuleIntent::Run { capture, .. } = &mut intents[rule.0] else {
            panic!("capture belongs to a declared live consumer")
        };
        *capture = CaptureIntent::DiagnosticAndObservations;
    }
    Ok(PreparedCase {
        prepared,
        services,
        intents,
        capture_rule,
    })
}
