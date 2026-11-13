use super::program::{corpus, engine, prepared};
use super::subscriber::{Activation, Sample, Window};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::SourceLanguage;
use ryo_execution::inbound::use_cases::execution::interface::{
    Arena, CaptureContext, FileRunInput,
};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{FileId, Words, WorkerNumber};
use ryo_values::domain::stored_facts::types::Bucket;

/// Only declared relevant intents change; the installed engine and public entry, the one per-file route, stay identical.
pub(crate) fn measure(total: usize, demanded: &[usize]) -> (Sample, Window) {
    let engine = engine();
    let prepared = prepared(&engine, total);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
    let mut intents = vec![FileRuleIntent::Undemanded; total];
    for &index in demanded {
        intents[index] = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        };
    }
    // No relation is held: the activation reads no provider, and a demanded rule's read of one is refused, which stops the file.
    let mut facts = TableFacts::new(&[]);
    let elements = Elements::default();
    let observed = Activation::default();
    let dispatch = tracing::Dispatch::new(observed.clone());
    // The first actual run warms callsite registration and the allocator's
    // own metadata. The second sees exactly the same real request and intents.
    for _ in 0..2 {
        tracing::dispatcher::with_default(&dispatch, || {
            let mut symbols = Symbols::default();
            let first = vec![Bucket(0); intents.len()];
            let (mut arena, mut places, mut words) = (Vec::new(), Vec::new(), Words::default());
            let _rules = request.run_file(
                FileRunInput {
                    language: SourceLanguage::Recognized,
                    file: FileId(0),
                    intents: &intents,
                    source: ryo_values::domain::files::types::SourceView(b""),
                    sources: ryo_execution_runtime::outbound::facts::types::fact_sources::value::FactSources::unread(&elements),
                    wanted: &[],
                    active: &[],
                    said: &[],
                    recovery: ryo_execution::inbound::use_cases::metadata::interface::Recovery::Whole,
                    answering: ryo_execution::inbound::use_cases::execution::interface::Partiality::Physical,
                    semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                    kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
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
        });
    }
    request.finish();
    let state = observed.0.lock().unwrap();
    (
        state.sample,
        Window {
            starts: state.starts,
            ends: state.ends,
            active: state.active,
            misordered: state.misordered,
            provider_reads: facts.reads(),
            delivered: 0,
        },
    )
}
