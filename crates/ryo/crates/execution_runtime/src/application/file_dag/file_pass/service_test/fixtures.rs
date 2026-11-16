//! The support of the file pass's cases: a request of authored plans prepared over the engine's own operators, which a case
//! watches through the operators port, and one file run through the request's per-file route over a source that logs what it
//! is asked for.

use crate::application::file_dag::program::file_session::types::KindSupport;
use crate::application::file_dag::program::file_session::types::{
    CaptureContext, EmittedRoot, FileEnd, FileRules, FileRunInput, Partiality, RuleAfter,
    SemanticSupport, SourceLanguage,
};
use crate::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
use crate::application::file_dag::program::request_execution::service::begin;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::application::file_dag::service::prepare;
use crate::outbound::facts::fixtures::fake::FactsFake;
use crate::outbound::facts::fixtures::sources::SourceFake;
use crate::outbound::facts::types::fact_sources::value::FactSources;
use crate::outbound::operators::interface::OperatorsPort;
use ryo_execution_domain::domain::coverage::types::{Recovery, RuleIndex};
use ryo_execution_domain::domain::executable_program::service::{consumers, rule as rule_nodes};
use ryo_execution_domain::domain::finish::types::Arena;
use ryo_execution_domain::domain::prepared_request::service::assemble;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_execution_domain::domain::rule_intent::types::{
    CaptureIntent, DiagnosticDelivery, FileRuleIntent, ObservationDemand,
};
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::fixtures::intern;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_relations::domain::relations::types::{FileId, Offset, Relation, Words, WorkerNumber};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds, SourceView,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, RepeatedRows, StepId, StepScope};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId};
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use ryo_values::domain::stored_facts::types::{Bucket, Digest, ReadInPart};
use std::fmt::Write as _;
use std::sync::Arc;

/// The bytes of the run's file.
const SOURCE: &[u8] = b"abc";

/// A rule that runs on the file, its diagnostic collected and every frontier it hands over captured.
pub(super) const RUN: FileRuleIntent = FileRuleIntent::Run {
    diagnostic: DiagnosticDelivery::Collected,
    observations: ObservationDemand::NotRequired,
    capture: CaptureIntent::DiagnosticAndObservations,
};

/// A step in each file that selects every row of `relation`.
pub(super) fn every(relation: Relation) -> PlanStep {
    PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: Input::Relation(relation),
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
    }
}

/// A step in each file that keeps the first column of step `from`, its repeated rows kept.
pub(super) fn first_column(from: u32) -> PlanStep {
    PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Project {
            from: Input::Step(StepId(from)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
    }
}

/// A plan of `steps` that emits step `emit` and hands over `frontier`.
pub(super) fn plan(steps: Vec<PlanStep>, emit: u32, frontier: Vec<Input>) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        frontier,
        emit: Emit {
            step: StepId(emit),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        steps,
    }
}

/// The engine's own operators.
pub(super) fn engine() -> Arc<dyn OperatorsPort> {
    Arc::new(crate::outbound::operators::factory::use_case(
        ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl::new(),
        ryo_operators::inbound::use_cases::run::use_case::RunUseCaseImpl::new(Arc::new(
            ryo_operators::outbound::matching::factory::use_case(),
        )),
    ))
}

/// A rule of the request, by its place.
fn spec(index: usize) -> RuleSpec {
    RuleSpec {
        id: RuleId(format!("file-pass-{index}").into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(format!("file-pass-{index}").into()),
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

/// The run's one file, file 0.
fn corpus() -> Vec<CorpusInput> {
    vec![CorpusInput::Available(CorpusEntry {
        executable: ExecutablePermission::Unknown,
        path: FilePath("input.py".into()),
        size: FileSize(3),
        modified: Nanoseconds(0),
    })]
}

/// The request of `plans`, one rule each with no binding, prepared over `operators`.
pub(super) fn request(plans: Vec<Plan>, operators: Arc<dyn OperatorsPort>) -> RequestExecution {
    let rules: Vec<RuleSpec> = (0..plans.len()).map(spec).collect();
    let count = plans.len();
    let compiled = assemble(plans, vec![vec![]; count], &rules).expect("the rules assemble");
    let prepared = prepare(
        compiled,
        &rules,
        &vec![Digest([0; 16]); count],
        &[],
        ObservationIdentity::Independent,
        operators,
    )
    .expect("the rules prepare");
    begin(&prepared, &corpus(), None)
}

/// The node of step `step` of `rule` in `request`'s program.
pub(super) fn node_of(request: &RequestExecution, rule: usize, step: usize) -> PlanNodeId {
    rule_nodes(&request.pass.program, RuleIndex(rule))
        .ok()
        .and_then(|nodes| nodes.steps.get(step).cloned())
        .and_then(Result::ok)
        .expect("the step has a node")
}

/// Every node of `request`'s program, in node order.
pub(super) fn every_node(request: &RequestExecution) -> Vec<PlanNodeId> {
    (0..)
        .map(PlanNodeId)
        .take_while(|node| consumers(&request.pass.program, *node).is_ok())
        .collect()
}

/// What one file's run was asked and came to.
#[derive(Debug)]
pub(super) struct Ran {
    /// What the file's source was asked for, one entry per read.
    pub(super) asked: Vec<Vec<Relation>>,
    /// The file's rules and end, as text.
    pub(super) rules: String,
}

/// File 0 run through `request` with `intents`, over a source that serves `function`, `comment` and `semantic_child` and says
/// `said` is read in part, the file's partiality the physical one.
pub(super) fn ran(
    request: &RequestExecution,
    intents: &[FileRuleIntent],
    said: &[ReadInPart],
) -> Ran {
    let mut source = SourceFake::serving(&[
        Relation::Function,
        Relation::Comment,
        Relation::SemanticChild,
    ]);
    source.said = said.to_vec();
    let facts = FactsFake::default();
    let mut worker = facts.open_worker().expect("a fake worker");
    // The source writes Symbol(0) and Symbol(1) into its rows: they are the file's own names, `a` and `b` of its bytes, so a
    // captured row's names leave the file as any file's do.
    let mut symbols = Symbols::default();
    let named = [
        intern(&mut symbols, ByteSlice(SOURCE), Offset(0), Offset(1)),
        intern(&mut symbols, ByteSlice(SOURCE), Offset(1), Offset(2)),
    ];
    assert_eq!(
        named,
        [Symbol(0), Symbol(1)],
        "the names the source writes are the file's own"
    );
    let (mut arena, mut places, mut words) = (Vec::new(), Vec::new(), Words::default());
    let first = vec![Bucket(0); intents.len()];
    let rules = request.run_file(
        FileRunInput {
            language: SourceLanguage::Recognized,
            file: FileId(0),
            intents,
            source: SourceView(SOURCE),
            sources: FactSources::read(&mut source, &mut []).expect("one source"),
            active: &[],
            said: &[],
            recovery: Recovery::Whole,
            answering: Partiality::Physical,
            semantic: SemanticSupport::Absent,
            kinds: KindSupport::Absent,
            first: &first,
        },
        &mut symbols,
        CaptureContext {
            worker: WorkerNumber(0),
            arena: Arena(&mut arena),
            places: &mut places,
            words: &mut words,
        },
        worker.as_mut(),
    );
    Ran {
        asked: source.reads,
        rules: described(&rules),
    }
}

/// A file's rules and end, as text: each rule's emitted root and what came after it, then how the file ended and what it read
/// in part.
fn described(rules: &FileRules) -> String {
    let mut text = String::new();
    for each in &rules.rules {
        let emit = match &each.emit {
            EmittedRoot::Delivered(outcome) => outcome_text(outcome),
            EmittedRoot::Withheld => "withheld".to_owned(),
        };
        let after = match &each.after {
            RuleAfter::Ran { captures } => format!(
                "ran {}",
                captures
                    .iter()
                    .map(|capture| if capture.outcome.is_ok() {
                        "captured"
                    } else {
                        "refused"
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            RuleAfter::Unreached => "unreached".to_owned(),
            RuleAfter::Stopped(stop) => format!("stopped {stop:?}"),
        };
        let _ = write!(text, "rule {}: {emit}; {after}. ", each.rule.0);
    }
    let end = match &rules.end {
        FileEnd::Finished(_) => "finished".to_owned(),
        FileEnd::Unfinished => "unfinished".to_owned(),
        FileEnd::Refused(stop) => format!("refused {stop:?}"),
    };
    let _ = write!(text, "{end}; partial {:?}", rules.partials);
    text
}

/// A rule's emitted root, as text.
fn outcome_text(outcome: &RootOutcome) -> String {
    match outcome {
        RootOutcome::Complete(table) => format!("complete {:?}", table.cells),
        RootOutcome::Partial { table, affected } => {
            format!("partial {:?} affecting {affected:?}", table.cells)
        }
        RootOutcome::Skipped(_) => "skipped".to_owned(),
        RootOutcome::Unknown(error) => format!("unknown {error:?}"),
        RootOutcome::Failed(error) => format!("failed {error:?}"),
        RootOutcome::NoQuery => "no query".to_owned(),
        RootOutcome::Deferred { phase } => format!("deferred {phase:?}"),
    }
}
