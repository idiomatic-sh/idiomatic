//! The project producer lifetime scenario every exit runs.
use super::super::super::fixtures::scenario::Delivery as Outcome;
use super::*;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use std::borrow::Cow;

/// The phase owner's derived state: whether its index observation is available, its retained,
/// working and owned bytes, and its equality builds.
#[derive(Debug, PartialEq)]
pub(in super::super) struct Owner {
    pub(in super::super) index_available: bool,
    pub(in super::super) retained: usize,
    pub(in super::super) working: usize,
    pub(in super::super) owned: usize,
    pub(in super::super) builds: u64,
}

/// What the project phase observed from preparation through its batch, replays and finish.
#[derive(Debug, PartialEq)]
pub(in super::super) struct Phase {
    /// Whether every compiled step keeps its instruction in its region.
    pub(in super::super) instructions_kept: bool,
    pub(in super::super) at_start: Owner,
    /// The project runs of the four rules that bound.
    pub(in super::super) bound: usize,
    /// The batch's refusal, when refused, and the guard calls it made.
    pub(in super::super) refusal: Option<OperatingError>,
    pub(in super::super) guards: usize,
    /// The batch's evaluations.
    pub(in super::super) evaluations: u64,
    pub(in super::super) after_batch: Owner,
    /// Whether the equality backing is alive after the batch, when there is one.
    pub(in super::super) backing_alive: Option<bool>,
    /// Each root the batch completed, in rule order.
    pub(in super::super) delivered: Cow<'static, [Outcome]>,
    /// Whether the first two transferred tables are equal, when two were transferred.
    pub(in super::super) duplicates_equal: Option<bool>,
    /// Each replay of rules 0, 1 and 3, twice each.
    pub(in super::super) replays: Cow<'static, [Outcome]>,
    /// Whether every replay left readiness, builds, working bytes and the alive backing unchanged.
    pub(in super::super) replays_quiet: bool,
    pub(in super::super) finished: Owner,
    /// Whether the backing is released (or was never built) after finish.
    pub(in super::super) backing_released: bool,
    /// The rows of each transferred table after the phase finished.
    pub(in super::super) transferred: Cow<'static, [usize]>,
}

fn owner_state(
    owner: &ryo_execution_store::application::file_dag::relations::service::RelationOwner,
) -> Owner {
    Owner {
        index_available: owner.derived().index_observation().available,
        retained: owner.derived().retained_bytes().0,
        working: owner.derived().working_bytes().0,
        owned: owner.derived().owned_bytes().0,
        builds: owner.derived().counts().0.0,
    }
}

fn frontier_table(distinct: bool) -> Table {
    Table {
        kinds: kinds(Relation::LanguageTarget).to_vec(),
        cells: vec![
            Cell::File(FileId(0)),
            Cell::Number(Integer(if distinct { 8 } else { 2 })),
            Cell::Number(Integer(9)),
            Cell::File(FileId(0)),
            Cell::Number(Integer(3)),
            Cell::Number(Integer(7)),
        ],
        count: RowCount(2),
    }
}

pub(in super::super) fn exercise(
    exit: PhaseExit,
    delivery: Delivery,
) -> Result<Phase, OperatingError> {
    let PreparedCase {
        prepared,
        services,
        corpus,
    } = prepare_case(exit)?;
    let mut instructions_kept = true;
    for (rule, program) in prepared.program().rules().iter().enumerate() {
        for (step, origin) in program.original_steps.iter().enumerate() {
            let origin = origin.as_ref().unwrap_or_else(|error| {
                panic!("{exit:?}: compiled rule {rule} step {step}: {error:?}")
            });
            let region = prepared.program().logical_regions()[origin.logical.0];
            instructions_kept &= prepared.program().regions()[region.0]
                .instructions()
                .iter()
                .any(|instruction| instruction.logical() == origin.logical);
        }
    }
    let phase = crate::application::file_dag::program::facade_runtime::service::publication_owner(
        &prepared,
        &services,
        Arc::new(crate::outbound::digest::fixtures::fake::DigestFake),
    )?;
    let owner = Rc::clone(phase.owner());
    // A project phase has no file-cache backend. Required derived views still
    // use this real owner; optional retention requires an installed file cache.
    // Configuring its absent backend would refuse before any root is submitted.
    let at_start = owner_state(&owner);
    let required =
        crate::application::file_dag::program::facade_runtime::service::required_partitions(
            &prepared,
            RuleIndex(0),
            &corpus,
            &[],
            &[],
            &[],
            &[],
            &services,
        )?;
    let source = PlanInput::Relation(Relation::LanguageTarget);
    let source_root = prepared.program().rules()[0].frontier[0].clone();
    let partials = Partials::default();
    let mut symbols = Symbols::default();
    let mut runs = Vec::new();
    for at in 0..4 {
        let distinct = at == 3 && matches!(exit, PhaseExit::RefuseLaterCohort);
        let table = frontier_table(distinct);
        runs.push(
            crate::application::file_dag::program::facade_runtime::project_execution::service::open_project(
                &prepared,
                required.clone(),
                ScopePartials::Project {
                    evaluation: &partials,
                    publication: &partials,
                },
                ProjectFrontiers {
                    publication: &phase,
                    ordinary: prepared_frontiers(
                        RuleIndex(at),
                        vec![FrontierSource {
                            input: source,
                            root: source_root.clone(),
                        }]
                        .into(),
                        vec![FrontierInput::Decoded {
                            frontier: Frontier {
                                input: source,
                                table: Arc::new(table),
                            },
                            coverage: CoverageState::Complete,
                        }]
                        .into(),
                        Arc::clone(prepared.program()),
                        required.clone(),
                        vec![].into(),
                    ),
                    bound_inputs: &[],
                },
                &mut symbols,
                &[],
                Arc::clone(&services),
            ),
        );
    }
    let bound = runs
        .iter()
        .filter(|run| run.as_ref().err().is_none())
        .count();
    let demand = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    let roots: Vec<_> = runs
        .into_iter()
        .enumerate()
        .map(|(at, run)| ProjectRoot {
            rule: RuleIndex(at),
            demand,
            run,
        })
        .collect();
    let guards = std::cell::Cell::new(0);
    let result = evaluate_roots(roots, &mut symbols, |rule| {
        guards.set(guards.get() + 1);
        if matches!(exit, PhaseExit::RefuseBeforeWork)
            || (matches!(exit, PhaseExit::RefuseLaterCohort) && rule == RuleIndex(3))
        {
            Err(OperatingError::WrongBinding)
        } else {
            Ok(())
        }
    });
    let (completed, refusal) = match result {
        ProjectBatch::Complete(completed) => (completed, None),
        ProjectBatch::Refused { completed, error } => (completed, Some(error)),
    };
    let counts = phase.readiness_counts();
    let builds = owner.derived().counts();
    let live = owner.derived().working_bytes();
    let after_batch = owner_state(&owner);
    let backing = owner
        .derived()
        .view_witness(ryo_derived::domain::request::types::DerivedKind::Equality);
    let backing_alive = backing
        .as_ref()
        .map(|backing| backing.liveness() == BackingLiveness::Alive);
    let mut transferred = Vec::new();
    let mut delivered = Vec::new();
    for result in completed {
        match result?.emit {
            RootOutcome::Complete(table) => {
                delivered.push(Outcome::Rows(table.count.0));
                transferred.push(table);
            }
            other => delivered.push(Outcome::from(other)),
        }
    }
    let duplicates_equal = (transferred.len() >= 2).then(|| transferred[0] == transferred[1]);
    let mut replays = Vec::new();
    let mut replays_quiet = true;
    if matches!(delivery, Delivery::Replayed) && !matches!(exit, PhaseExit::RefuseBeforeWork) {
        // A new public delivery object must bind the same actual population;
        // it cannot revive terminal roots or consume another alias's obligation.
        for at in [0, 1, 3] {
            if at == 3 && matches!(exit, PhaseExit::RefuseLaterCohort) {
                continue;
            }
            let mut replay = crate::application::file_dag::program::facade_runtime::project_execution::service::open_project(
                &prepared,
                required.clone(),
                ScopePartials::Project {
                    evaluation: &partials,
                    publication: &partials,
                },
                ProjectFrontiers {
                    publication: &phase,
                    ordinary: prepared_frontiers(
                        RuleIndex(at),
                        vec![FrontierSource {
                            input: source,
                            root: source_root.clone(),
                        }]
                        .into(),
                        vec![FrontierInput::Decoded {
                            frontier: Frontier {
                                input: source,
                                table: Arc::new(frontier_table(false)),
                            },
                            coverage: CoverageState::Complete,
                        }].into(),
                        Arc::clone(prepared.program()),
                        required.clone(),
                        vec![].into(),
                    ),
                    bound_inputs: &[],
                },
                &mut symbols,
                &[],
                Arc::clone(&services),
            )?;
            for _ in 0..2 {
                let outcome = replay.rule(RuleIndex(at), demand, &mut symbols)?;
                replays.push(Outcome::from(outcome.emit));
                replays_quiet &= phase.readiness_counts() == counts
                    && owner.derived().counts() == builds
                    && owner.derived().working_bytes() == live
                    && backing
                        .as_ref()
                        .is_some_and(|backing| backing.liveness() == BackingLiveness::Alive);
            }
        }
    }
    phase.finish()?;
    Ok(Phase {
        instructions_kept,
        at_start,
        bound,
        refusal,
        guards: guards.get(),
        evaluations: counts.evaluations.0,
        after_batch,
        backing_alive,
        delivered: Cow::Owned(delivered),
        duplicates_equal,
        replays: Cow::Owned(replays),
        replays_quiet,
        finished: owner_state(&owner),
        backing_released: backing
            .is_none_or(|backing| backing.liveness() == BackingLiveness::Released),
        transferred: transferred.iter().map(|table| table.count.0).collect(),
    })
}
