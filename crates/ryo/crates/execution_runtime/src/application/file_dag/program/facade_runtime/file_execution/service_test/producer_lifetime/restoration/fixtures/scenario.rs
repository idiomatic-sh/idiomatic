//! The restoration scenario every exit runs.
use super::captured::CapturedProject;
use super::*;
pub(in super::super) fn exercise(exit: Exit) -> Result<(), OperatingError> {
    let mut symbols = Symbols::default();
    let captured = CapturedProject::new(exit, &mut symbols)?;
    let named = captured
        .prepared
        .observation_metadata()
        .expect("actual named program");
    for (slot, _) in named.slots.iter().enumerate() {
        let union = *captured.prepared.program().observation_regions()[slot]
            .as_ref()
            .expect("valid compiled named union");
        assert_eq!(
            captured.prepared.program().regions()[union.0].phase(),
            StepScope::CrossFile,
            "restored project fixture must demand project-scoped unions, not unresolved file frontiers"
        );
    }
    let partials = Partials::default();
    let owner = Rc::clone(captured.phase.owner());
    assert!(!owner.derived().index_observation().available);
    let demand = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::Required,
    };
    let runs = (1..5)
        .map(|at| {
            captured.open(
                RuleIndex(at),
                &partials,
                &mut symbols,
                if at == 4 && matches!(exit, Exit::RefusedBinding | Exit::CancelledDelivery) {
                    BoundSupply::Refused
                } else {
                    BoundSupply::Restored
                },
            )
        })
        .collect::<Vec<_>>();
    // Observe the real healthy Join, not whichever Equality happens to be
    // present in this owner. Its physical producer/consumer and population all
    // come from the same compiled rule and actual ProjectRun binding.
    let physical = |step: StepId| {
        let original = captured.prepared.program().rules()[3].original_steps[step.0 as usize]
            .as_ref()
            .expect("actual healthy project instruction");
        captured.prepared.program().logical_regions()[original.logical.0]
    };
    let producer = physical(StepId(1));
    let consumer = physical(StepId(2));
    assert_ne!(
        producer, consumer,
        "the observed backing belongs to a real upstream producer"
    );
    assert!(
        captured.prepared.program().regions()[consumer.0]
            .dependencies()
            .contains(&producer),
        "the observed Join really consumes this compiled Union"
    );
    let join_logical = captured.prepared.program().rules()[3].original_steps[2]
        .as_ref()
        .expect("actual Join metadata")
        .logical;
    let region = &captured.prepared.program().regions()[consumer.0];
    let join = region
        .instructions()
        .iter()
        .find(|instruction| instruction.logical() == join_logical)
        .expect("actual compiled Join");
    let equality = join
        .contract()
        .derived
        .slots
        .iter()
        .flatten()
        .find(|group| {
            group.primary.plan.kind == ryo_derived::domain::request::types::DerivedKind::Equality
        })
        .expect("actual Join declares Equality");
    assert_eq!(
        equality.primary.len,
        ryo_execution_domain::domain::derived_views::types::InputsUsed(1)
    );
    assert_eq!(
        region.instructions().first().map(|first| first.logical()),
        Some(join_logical),
        "observed Equality must read the region's own inputs, a real upstream publication"
    );
    assert_eq!(
        region.inputs()[equality.primary.inputs[0].0],
        ryo_execution_domain::domain::executable_program::types::InputSource::Region(producer),
        "observe the precise population demanded by the compiled Equality contract"
    );
    runs[2]
        .as_ref()
        .map_err(|error| error.clone())?
        .observe_producer_releases(RuleIndex(3), producer, &[consumer])?;
    let roots = runs
        .into_iter()
        .enumerate()
        .map(|(at, run)| ProjectRoot {
            rule: RuleIndex(at + 1),
            demand,
            run,
        })
        .collect::<Vec<_>>();
    let guards = std::cell::Cell::new(0);
    let batch = evaluate_roots(roots, &mut symbols, |rule| {
        guards.set(guards.get() + 1);
        if rule == RuleIndex(4) && matches!(exit, Exit::CancelledDelivery) {
            Err(OperatingError::NotRead)
        } else {
            Ok(())
        }
    });
    let outcomes = match batch {
        ProjectBatch::Complete(outcomes) => {
            assert!(
                !matches!(exit, Exit::CancelledDelivery),
                "cancellation must refuse its actual delivery"
            );
            outcomes
        }
        ProjectBatch::Refused { completed, error } => {
            assert!(matches!(exit, Exit::CancelledDelivery));
            assert!(matches!(error, OperatingError::NotRead));
            assert_eq!(
                completed.len(),
                3,
                "earlier delivered aliases survive later refusal"
            );
            completed
        }
    };
    let mut transferred = vec![];
    for (at, outcome) in outcomes.into_iter().enumerate() {
        let outcome = outcome?;
        match outcome.emit {
            RootOutcome::Complete(table) => transferred.push(table),
            RootOutcome::Failed(OperatingError::Table(RowsError::NoSuchColumn { index: 99 })) => {
                assert_eq!(at, 3);
                assert!(matches!(exit, Exit::SemanticFailure));
            }
            RootOutcome::Failed(OperatingError::WrongBinding) => {
                assert_eq!(at, 3);
                assert!(matches!(exit, Exit::RefusedBinding));
            }
            _ => panic!("{exit:?}: actual restored delivery lost its terminal result"),
        }
    }
    assert_eq!(
        transferred[0], transferred[1],
        "duplicate named obligations deliver the same canonical rows"
    );
    assert_eq!(transferred[0].count, RowCount(2));
    let observed = captured.open(RuleIndex(3), &partials, &mut symbols, BoundSupply::Restored)?;
    let events = observed.take_producer_releases(RuleIndex(3))?;
    drop(observed);
    assert_eq!(
        events.len(),
        2,
        "one actual Join release, before and after, in its bound population"
    );
    let (before, after) = (&events[0], &events[1]);
    assert_eq!(before.consumer, consumer);
    assert_eq!(after.consumer, consumer);
    assert_eq!(before.point, ConsumerReleasePoint::Before);
    assert_eq!(after.point, ConsumerReleasePoint::After);
    assert_eq!(before.checkpoint.attempt, AttemptCheckpoint::Published);
    assert!(
        before.checkpoint.remaining.0 > 0,
        "the producer is still needed during the actual Join"
    );
    assert!(
        before.index_liveness == IndexLiveness::Alive,
        "required Equality backing is alive at its actual consumer boundary"
    );
    assert!(
        before.working_bytes.0 > 0,
        "the actual required lookup is charged while needed"
    );
    let backing = before
        .index
        .as_ref()
        .expect("backing of the qualified producer");
    let released = after
        .index
        .as_ref()
        .expect("same backing remains observable after retirement");
    assert!(
        backing.compare(released) == BackingIdentity::Same,
        "release observes the same actual allocation, not another population's index"
    );
    assert_eq!(
        after.checkpoint.remaining.0, 0,
        "this Join is the final compute consumer"
    );
    assert_eq!(after.checkpoint.attempt, AttemptCheckpoint::Terminal);
    assert!(
        after.index_liveness == IndexLiveness::Released,
        "the final compute release drops the lookup before output delivery"
    );
    assert_eq!(
        after.working_bytes.0, 0,
        "true last use releases its required charge"
    );
    assert!(backing.liveness() == BackingLiveness::Released);
    let counts = captured.phase.readiness_counts();
    let builds = owner.derived().counts();
    // The previous oracle incorrectly required a lookup to outlive the batch.
    // Returned Tables and terminal replay have no dependency on that lookup.
    assert_eq!(
        owner.derived().working_bytes(),
        0,
        "all compute lookup leases are already released"
    );
    assert!(counts.evaluations.0 > 0);
    assert!(
        builds.0 > 0,
        "the actual restored population acquires required equality backing"
    );

    assert_eq!(owner.derived().retained_bytes(), 0);

    for at in [1, 2, 4] {
        if at == 4 && matches!(exit, Exit::CancelledDelivery) {
            continue;
        }
        let mut run = captured.open(
            RuleIndex(at),
            &partials,
            &mut symbols,
            if at == 4 && matches!(exit, Exit::RefusedBinding | Exit::CancelledDelivery) {
                BoundSupply::Refused
            } else {
                BoundSupply::Restored
            },
        )?;
        for _ in 0..2 {
            let replay = run.rule(RuleIndex(at), demand, &mut symbols)?;
            match (at, exit, replay.emit) {
                (
                    4,
                    Exit::SemanticFailure,
                    RootOutcome::Failed(OperatingError::Table(RowsError::NoSuchColumn {
                        index: 99,
                    })),
                )
                | (4, Exit::RefusedBinding, RootOutcome::Failed(OperatingError::WrongBinding)) => {}
                (_, _, RootOutcome::Complete(table)) => assert!(table.count.0 > 0),
                _ => panic!(
                    "{exit:?}: repeated project delivery must preserve its exact terminal outcome"
                ),
            }
            assert_eq!(
                captured.phase.readiness_counts(),
                counts,
                "replay cannot schedule a terminal restored root"
            );
            assert_eq!(
                owner.derived().counts(),
                builds,
                "replay cannot reacquire a restored view"
            );
            assert_eq!(owner.derived().working_bytes(), 0);
            assert!(
                backing.liveness() == BackingLiveness::Released,
                "copied output replay must not reacquire the retired lookup"
            );
        }
    }
    captured.phase.finish()?;
    assert_eq!(owner.derived().working_bytes(), 0);
    assert_eq!(owner.derived().owned_bytes(), 0);
    assert!(
        backing.liveness() == BackingLiveness::Released,
        "phase finish cannot revive the already released compute backing"
    );
    assert_eq!(
        owner.derived().counts(),
        builds,
        "finish cannot dispatch or acquire work"
    );
    assert_eq!(
        transferred[0].count,
        RowCount(2),
        "transferred rows outlive phase retirement"
    );
    Ok(())
}
