//! Actual external aliases retain one production backing through terminal exits.
use super::super::super::*;
use super::super::fixtures::{FinalConsumer, ProjectionOrder, shared_plan};
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::rows::errors::TableModelError;

#[derive(Clone, Copy, Debug)]
enum ExternalRootExit {
    Finish,
    RejectedDelivery,
    SemanticFailure,
    ProviderRefusal,
    SkippedAlias,
    UndemandedAlias,
}

/// Two diagnostic obligations share an external Union root. Distinct Join
/// consumers acquire the real producer's equality lease during the same batch.
/// Unlike an internal-only producer, the externally held Union must stay live
/// after both Join readers release and until the file ends.
fn external_root_exit(exit: ExternalRootExit) -> Result<(), OperatingError> {
    let derived = crate::application::file_dag::testing::program_derived_port();
    let mut external = shared_plan(ProjectionOrder::Forward, FinalConsumer::Completes);
    external.steps.truncate(2);
    external.emit.step = StepId(1);
    let mut rules = vec![rule(); 4];
    for (at, rule) in rules.iter_mut().enumerate() {
        rule.id = RuleId(format!("external-root-{at}").into());
    }
    let prepared = facade::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]; 4],
            plans: vec![
                external.clone(),
                external,
                shared_plan(ProjectionOrder::Forward, FinalConsumer::Completes),
                shared_plan(
                    ProjectionOrder::Reversed,
                    match exit {
                        ExternalRootExit::SemanticFailure => FinalConsumer::InvalidTerminal,
                        ExternalRootExit::ProviderRefusal => FinalConsumer::RefusingProvider,
                        _ => FinalConsumer::Completes,
                    },
                ),
            ],
            bindings: vec![vec![]; 4],
        },
        &rules,
        &vec![Digest([0; 16]); 4],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let root = |rule: RuleIndex| *prepared.program().rules()[rule.0].emit.as_ref().unwrap();
    let producer = root(RuleIndex(0));
    assert_eq!(
        producer,
        root(RuleIndex(1)),
        "actual compiled external root is shared"
    );
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
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
        ExternalRootExit::SkippedAlias => {
            intents[1] = FileRuleIntent::Skipped(
                crate::application::file_dag::program::admission::ApplicabilityReason::DiagnosticNotApplicable,
            );
        }
        ExternalRootExit::UndemandedAlias => intents[1] = FileRuleIntent::Undemanded,
        _ => {}
    }
    let external_readers = if matches!(
        exit,
        ExternalRootExit::SkippedAlias | ExternalRootExit::UndemandedAlias
    ) {
        1
    } else {
        2
    };
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
        b"",
        true,
        &mut symbols,
        None,
        None,
    )?;
    file.owner.derived.set_optional_limit(0)?;
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
        external_readers + 2,
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
        checkpoint.remaining.0, external_readers,
        "both internal readers released exactly once; external aliases remain live"
    );
    let required = file.owner.derived.working_bytes();
    assert!(
        required > 0,
        "external publication retains the acquired required equality lease"
    );
    let backing = file
        .owner
        .derived
        .view_weak(ryo_derived::domain::contract::types::DerivedKind::Equality)
        .expect("actual equality backing");
    assert!(backing.upgrade().is_some());
    let builds = file.owner.derived.counts();
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
        if matches!(exit, ExternalRootExit::RejectedDelivery) {
            Err(OperatingError::WrongBinding)
        } else {
            Ok(())
        }
    });
    if matches!(exit, ExternalRootExit::RejectedDelivery) {
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
        ExternalRootExit::SemanticFailure => assert!(matches!(
            final_consumer.emit,
            RootOutcome::Failed(OperatingError::Table(TableModelError::NoSuchColumn {
                index: 99
            }))
        )),
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
    assert_eq!(file.owner.derived.counts(), builds);
    assert_eq!(
        file.owner.derived.working_bytes(),
        required,
        "duplicate delivery, failed roots and sink refusal cannot retire another reader's backing"
    );
    assert_eq!(
        file.execution
            .retry_checkpoint(file.execution.file_state(), producer)?,
        checkpoint
    );
    // Retain the owner solely to inspect real byte accounting after finish. A
    // Weak-only check would miss a stale cache retaining its required charge.
    let owner = Rc::clone(&file.owner);
    file.finish()?;
    assert_eq!(
        owner.derived.working_bytes(),
        0,
        "finish releases required lease accounting"
    );
    assert_eq!(
        owner.derived.owned_bytes(),
        0,
        "finish releases retained and required resources"
    );
    assert!(
        backing.upgrade().is_none(),
        "finish drops the same actual shared backing"
    );
    assert_eq!(
        transferred.count.0, 2,
        "transferred rows survive file retirement"
    );
    Ok(())
}

#[test]
fn deduplicated_external_roots_keep_real_leases_through_replay_failure_and_finish() {
    for exit in [
        ExternalRootExit::Finish,
        ExternalRootExit::RejectedDelivery,
        ExternalRootExit::SemanticFailure,
        ExternalRootExit::ProviderRefusal,
        ExternalRootExit::SkippedAlias,
        ExternalRootExit::UndemandedAlias,
    ] {
        external_root_exit(exit).unwrap_or_else(|error| panic!("{exit:?}: {error:?}"));
    }
}
