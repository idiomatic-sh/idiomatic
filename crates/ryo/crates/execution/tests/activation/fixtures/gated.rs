//! One file run through the public seam over a program, by the operators as the engine composes them or with every node
//! active (`all_active`): what the file's rules came to, and what a warm run allocated.
use super::subscriber::RunCost;
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::{
    Arena, CaptureContext, CaptureIntent, DiagnosticDelivery, ExecutionUseCase, FileRuleIntent,
    FileRules, FileRunInput, KindSupport, ObservationDemand, Partiality,
    ProgramRequestExecutionPort, SemanticSupport, SourceLanguage,
};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_language::domain::builders::fixtures::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::fixtures::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::kind_index::fixtures::index_kinds;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{FileId, KindBits, Offset, Words, WorkerNumber};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds, SourceView,
};
use ryo_values::domain::stored_facts::types::Bucket;
use std::sync::Arc;

/// The bytes of the file.
const SOURCE: &[u8] = b"abc";

/// Which operators a run goes through.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Operators {
    /// The operators as the engine composes them: a node the file's triggers show dormant does not run.
    Gated,
    /// The same operators, every node active.
    AllActive,
}

/// The tree of the file: a `file` node over the tokens `word`, `other`, `word`, its kinds indexed.
fn tree() -> (Elements, Symbols) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(SOURCE));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    token(&mut builder, KindBits(3), Offset(1), Offset(2));
    token(&mut builder, KindBits(2), Offset(2), Offset(3));
    leave_node(&mut builder, Offset(3));
    {
        let mut completed = completion(&mut elements, &mut symbols, ByteSlice(SOURCE), FileId(0));
        for (kind, name) in [(1, "file"), (2, "word"), (3, "other")] {
            name_kind(&mut completed, KindBits(kind), NameText(name));
        }
    }
    index_kinds(&mut elements);
    (elements, symbols)
}

/// The file run `times` times through the program of `request` by `operators`, every rule's diagnostic collected and its named
/// outputs demanded, over facts that hold no relation: what its rules came to and what its last run allocated.
fn ran(request: PrepareRequest, operators: Operators, times: usize) -> (FileRules, RunCost) {
    let engine = match operators {
        Operators::Gated => PrepareUseCaseImpl::new(Arc::new(
            ryo_execution::inbound::use_cases::execution::fixtures::operators(),
        )),
        Operators::AllActive => PrepareUseCaseImpl::new(Arc::new(
            ryo_execution_runtime::outbound::operators::fixtures::all_active::AllActiveOperators {
                inner: Arc::new(
                    ryo_execution::inbound::use_cases::execution::fixtures::operators(),
                ),
            },
        )),
    };
    let count = request.rules.len();
    let program = engine.prepare(request).expect("a valid authored request");
    let corpus = [CorpusInput::Available(CorpusEntry {
        executable: ExecutablePermission::Unknown,
        path: FilePath("gated.fixture".into()),
        size: FileSize(3),
        modified: Nanoseconds(0),
    })];
    let execution = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let intents = vec![
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::Required,
            capture: CaptureIntent::DiagnosticAndObservations,
        };
        count
    ];
    let first = vec![Bucket(0); count];
    let mut answered = None;
    for _ in 0..times {
        let (elements, mut symbols) = tree();
        let mut facts = TableFacts::new(&[]);
        let (mut arena, mut places, mut words) = (Vec::new(), Vec::new(), Words::default());
        let before = dhat::HeapStats::get();
        let rules = execution.run_file(
            FileRunInput {
                language: SourceLanguage::Recognized,
                file: FileId(0),
                intents: &intents,
                source: SourceView(SOURCE),
                sources: ryo_execution_runtime::outbound::facts::types::fact_sources::value::FactSources::unread(&elements, &mut [])
                    .expect("a file with no further source"),
                active: &[],
                said: &[],
                recovery: ryo_execution::inbound::use_cases::metadata::interface::Recovery::Whole,
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
            &mut facts,
        );
        let after = dhat::HeapStats::get();
        answered = Some((
            rules,
            RunCost {
                blocks: after.total_blocks - before.total_blocks,
                bytes: after.total_bytes - before.total_bytes,
            },
        ));
    }
    execution.finish();
    answered.expect("the file ran at least once")
}

/// What the file's rules came to through the program of `request` by `operators`.
pub(crate) fn file_rules(request: PrepareRequest, operators: Operators) -> FileRules {
    ran(request, operators, 1).0
}

/// What the second of two runs of the file allocated through the program of `request` by the composed operators: the first
/// warms the worker's buffers and the allocator's own metadata.
pub(crate) fn warm_cost(request: PrepareRequest) -> RunCost {
    ran(request, Operators::Gated, 2).1
}
