//! The support of the `handovers` cases: two rules, each handed its file step and one project relation after the files,
//! prepared as the engine prepares them; a store whose every read is logged; and one batch of the phase run over them.
use crate::application::file_dag::derived::service::prepare;
use crate::application::file_dag::frontiers::types::PhaseScoping;
use crate::application::file_dag::program::project_phase::types::{PhasePartials, PhaseStop};
use crate::application::file_dag::program::request_execution::service::begin;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::application::file_dag::program::rule_outcome::types::{ProjectBatch, RootOutcome};
use crate::application::file_dag::project_pass::service::{project_reads, run_batch, uses_of};
use crate::application::file_dag::project_pass::types::{
    PhaseBatch, PhaseOccurrence, PhaseState, RuleUses,
};
use crate::application::file_dag::project_source::service::{opened, table_of};
use crate::application::file_dag::project_source::types::ProjectWritten;
use crate::application::file_dag::publications::types::{BoundTables, PublicationSlot};
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::types::KeptBucket;
use ryo_execution_domain::domain::coverage::types::{
    InapplicableFiles, Partials, RefusedFiles, RuleIndex,
};
use ryo_execution_domain::domain::executable_program::service::rule as rule_nodes;
use ryo_execution_domain::domain::prepared_request::service::assemble;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::physical_plan::types::NodeRows;
use ryo_relations::domain::relations::fixtures::kinds;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId, StepScope};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId};
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use ryo_values::domain::stored_facts::types::{Bucket, Digest, KeptWords, PackedWords};
use std::sync::{Arc, Mutex, PoisonError};

/// The project relation both rules read after the files.
pub(super) const PROJECT: Relation = Relation::ModuleIdentity;

/// The first bucket of each rule's frontier, by rule: rule 0's file frontier is bucket 0, rule 1's bucket 16.
pub(super) const FIRST: [Bucket; 2] = [Bucket(0), Bucket(16)];

/// What the phase did, in order: each store read, by bucket, and each rule the guard admitted.
#[derive(Debug, Default)]
pub(super) struct Log(pub(super) Mutex<Vec<String>>);

impl Log {
    /// Adds what happened.
    pub(super) fn push(&self, event: String) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
    /// Everything that happened, in order.
    pub(super) fn events(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// A store no file kept any word in, every read logged; the read of `refused` refused.
pub(super) struct LoggedKept<'l> {
    /// Where each read is logged.
    pub(super) log: &'l Log,
    /// The bucket whose read the store refuses, if any.
    pub(super) refused: Option<Bucket>,
}

impl FactsFinish for LoggedKept<'_> {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        self.log.push(format!("kept {}", bucket.0));
        if self.refused == Some(bucket) {
            return Err(FactsError::Store {
                reason: "the kept frontier is unreadable".into(),
            });
        }
        Ok(KeptBucket {
            words: Arc::new(KeptWords(PackedWords(Vec::new()))),
            sources: Vec::new(),
        })
    }
}

/// A rule of the request, by its place.
fn rule(index: usize) -> RuleSpec {
    RuleSpec {
        id: RuleId(format!("handover-{index}").into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(format!("handover-{index}").into()),
                text: "".into(),
            },
            modules: Default::default(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters("".into()),
        include: vec![],
        exclude: vec![],
    }
}

/// A plan whose file step selects Function's file and whose emit, after the files, selects the project relation's first column:
/// its frontier is the file step and the project relation.
fn plan() -> Plan {
    let file = kinds(Relation::Function)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .and_then(|at| u32::try_from(at).ok())
        .unwrap_or(0);
    Plan {
        observation_inputs: vec![],
        parameters: vec![],
        steps: vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Relation(Relation::Function),
                    comparisons: vec![],
                    kept: vec![ColumnIndex(file)],
                },
            },
            PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Select {
                    from: Input::Relation(PROJECT),
                    comparisons: vec![],
                    kept: vec![ColumnIndex(0)],
                },
            },
        ],
        comparison_origins: vec![],
        observations: vec![],
        emit: Emit {
            step: StepId(1),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![Input::Step(StepId(0)), Input::Relation(PROJECT)],
    }
}

/// The request of the two rules over no file, prepared over the engine's own operators.
pub(super) fn request() -> RequestExecution {
    let rules = [rule(0), rule(1)];
    let compiled =
        assemble(vec![plan(), plan()], vec![vec![]; 2], &rules).expect("the two rules assemble");
    let operators: Arc<dyn crate::outbound::operators::interface::OperatorsPort> =
        Arc::new(crate::outbound::operators::factory::use_case(
            ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl::new(),
            ryo_operators::inbound::use_cases::run::use_case::RunUseCaseImpl::new(Arc::new(
                ryo_operators::outbound::matching::factory::use_case(),
            )),
        ));
    let prepared = prepare(
        compiled,
        &rules,
        &[Digest([0; 16]); 2],
        &[],
        ObservationIdentity::Independent,
        operators,
    )
    .expect("the two rules prepare");
    begin(&prepared, &[], None)
}

/// What one batch of the phase left.
#[derive(Debug)]
pub(super) struct Ran {
    /// Each store read and each rule the guard admitted, in order.
    pub(super) log: Vec<String>,
    /// Each occurrence's emit, or why the phase stopped it, as text.
    pub(super) outcomes: Vec<String>,
    /// How many holders the phase's table of the project relation had when the batch ended, the phase's own included.
    pub(super) holders: usize,
    /// Whether each rule's emit still held rows when the batch ended, by rule.
    pub(super) emits: Vec<&'static str>,
}

/// Runs `occurrences`, the phase's declared occurrences in order, as one batch, the store refusing `refused`; `uses` stands in
/// for the count the phase's callers set from them, when given.
pub(super) fn ran(
    occurrences: &[RuleIndex],
    uses: Option<RuleUses>,
    refused: Option<Bucket>,
) -> Ran {
    let request = request();
    let log = Log::default();
    let facts = LoggedKept { log: &log, refused };
    let project_facts = ProjectFacts::new(PROJECT);
    let project_symbols = Symbols::default();
    let project = opened(
        ProjectWritten {
            rows: &project_facts,
            symbols: &project_symbols,
        },
        project_reads(&request.pass, occurrences),
    );
    let inapplicable = [InapplicableFiles::default(), InapplicableFiles::default()];
    let refusals = [RefusedFiles::default(), RefusedFiles::default()];
    let stage = Arc::new(());
    let batch = PhaseBatch {
        program: &request.program,
        pass: &request.pass,
        execution: &request,
        scoping: PhaseScoping {
            corpus: &[],
            covered: &[],
            skipped: &[],
            inapplicable: &inapplicable,
            refused: &refusals,
        },
        first: &FIRST,
        facts: &facts,
        project: &project,
        arenas: &[],
        observer: None,
        stage: &stage,
    };
    let declared = occurrences
        .iter()
        .map(|rule| PhaseOccurrence {
            rule: *rule,
            slot: PublicationSlot::Unused,
            ready: None,
            bound: BoundTables::default(),
        })
        .collect::<Vec<_>>();
    let mut state = PhaseState {
        uses: uses.unwrap_or_else(|| uses_of(occurrences.iter().copied())),
        ..PhaseState::default()
    };
    let mut symbols = Symbols::default();
    let batched = run_batch(
        &batch,
        &mut state,
        &declared,
        &mut symbols,
        |_| Ok::<(), ()>(()),
        |_| {
            Ok(PhasePartials {
                evaluation: Partials::default(),
                publication: Partials::default(),
            })
        },
        |rule| {
            log.push(format!("guard {}", rule.0));
            Ok(())
        },
        |_, outcome, _, _| Ok(described(&outcome.emit)),
    );
    let outcomes = match batched {
        ProjectBatch::Complete(done)
        | ProjectBatch::Refused {
            completed: done, ..
        } => done
            .into_iter()
            .map(|each| match each {
                Ok(text) => text,
                Err(PhaseStop::Frontiers(error)) => format!("stopped frontiers {error}"),
                Err(stop) => format!("stopped {stop:?}"),
            })
            .collect(),
    };
    let holders = table_of(&project, PROJECT).map_or(0, Arc::strong_count);
    let emits = [RuleIndex(0), RuleIndex(1)]
        .iter()
        .map(|rule| {
            match rule_nodes(&request.pass.program, *rule).map(|nodes| nodes.emit.clone()) {
                Ok(Ok(node)) => match state.buffers.rows(node) {
                    NodeRows::Materialized(_) => "held",
                    NodeRows::Absent => "absent",
                },
                Ok(Err(_)) | Err(_) => "no node",
            }
        })
        .collect();
    Ran {
        log: log.events(),
        outcomes,
        holders,
        emits,
    }
}

/// An emit's answer, as text.
fn described(outcome: &RootOutcome) -> String {
    match outcome {
        RootOutcome::Complete(table) => format!("complete {:?}", table.count),
        RootOutcome::Partial { table, .. } => format!("partial {:?}", table.count),
        RootOutcome::Failed(error) => format!("failed {error:?}"),
        RootOutcome::Unknown(error) => format!("unknown {error:?}"),
        RootOutcome::Deferred { phase } => format!("deferred {phase:?}"),
        RootOutcome::Skipped(_) => "skipped".to_owned(),
        RootOutcome::NoQuery => "no query".to_owned(),
    }
}
