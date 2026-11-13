//! The restoration scenario every exit runs.
use super::super::super::fixtures::scenario::{Delivery, Release};
use super::captured::{Built, CapturedProject};
use super::*;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use std::borrow::Cow;

/// How the compiled healthy rule reads its producer: whether the observed producer is another
/// region than the Join's, whether the Join's region depends on it, the inputs its Equality uses,
/// whether the Join is its region's first instruction, and whether the Equality's input is the producer.
#[derive(Debug, PartialEq)]
pub(in super::super) struct Reading {
    pub(in super::super) upstream: bool,
    pub(in super::super) depends: bool,
    pub(in super::super) equality_inputs: usize,
    pub(in super::super) join_first: bool,
    pub(in super::super) reads_producer: bool,
}

/// The owner's working, retained (or owned) bytes, and whether the equality backing is released.
#[derive(Debug, PartialEq)]
pub(in super::super) struct Held {
    pub(in super::super) working: usize,
    pub(in super::super) kept: usize,
    pub(in super::super) backing_released: bool,
}

/// What the restored project observed through its batch, replay and finish.
#[derive(Debug, PartialEq)]
pub(in super::super) struct Restored {
    pub(in super::super) built: Built,
    /// The phase of each named slot's union region.
    pub(in super::super) union_phases: Cow<'static, [StepScope]>,
    /// Whether the derived index observation was available before any rule ran.
    pub(in super::super) index_available: bool,
    pub(in super::super) reading: Reading,
    /// The batch's refusal, when it was refused.
    pub(in super::super) refusal: Option<OperatingError>,
    /// Each root the batch delivered, in rule order.
    pub(in super::super) delivered: Cow<'static, [Delivery]>,
    /// Whether the two duplicate named obligations delivered the same rows.
    pub(in super::super) duplicates_equal: bool,
    /// The Join's release events, and the one release they make.
    pub(in super::super) release_events: usize,
    pub(in super::super) release: Release,
    /// Whether both events name the observed Join and observe the same backing.
    pub(in super::super) observed_join: bool,
    /// Whether the batch ran evaluations, and the equality builds it made.
    pub(in super::super) evaluated: bool,
    pub(in super::super) equality_builds: u64,
    /// The owner after the batch (kept is retained bytes).
    pub(in super::super) after_batch: Held,
    /// Each replay of rules 1, 2 and 4, twice each.
    pub(in super::super) replays: Cow<'static, [Delivery]>,
    /// Whether every replay left readiness, builds, working bytes and the released backing unchanged.
    pub(in super::super) replays_quiet: bool,
    /// The owner after the phase finished (kept is owned bytes), and whether builds are unchanged.
    pub(in super::super) finished: Held,
    pub(in super::super) finish_builds_nothing: bool,
    /// The rows of the first transferred table after retirement.
    pub(in super::super) transferred_rows: usize,
}

pub(in super::super) fn exercise(exit: Exit) -> Result<Restored, OperatingError> {
    let mut symbols = Symbols::default();
    let captured = CapturedProject::new(exit, &mut symbols)?;
    let named = captured
        .prepared
        .observation_metadata()
        .expect("actual named program");
    let union_phases = (0..named.slots.len())
        .map(|slot| {
            let union = *captured.prepared.program().observation_regions()[slot]
                .as_ref()
                .expect("valid compiled named union");
            captured.prepared.program().regions()[union.0].phase()
        })
        .collect();
    let partials = Partials::default();
    let owner = Rc::clone(captured.phase.owner());
    let index_available = owner.derived().index_observation().available;
    let demand = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::Required,
    };
    let supply = |at: usize| {
        if at == 4 && matches!(exit, Exit::RefusedBinding | Exit::CancelledDelivery) {
            BoundSupply::Refused
        } else {
            BoundSupply::Restored
        }
    };
    let runs = (1..5)
        .map(|at| captured.open(RuleIndex(at), &partials, &mut symbols, supply(at)))
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
    let reading = Reading {
        upstream: producer != consumer,
        depends: region.dependencies().contains(&producer),
        equality_inputs: usize::from(equality.primary.len.0),
        join_first: region.instructions().first().map(|first| first.logical())
            == Some(join_logical),
        reads_producer: region.inputs()[equality.primary.inputs[0].0]
            == ryo_execution_domain::domain::executable_program::types::InputSource::Region(
                producer,
            ),
    };
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
    let batch = evaluate_roots(roots, &mut symbols, |rule| {
        if rule == RuleIndex(4) && matches!(exit, Exit::CancelledDelivery) {
            Err(OperatingError::NotRead)
        } else {
            Ok(())
        }
    });
    let (outcomes, refusal) = match batch {
        ProjectBatch::Complete(outcomes) => (outcomes, None),
        ProjectBatch::Refused { completed, error } => (completed, Some(error)),
    };
    let mut transferred = vec![];
    let mut delivered = vec![];
    for outcome in outcomes {
        match outcome?.emit {
            RootOutcome::Complete(table) => {
                delivered.push(Delivery::Rows(table.count.0));
                transferred.push(table);
            }
            other @ (RootOutcome::Partial { .. }
            | RootOutcome::Skipped(_)
            | RootOutcome::Unknown(_)
            | RootOutcome::Failed(_)
            | RootOutcome::NoQuery
            | RootOutcome::Deferred { .. }) => delivered.push(Delivery::from(other)),
        }
    }
    let duplicates_equal = transferred[0] == transferred[1];
    let observed = captured.open(RuleIndex(3), &partials, &mut symbols, BoundSupply::Restored)?;
    let events = observed.take_producer_releases(RuleIndex(3))?;
    drop(observed);
    let (before, after) = (&events[0], &events[1]);
    let backing = before
        .index
        .as_ref()
        .expect("backing of the qualified producer");
    let released = after
        .index
        .as_ref()
        .expect("same backing remains observable after retirement");
    let release = Release {
        remaining: (before.checkpoint.remaining.0, after.checkpoint.remaining.0),
        attempt: (before.checkpoint.attempt, after.checkpoint.attempt),
        alive: (
            before.index_liveness == IndexLiveness::Alive,
            after.index_liveness == IndexLiveness::Alive,
        ),
        holds_bytes: (before.working_bytes.0 > 0, after.working_bytes.0 > 0),
        paired: before.point == ConsumerReleasePoint::Before
            && after.point == ConsumerReleasePoint::After
            && before.consumer == after.consumer,
    };
    let observed_join =
        before.consumer == consumer && backing.compare(released) == BackingIdentity::Same;
    let counts = captured.phase.readiness_counts();
    let builds = owner.derived().counts();
    // The previous oracle incorrectly required a lookup to outlive the batch.
    // Returned Tables and terminal replay have no dependency on that lookup.
    let after_batch = Held {
        working: owner.derived().working_bytes().0,
        kept: owner.derived().retained_bytes().0,
        backing_released: backing.liveness() == BackingLiveness::Released,
    };
    let mut replays = vec![];
    let mut replays_quiet = true;
    for at in [1, 2, 4] {
        if at == 4 && matches!(exit, Exit::CancelledDelivery) {
            continue;
        }
        let mut run = captured.open(RuleIndex(at), &partials, &mut symbols, supply(at))?;
        for _ in 0..2 {
            let replay = run.rule(RuleIndex(at), demand, &mut symbols)?;
            replays.push(Delivery::from(replay.emit));
            replays_quiet &= captured.phase.readiness_counts() == counts
                && owner.derived().counts() == builds
                && owner.derived().working_bytes() == 0
                && backing.liveness() == BackingLiveness::Released;
        }
    }
    captured.phase.finish()?;
    Ok(Restored {
        built: captured.built.clone(),
        union_phases,
        index_available,
        reading,
        refusal,
        delivered: Cow::Owned(delivered),
        duplicates_equal,
        release_events: events.len(),
        release,
        observed_join,
        evaluated: counts.evaluations.0 > 0,
        equality_builds: builds.0.0,
        after_batch,
        replays: Cow::Owned(replays),
        replays_quiet,
        finished: Held {
            working: owner.derived().working_bytes().0,
            kept: owner.derived().owned_bytes().0,
            backing_released: backing.liveness() == BackingLiveness::Released,
        },
        finish_builds_nothing: owner.derived().counts() == builds,
        transferred_rows: transferred[0].count.0,
    })
}
