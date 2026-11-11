//! The native trees of the files and the run through the owner that the walk proofs compare plans by.
use crate::boundary_fixture::fixtures::{authored_request, demand, engine};
use crate::project_phase::source_facts::fixtures::ProjectFacts;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_domain::domain::frontier_capture::types::CapturedFrontier;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::errors::FactsError;
use ryo_execution_runtime::outbound::facts::interface::FactsFinish;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution_runtime::outbound::facts::types::{BucketSource, KeptBucket};
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::{
    builders::fixtures as builders, completion::fixtures as completion, elements::types::Elements,
    symbols::types::ByteSlice,
};
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_relations::domain::relations::types::{KindBits, Offset};
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
};
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::{ArenaBytes, KeptWords, PackedWords};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, StepId, StepScope},
    plans::types::PlanStep,
};
use std::sync::Arc;

/// The text of every file: two tokens under one root, at ordinals 0 and 1.
const SOURCE: &[u8] = b"x\nx\n";

/// The syntax tree of `file`: a root node of kind `root` holding two tokens of kind `token`.
fn native_tree(file: FileId) -> (Elements, Symbols) {
    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    {
        let mut builder = builders::table_builder(&mut elements, &mut symbols, ByteSlice(SOURCE));
        builders::enter_node(&mut builder, KindBits(0), Offset(0));
        builders::token(&mut builder, KindBits(1), Offset(0), Offset(1));
        builders::token(&mut builder, KindBits(1), Offset(2), Offset(3));
        builders::leave_node(&mut builder, Offset(4));
    }
    let mut completion =
        completion::completion(&mut elements, &mut symbols, ByteSlice(SOURCE), file);
    completion::name_kind(&mut completion, KindBits(0), NameText("root"));
    completion::name_kind(&mut completion, KindBits(1), NameText("token"));
    (elements, symbols)
}

/// The frontiers the files captured, by bucket.
struct CapturedBuckets {
    captures: Vec<(FileId, CapturedFrontier)>,
}
impl FactsFinish for CapturedBuckets {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        let mut words = vec![];
        let mut sources = vec![];
        for (file, capture) in self
            .captures
            .iter()
            .filter(|(_, capture)| capture.bucket == bucket)
        {
            sources.push(BucketSource {
                file: *file,
                start: ryo_execution_runtime::outbound::facts::types::KeptWordsStart(words.len()),
                length: ryo_execution_runtime::outbound::facts::types::KeptWordsLength(
                    capture.words.0.len(),
                ),
                proof: capture.proof.clone(),
            });
            words.extend_from_slice(&capture.words.0);
        }
        Ok(KeptBucket {
            words: Arc::new(KeptWords(PackedWords(words))),
            sources,
        })
    }
}

/// Why a root gave no rows, in words.
fn unanswered(outcome: &RootOutcome) -> String {
    match outcome {
        RootOutcome::Failed(error) => format!("failed: {error}"),
        RootOutcome::Complete(_)
        | RootOutcome::Partial { .. }
        | RootOutcome::Skipped(_)
        | RootOutcome::Unknown(_)
        | RootOutcome::NoQuery
        | RootOutcome::Deferred { .. } => "no complete rows".to_owned(),
    }
}

/// The rows of a table, each written out, sorted: the proofs compare rows, not the order a run hands them over in.
fn written(table: &ryo_values::domain::rows::types::Table) -> Vec<String> {
    let width = table.kinds.len().max(1);
    let mut rows: Vec<String> = table
        .cells
        .chunks_exact(width)
        .map(|row| format!("{row:?}"))
        .collect();
    rows.sort_unstable();
    rows
}

/// The rows a plan of `steps` emitting `emitted` answers over `files` native trees, run through the owner as a run runs
/// it: each file's steps in the file, then the steps after the files over what every file captured. A file or the
/// project phase that does not complete is its outcome, never rows.
pub(super) fn rows(
    steps: Vec<PlanStep>,
    emitted: StepId,
    files: usize,
) -> Result<Vec<String>, String> {
    let crossing = steps.iter().any(|step| step.scope == StepScope::CrossFile);
    let engine = engine();
    let mut input = authored_request(1);
    let plan = &mut crate::boundary_fixture::fixtures::plans(&mut input)[0];
    plan.frontier =
        ryo_query_language::inbound::use_cases::compile::fixtures::rewrites::frontier(&steps);
    plan.steps = steps;
    plan.emit = Emit {
        step: emitted,
        subject: ColumnIndex(0),
        key: vec![],
        headings: vec![],
    };
    let program = engine.prepare(input).expect("prepare the plan");
    let corpus: Vec<CorpusInput> = (0..files)
        .map(|place| {
            CorpusInput::Available(CorpusEntry {
                executable: ExecutablePermission::Unknown,
                path: ryo_language::domain::files::types::FilePath(format!("file-{place}").into()),
                size: FileSize(u64::try_from(SOURCE.len()).expect("a source length fits in u64")),
                modified: Nanoseconds(0),
            })
        })
        .collect();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let mut arena = vec![];
    let mut places = vec![];
    let mut words = Words::default();
    let mut captures = vec![];
    let mut answered = vec![];
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: if crossing {
            CaptureIntent::DiagnosticAndObservations
        } else {
            CaptureIntent::Disabled
        },
    }];
    for place in 0..files {
        let file_id = FileId(u32::try_from(place).unwrap());
        let (elements, mut symbols) = native_tree(file_id);
        let read = |_| Err::<SourceRows<'_>, _>(OperatingError::NotRead);
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: ResidentStability::Stable,
                    file: file_id,
                    evaluation: &[],
                    publication: &[],
                    intents: &intents,
                    source: ryo_values::domain::files::types::SourceView(SOURCE),
                    elements: &elements,
                    read: &read,
                    semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
                },
                &mut symbols,
            )
            .expect("open the file");
        match file
            .rule(RuleIndex(0), demand(), &mut symbols)
            .expect("run the file")
            .emit
        {
            RootOutcome::Complete(table) => answered.extend(written(&table)),
            RootOutcome::Deferred {
                phase: StepScope::CrossFile,
            } => {
                for capture in file
                    .capture_frontiers(
                        RuleIndex(0),
                        Bucket(0),
                        &mut symbols,
                        CaptureContext {
                            worker: WorkerNumber(0),
                            arena: ryo_execution::inbound::use_cases::execution::interface::Arena(
                                &mut arena,
                            ),
                            places: &mut places,
                            words: &mut words,
                        },
                    )
                    .expect("capture the file's frontiers")
                {
                    captures.push((file_id, capture.outcome.expect("a complete capture")));
                }
            }
            other @ (RootOutcome::Partial { .. }
            | RootOutcome::Skipped(_)
            | RootOutcome::Unknown(_)
            | RootOutcome::Failed(_)
            | RootOutcome::NoQuery
            | RootOutcome::Deferred { .. }) => {
                return Err(format!("file {place}: {}", unanswered(&other)));
            }
        }
        let _finished = file.finish().expect("release the file");
    }
    if crossing {
        let facts = CapturedBuckets { captures };
        let project = ProjectFacts::empty(Relation::ModuleIdentity);
        let covered: Vec<Covered> = (0..files)
            .map(|place| Covered {
                file: FileId(u32::try_from(place).unwrap()),
                rules: vec![RuleIndex(0)],
                partial: vec![],
            })
            .collect();
        let phase = request.project_phase().expect("the project phase");
        let project_symbols = Symbols::default();
        let mut symbols = Symbols::default();
        let partials = Partials::default();
        let arenas = [ArenaBytes(arena)];
        let outcome = crate::boundary_fixture::fixtures::project_root(
            &phase,
            PhaseRules {
                rules: &[RuleIndex(0)],
                first: &[Bucket(0)],
                corpus: &corpus,
                covered: &covered,
                skipped: &[],
                inapplicable: &[ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default()],
                refused: &[ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default()],
                facts: &facts,
                project: &project,
                project_symbols: &project_symbols,
                arenas: &arenas,
            },
            &partials,
            &partials,
            &mut symbols,
        )
        .expect("run the steps after the files")
        .emit;
        match outcome {
            RootOutcome::Complete(table) => answered.extend(written(&table)),
            other @ (RootOutcome::Partial { .. }
            | RootOutcome::Skipped(_)
            | RootOutcome::Unknown(_)
            | RootOutcome::Failed(_)
            | RootOutcome::NoQuery
            | RootOutcome::Deferred { .. }) => {
                return Err(format!("after the files: {}", unanswered(&other)));
            }
        }
        phase.finish().expect("finish the project phase");
    }
    request.finish();
    answered.sort_unstable();
    Ok(answered)
}
