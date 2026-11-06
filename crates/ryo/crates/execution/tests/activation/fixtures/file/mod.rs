use super::program::{corpus, engine, prepared};
use super::subscriber::{Activation, Sample};
use ryo_execution::inbound::use_cases::execution::{
    interface::{ExecutionRequest, ExecutionUseCase, FileExecution},
    types::*,
};
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::FileId;
use std::cell::Cell;

/// Only declared relevant intents change; the installed engine and public entry stay identical.
pub(crate) fn measure(total: usize, demanded: &[usize]) -> Sample {
    let engine = engine();
    let prepared = prepared(&engine, total);
    let corpus = corpus();
    let request = engine.begin(&prepared, &corpus);
    let mut intents = vec![FileRuleIntent::Undemanded; total];
    for &index in demanded {
        intents[index] = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        };
    }
    let reads = Cell::new(0);
    let read = |_| {
        reads.set(reads.get() + 1);
        Err(OperatingError::NotRead)
    };
    let elements = Elements::default();
    let observed = Activation::default();
    let dispatch = tracing::Dispatch::new(observed.clone());
    // The first actual opening warms callsite registration and the allocator's
    // own metadata. The second sees exactly the same real request and intents.
    for _ in 0..2 {
        tracing::dispatcher::with_default(&dispatch, || {
            let mut symbols = Symbols::default();
            let file = request
                .open_file(
                    FileInput {
                        language: SourceLanguage::Recognized,
                        resident_stability: facts::ResidentStability::Stable,
                        file: FileId(0),
                        evaluation: &[],
                        publication: &[],
                        intents: &intents,
                        source: b"",
                        elements: &elements,
                        read: &read,
                        semantic: None,
                        kinds: None,
                    },
                    &mut symbols,
                )
                .unwrap();
            assert_eq!(file.readiness_counts().evaluations.0, 0);
            file.finish().unwrap();
        });
    }
    assert_eq!(reads.get(), 0, "activation must not evaluate a provider");
    request.finish();
    let state = observed.0.lock().unwrap();
    assert_eq!((state.starts, state.ends, state.active), (2, 2, false));
    state.sample
}
