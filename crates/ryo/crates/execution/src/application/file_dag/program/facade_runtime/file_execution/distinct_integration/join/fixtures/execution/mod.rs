//! The real file owner and its semantic port, invoked once per join observation.
use super::super::super::*;
use super::provider::{Provider, provider};
use crate::application::file_dag::derived::owner::fixtures::resources::RecordedDerived;
use crate::domain::operating::types::ExecutionObservation;
use crate::domain::table_model::service::materialize;
use ryo_values::domain::compact_table::service::admit_compact;
use ryo_values::domain::stored_facts::types::KeptWords;
use ryo_values::domain::table_words::types::DecodeMode;
use std::cell::Cell as Counter;
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
    pub(crate) derived_builds: Vec<ryo_derived::domain::contract::types::ViewRequest>,
    pub(crate) fed: Vec<crate::domain::operator::types::RowsFed>,
    pub(crate) instructions: usize,
    pub(crate) native_prepared: bool,
    pub(crate) column_prepared: bool,
    pub(crate) frontiers: Vec<Table>,
    pub(crate) frontier_errors: Vec<OperatingError>,
    pub(crate) affected: Vec<Option<FileId>>,
}
/// Fixture behavior belongs to real external source/sink dependencies only.
#[derive(Default)]
pub(crate) struct Faults {
    pub(crate) derived: Arc<RecordedDerived>,
    pub(crate) left: Option<OperatingError>,
    pub(crate) right: Option<OperatingError>,
    pub(crate) sink_after: Option<usize>,
    pub(crate) optional_limit: Option<usize>,
    pub(crate) partials: Vec<crate::domain::coverage::types::Partial>,
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
    let mut request = plan(Scenario::ProjectRetry);
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
    let derived: Arc<dyn crate::outbound::derived::interface::DerivedPort> = faults.derived.clone();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            plans: vec![request],
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
    let instructions = program.regions()[region.0].instructions.len();
    let native_prepared = crate::domain::executable_program::service::native_positions(
        &program.regions()[region.0].route,
    )
    .is_some();
    let column_prepared =
        crate::domain::executable_program::service::column(&program.regions()[region.0].route)
            .is_some();
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
        crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
        symbols,
        None,
        None,
    )?;
    if let Some(limit) = faults.optional_limit {
        file.owner.derived.set_optional_limit(
            crate::application::file_dag::derived::owner::index::types::ByteLimit(limit),
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
    let mut frontier_errors = vec![];
    let frontiers = if has_frontier {
        let mut arena = vec![];
        let mut places = vec![];
        let mut words = ryo_relations::domain::relations::types::Words::default();
        file.capture_frontiers(
            RuleIndex(0),
            ryo_values::domain::stored_facts::types::Bucket(0),
            &read,
            symbols,
            crate::application::file_dag::program::protocol::types::CaptureScope {
                file: FileId(0),
                rule_digest: Digest([0; 16]),
                worker: ryo_relations::domain::relations::types::WorkerNumber(0),
                retained: crate::domain::finish::types::Retained {
                    arena: crate::domain::finish::types::Arena(&mut arena),
                    places: &mut places,
                },
                words: &mut words,
            },
        )?
        .into_iter()
        .filter_map(|capture| {
            let capture = match capture.outcome {
                Ok(capture) => capture,
                Err(error) => {
                    frontier_errors.push(error);
                    return None;
                }
            };
            let ryo_values::domain::table_words::types::Admitted { table: compact, .. } =
                admit_compact(
                    ryo_values::domain::table_words::types::CompactBacking(Arc::new(KeptWords(
                        capture.words,
                    ))),
                    &mut vec![],
                    DecodeMode::Complete,
                )
                .expect("actual captured words decode");
            let mut table = Table::default();
            materialize(&Rows::Compact(&compact), &mut table, &mut vec![]);
            Some(table)
        })
        .collect()
    } else {
        vec![]
    };
    let counts = file.owner.derived.counts();
    let fed = file.counts(RuleIndex(0))?.fed.to_vec();
    let owner = Rc::clone(&file.owner);
    file.finish()?;
    assert_eq!(
        owner.derived.publication_ownership_bytes(),
        (0, 0, 0),
        "normal finish releases the actual shared physical ledger, including finite raw orders"
    );
    assert_eq!(
        owner.derived.working_bytes(),
        0,
        "actual invocation resources released by normal owner shutdown"
    );
    assert_eq!(
        owner.derived.owned_bytes(),
        0,
        "finished file releases owned index storage"
    );
    Ok(Observation {
        result,
        prefix,
        reads: reads.map(|n| n.get()),
        attempts: attempts.into_inner(),
        work,
        counts: (counts.0.0, counts.1.0, counts.2.0),
        derived_builds: faults.derived.builds(),
        fed,
        instructions,
        native_prepared,
        column_prepared,
        frontiers,
        frontier_errors,
        affected,
    })
}
pub(crate) fn real_port() -> Arc<dyn crate::outbound::operators::interface::OperatorsPort> {
    Arc::new(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new())
}
pub(crate) fn made(
    op: &Operation,
    rows: [Rows<'_>; 2],
    symbols: &mut Symbols,
    source: &[u8],
) -> Result<Table, OperatingError> {
    invoke(
        op,
        rows,
        symbols,
        source,
        false,
        real_port(),
        Faults::default(),
    )?
    .result
}
