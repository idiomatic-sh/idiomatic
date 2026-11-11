//! Actual compiled request and services for the external lifecycle proof.
use super::super::super::super::*;
use super::super::super::fixtures::*;
use super::super::fixtures::{FinalConsumer, ProjectionOrder, shared_plan};
use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::runtime::types::AttemptCheckpoint;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::derived::owner::witness::service::BackingLiveness;
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{FileId, Integer, Relation, Value as Cell},
};
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::types::{RowCount, Table};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{Input as PlanInput, StepId},
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use std::rc::Rc;
use std::sync::Arc;

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
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
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
        ObservationIdentity::Independent,
    )?;
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
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
                ryo_execution_domain::domain::rule_intent::types::ApplicabilityReason::DiagnosticNotApplicable,
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

/// Two diagnostic obligations share an external Union root. Distinct Join
/// consumers acquire the real producer's equality lease during the same batch.
/// Unlike an internal-only producer, the externally held Union must stay live
/// after both Join readers release and until the file ends.
pub(super) fn external_root_exit(exit: ExternalRootExit) -> Result<(), OperatingError> {
    let PreparedCase {
        prepared,
        services,
        intents,
        capture_rule,
    } = prepare_case(exit)?;
    let root = |rule: RuleIndex| *prepared.program().rules()[rule.0].emit.as_ref().unwrap();
    let producer = root(RuleIndex(0));
    assert_eq!(
        producer,
        root(RuleIndex(1)),
        "actual compiled external root is shared"
    );
    let external_readers = if matches!(
        exit,
        ExternalRootExit::SkippedAlias
            | ExternalRootExit::UndemandedAlias
            | ExternalRootExit::CaptureCancelledAlias
    ) {
        1
    } else {
        2
    };
    let retained_readers = external_readers + usize::from(capture_rule.is_some());
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &intents,
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )?;
    file.owner.derived().set_optional_limit(
        ryo_execution_store::application::file_dag::derived::owner::index::types::ByteLimit(0),
    )?;
    assert_eq!(
        file.prepared_roots
            .iter()
            .filter(|&&id| id == producer)
            .count(),
        1,
        "deduplication applies to queue entries, not external reader obligations"
    );
    assert_eq!(
        file.execution
            .retry_checkpoint(file.execution.file_state(), producer)?
            .remaining
            .0,
        retained_readers + 2,
        "both actual Join consumers and every selected alias count"
    );
    let columns = [
        FactColumn(vec![to_bits(Cell::File(FileId(0))).0; 2]),
        FactColumn(vec![
            to_bits(Cell::Number(Integer(2))).0,
            to_bits(Cell::Number(Integer(3))).0,
        ]),
        FactColumn(vec![
            to_bits(Cell::Number(Integer(9))).0,
            to_bits(Cell::Number(Integer(7))).0,
        ]),
    ];
    let reads = std::cell::Cell::new(0);
    let read = |relation| {
        if relation == Relation::LanguageTarget {
            reads.set(reads.get() + 1);
            Ok(Rows::Stored(ColumnsView {
                kinds: kinds(relation),
                columns: &columns,
            }))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let demand = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    let mut cells = Vec::new();
    file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |row| {
        cells.extend_from_slice(row);
        Ok(())
    })?;
    let transferred = Table {
        kinds: kinds(Relation::LanguageTarget).to_vec(),
        count: RowCount(cells.len() / 3),
        cells,
    };
    assert_eq!(
        transferred.count.0, 2,
        "canonical Union keeps its two distinct rows"
    );
    let checkpoint = file
        .execution
        .retry_checkpoint(file.execution.file_state(), producer)?;
    assert_eq!(checkpoint.attempt, AttemptCheckpoint::Published);
    assert_eq!(
        checkpoint.remaining.0, retained_readers,
        "both internal readers released exactly once; external aliases remain live"
    );
    let required = file.owner.derived().working_bytes();
    assert!(
        required > 0,
        "external publication retains the acquired required equality lease"
    );
    let backing = file
        .owner
        .derived()
        .view_witness(ryo_derived::domain::request::types::DerivedKind::Equality)
        .expect("actual equality backing");
    assert!(backing.liveness() == BackingLiveness::Alive);
    let builds = file.owner.derived().counts();
    assert_eq!(
        builds.0, 1,
        "one actual equality construction across submitted roots"
    );
    if !matches!(exit, ExternalRootExit::ProviderRefusal) {
        assert!(builds.1 > 0, "second Join uses the shared backing");
    }
    let counts = file.readiness_counts();
    let source_reads = reads.get();
    let mut callbacks = 0;
    let delivered = file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |_| {
        callbacks += 1;
        if matches!(
            exit,
            ExternalRootExit::RejectedDelivery | ExternalRootExit::CaptureCallbackRefusal
        ) {
            Err(OperatingError::WrongBinding)
        } else {
            Ok(())
        }
    });
    if matches!(
        exit,
        ExternalRootExit::RejectedDelivery | ExternalRootExit::CaptureCallbackRefusal
    ) {
        assert!(matches!(delivered, Err(OperatingError::WrongBinding)));
        assert_eq!(callbacks, 1, "delivery refusal stops that sink");
        let mut repeated = 0;
        file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |_| {
            repeated += 1;
            Ok(())
        })?;
        assert_eq!(
            repeated, 2,
            "authorized delivery retry replays the full canonical root"
        );
    } else {
        delivered?;
        assert_eq!(callbacks, 2);
    }
    if external_readers == 2 {
        let alias = file.rule(RuleIndex(1), demand, &read, &mut symbols)?;
        let RootOutcome::Complete(alias) = alias.emit else {
            panic!("duplicate external consumer")
        };
        assert_eq!(
            alias, transferred,
            "both external consumers retain complete canonical rows"
        );
    } else {
        assert!(matches!(
            file.rule(RuleIndex(1), demand, &read, &mut symbols),
            Err(OperatingError::NotRun)
        ));
    }
    let final_consumer = file.rule(RuleIndex(3), demand, &read, &mut symbols)?;
    match exit {
        ExternalRootExit::SemanticFailure | ExternalRootExit::CaptureCollectedFailure => {
            assert!(matches!(
                final_consumer.emit,
                RootOutcome::Failed(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }))
            ))
        }
        ExternalRootExit::ProviderRefusal => assert!(matches!(
            final_consumer.emit,
            RootOutcome::Failed(OperatingError::NotRead)
        )),
        _ => assert!(matches!(final_consumer.emit, RootOutcome::Complete(_))),
    }
    assert_eq!(
        file.readiness_counts(),
        counts,
        "all external delivery is terminal replay"
    );
    assert_eq!(reads.get(), source_reads);
    assert_eq!(file.owner.derived().counts(), builds);
    assert_eq!(
        file.owner.derived().working_bytes(),
        required,
        "duplicate delivery, failed roots and sink refusal cannot retire another reader's backing"
    );
    assert_eq!(
        file.execution
            .retry_checkpoint(file.execution.file_state(), producer)?,
        checkpoint
    );
    if let Some(capture_rule) = capture_rule {
        if matches!(exit, ExternalRootExit::CaptureCollectedFailure) {
            for _ in 0..2 {
                let failed = file.rule(capture_rule, demand, &read, &mut symbols)?;
                assert!(
                    matches!(
                        failed.emit,
                        RootOutcome::Failed(OperatingError::Table(RowsError::NoSuchColumn {
                            index: 99
                        }))
                    ),
                    "terminal semantic error survives repeated collected delivery"
                );
            }
        }
        let mut arena = vec![];
        let mut places = vec![];
        let mut words = ryo_relations::domain::relations::types::Words::default();
        for _ in 0..2 {
            let mut captures = file.capture_frontiers(
                capture_rule,
                ryo_values::domain::stored_facts::types::Bucket(0),
                &read,
                &mut symbols,
                ryo_execution_domain::domain::frontier_capture::types::CaptureScope {
                    file: FileId(0),
                    rule_digest: Digest([0; 16]),
                    worker: ryo_relations::domain::relations::types::WorkerNumber(0),
                    retained: ryo_execution_domain::domain::finish::types::Retained {
                        arena: ryo_execution_domain::domain::finish::types::Arena(&mut arena),
                        places: &mut places,
                    },
                    words: &mut words,
                },
            )?;
            assert_eq!(
                captures.len(),
                1,
                "one declared physical frontier obligation"
            );
            let captured = captures.pop().unwrap();
            let input = captured.input;
            let captured = captured.outcome?;
            let decoded = ryo_values::domain::table_words::service::decode_complete(
                &captured.words,
                &mut vec![],
            )
            .expect("actual captured canonical publication decodes");
            assert_eq!(
                decoded, transferred,
                "failure/refusal cannot corrupt the independent frontier"
            );
            file.commit_frontier(capture_rule, input)?;
            assert!(
                backing.liveness() == BackingLiveness::Alive,
                "capture/commit cannot consume another external lease"
            );
            assert_eq!(file.owner.derived().working_bytes(), required);
            assert_eq!(
                file.readiness_counts(),
                counts,
                "capture replay never reschedules semantics"
            );
            assert_eq!(file.owner.derived().counts(), builds);
            assert_eq!(reads.get(), source_reads);
        }
    }
    // Retain the owner solely to inspect real byte accounting after finish. A
    // Weak-only check would miss a stale cache retaining its required charge.
    let owner = Rc::clone(&file.owner);
    file.finish()?;
    assert_eq!(
        owner.derived().working_bytes(),
        0,
        "finish releases required lease accounting"
    );
    assert_eq!(
        owner.derived().owned_bytes(),
        0,
        "finish releases retained and required resources"
    );
    assert!(
        backing.liveness() == BackingLiveness::Released,
        "finish drops the same actual shared backing"
    );
    assert_eq!(
        transferred.count.0, 2,
        "transferred rows survive file retirement"
    );
    Ok(())
}
