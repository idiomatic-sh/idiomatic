//! The real file owner and its semantic port, invoked once per join observation.
use super::super::super::super::service::*;
use super::super::super::fixtures::*;
use super::provider::{Provider, provider};
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::derived::owner::fixtures::resources::RecordedDerived;
use ryo_execution_store::application::file_dag::derived::relation_owner::types::ScopePartials;
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{FileId, Value as Cell};
use ryo_values::domain::compact_table::service::admit_compact;
use ryo_values::domain::stored_facts::types::KeptWords;
use ryo_values::domain::table_words::types::DecodeMode;
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{StepId, StepScope},
    plans::types::{Operation, Plan, PlanStep},
    rows::types::{RowCount, Table},
    stored_facts::types::Digest,
};
use std::cell::Cell as Counter;
use std::{rc::Rc, sync::Arc};
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SourceAttempt {
    Available(usize),
    Refused(usize, OperatingError),
}
pub(crate) struct Observation {
    pub(crate) result: Result<Table, OperatingError>,
    pub(crate) prefix: Vec<Vec<Cell>>,
    pub(crate) reads: [usize; 2],
    pub(crate) attempts: Vec<SourceAttempt>,
    pub(crate) work: ExecutionObservation,
    pub(crate) counts: (u64, u64, u64),
    pub(crate) derived_builds: Vec<ryo_derived::domain::request::types::DerivedRequest>,
    pub(crate) instructions: usize,
    pub(crate) affected: Vec<Option<FileId>>,
    /// What the graph owner still held once the file finished.
    pub(crate) held: Held,
}
/// What the graph owner still held once the file finished: the publication ledger's bytes, the working bytes and
/// the owned index bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Held {
    pub(crate) ledger: (usize, usize, usize),
    pub(crate) working: usize,
    pub(crate) owned: usize,
}
/// What a normal finish leaves held: nothing.
pub(crate) const NOTHING_HELD: Held = Held {
    ledger: (0, 0, 0),
    working: 0,
    owned: 0,
};
/// Fixture behavior belongs to real external source/sink dependencies only.
#[derive(Default)]
pub(crate) struct Faults {
    pub(crate) derived: Arc<RecordedDerived>,
    pub(crate) left: Option<OperatingError>,
    pub(crate) right: Option<OperatingError>,
    pub(crate) sink_after: Option<usize>,
    pub(crate) optional_limit: Option<usize>,
    pub(crate) partials: Vec<ryo_execution_domain::domain::coverage::types::Partial>,
}
pub(crate) fn invoke(
    operation: &Operation,
    rows: [Rows<'_>; 2],
    symbols: &mut Symbols,
    source: &[u8],
    delivery: bool,
    port: Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
    faults: Faults,
) -> Result<Observation, OperatingError> {
    let mut steps = Vec::new();
    let (left, l) = provider(rows[0].clone(), 0, &mut steps);
    let (right, r) = provider(rows[1].clone(), 1, &mut steps);
    let mut operation = operation.clone();
    let Operation::Join {
        left: from,
        right: other,
        ..
    } = &mut operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    *from = l;
    *other = r;
    let target = steps.len();
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation,
    });
    let mut request = plan(PlanShape::ProjectRetry);
    request.steps = steps;
    request.emit.step = StepId(target as u32);
    execute(
        request,
        [left, right],
        symbols,
        source,
        delivery,
        port,
        faults,
    )
}
pub(crate) fn execute(
    request: Plan,
    providers: [Provider<'_>; 2],
    symbols: &mut Symbols,
    source: &[u8],
    delivery: bool,
    port: Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
    faults: Faults,
) -> Result<Observation, OperatingError> {
    let target = request.emit.step;
    let has_frontier = !request.frontier.is_empty();
    let derived: Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort> =
        faults.derived.clone();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            plans: std::iter::IntoIterator::into_iter(vec![request])
                .map(std::sync::Arc::new)
                .collect(),
            bindings: vec![vec![]],
            observations: vec![vec![]],
            schedule: RuleSchedule::Independent,
        },
        &[rule()],
        &[Digest([0; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let program = prepared.program();
    let logical = program.rules()[0].original_steps[target.0 as usize]
        .as_ref()
        .unwrap()
        .logical;
    let region = program.logical_regions()[logical.0];
    let instructions = program.regions()[region.0].instructions().len();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(port)),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("join-owner.py".into()),
            size: FileSize(source.len() as u64),
            modified: Nanoseconds(0),
        })],
    ));
    let reads = [Counter::new(0), Counter::new(0)];
    let attempts = std::cell::RefCell::new(Vec::new());
    let read = |relation| {
        let side = providers
            .iter()
            .position(|p| p.relation == relation)
            .ok_or(OperatingError::NotRead)?;
        reads[side].set(reads[side].get() + 1);
        let fault = if side == 0 {
            &faults.left
        } else {
            &faults.right
        };
        if let Some(error) = fault {
            attempts
                .borrow_mut()
                .push(SourceAttempt::Refused(side, error.clone()));
            return Err(error.clone());
        }
        attempts.borrow_mut().push(SourceAttempt::Available(side));
        Ok(providers[side].rows())
    };
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &faults.partials,
            publication: &faults.partials,
        },
        &[FileRuleIntent::Run {
            diagnostic: if delivery {
                DiagnosticDelivery::Forwarded
            } else {
                DiagnosticDelivery::Collected
            },
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }],
        services,
        ryo_values::domain::files::types::SourceView(source),
        ReuseEligibility::Eligible,
        symbols,
        None,
        None,
    )?;
    if let Some(limit) = faults.optional_limit {
        file.owner.derived().set_optional_limit(
            ryo_execution_store::application::file_dag::derived::owner::index::types::ByteLimit(
                limit,
            ),
        )?;
    }
    let mut prefix = Vec::new();
    let mut affected = vec![];
    let result = if delivery {
        file.emit_rows(RuleIndex(0), &read, symbols, &mut |row| {
            if faults.sink_after == Some(prefix.len()) {
                return Err(OperatingError::NotRun);
            }
            prefix.push(row.to_vec());
            Ok(())
        })
        .map(|_| Table {
            cells: prefix.iter().flatten().copied().collect(),
            count: RowCount(prefix.len()),
            ..Table::default()
        })
    } else {
        let outcome = file.rule(
            RuleIndex(0),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            symbols,
        )?;
        match outcome.emit {
            RootOutcome::Complete(table) => Ok(table),
            RootOutcome::Partial {
                table,
                affected: partial,
            } => {
                affected = partial;
                Ok(table)
            }
            RootOutcome::Unknown(error) | RootOutcome::Failed(error) => Err(error),
            _ => Err(OperatingError::NotRun),
        }
    };
    let work = file.route_observation(RuleIndex(0), target)?;
    if has_frontier {
        let mut arena = vec![];
        let mut places = vec![];
        let mut words = ryo_relations::domain::relations::types::Words::default();
        let captured = file.capture_frontiers(
            RuleIndex(0),
            ryo_values::domain::stored_facts::types::Bucket(0),
            &read,
            symbols,
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
        // A refused frontier capture contributes no frontier; an admitted one decodes.
        for capture in captured
            .into_iter()
            .filter_map(|capture| capture.outcome.ok())
        {
            admit_compact(
                ryo_values::domain::table_words::types::CompactBacking(Arc::new(KeptWords(
                    capture.words,
                ))),
                &mut vec![],
                DecodeMode::Complete,
            )
            .expect("actual captured words decode");
        }
    }
    let counts = file.owner.derived().counts();
    let owner = Rc::clone(&file.owner);
    file.finish()?;
    let held = Held {
        ledger: owner.derived().publication_ownership_bytes(),
        working: owner.derived().working_bytes().0,
        owned: owner.derived().owned_bytes().0,
    };
    Ok(Observation {
        result,
        prefix,
        reads: reads.map(|n| n.get()),
        attempts: attempts.into_inner(),
        work,
        counts: (counts.0.0, counts.1.0, counts.2.0),
        derived_builds: faults.derived.builds(),
        instructions,
        affected,
        held,
    })
}
pub(crate) fn real_port() -> Arc<dyn crate::outbound::operators::interface::OperatorsPort> {
    Arc::new(crate::application::file_dag::fixtures::operators())
}
pub(crate) fn made(
    op: &Operation,
    rows: [Rows<'_>; 2],
    symbols: &mut Symbols,
    source: &[u8],
) -> Result<(Table, Held), OperatingError> {
    let observed = invoke(
        op,
        rows,
        symbols,
        source,
        false,
        real_port(),
        Faults::default(),
    )?;
    Ok((observed.result?, observed.held))
}
