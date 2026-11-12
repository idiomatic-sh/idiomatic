use super::program::{corpus, engine, prepared};
use super::subscriber::{Activation, Sample, Window};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileInput;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::SourceLanguage;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::FileId;
use std::cell::Cell;

/// Only declared relevant intents change; the installed engine and public entry stay identical.
pub(crate) fn measure(total: usize, demanded: &[usize]) -> (Sample, Window) {
    let engine = engine();
    let prepared = prepared(&engine, total);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
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
                        resident_stability: ResidentStability::Stable,
                        file: FileId(0),
                        evaluation: &[],
                        publication: &[],
                        intents: &intents,
                        source: ryo_values::domain::files::types::SourceView(b""),
                        elements: &elements,
                        read: &read,
                        semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                        kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
                    },
                    &mut symbols,
                )
                .unwrap();
            file.finish().unwrap();
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
            provider_reads: reads.get(),
            delivered: 0,
        },
    )
}
