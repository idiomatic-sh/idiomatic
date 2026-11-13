//! The support of the `routes` cases and of the groups nested in it.
use super::super::inventory::fixtures::every_intent;
use super::super::witness::fixtures::{Probe, probe};
use crate::boundary_fixture::fixtures::{corpus, engine, prepared_rules};
use crate::file_intents::fixtures::named_request;
use ryo_execution::inbound::use_cases::execution::interface::CaptureContext;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileEnd;
use ryo_execution::inbound::use_cases::execution::interface::FileInput;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::FileRunInput;
use ryo_execution::inbound::use_cases::execution::interface::FileStop;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RequestExecution;
use ryo_execution::inbound::use_cases::execution::interface::SourceLanguage;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::FileId;
use std::collections::BTreeSet;

/// File route: one intent per prepared rule, for every intent kind, over independent and named programs; each distinct answer
/// once, by the number of intents given.
pub(in super::super) fn file_route() -> Vec<Probe> {
    let engine = engine();
    let programs = [
        prepared_rules(&engine, 2),
        engine.prepare(named_request()).expect("named request"),
    ];
    let corpus = corpus();
    let elements = Elements::default();
    let read = |_| Err(OperatingError::NotRead);
    let open = |request: &RequestExecution, intents: &[FileRuleIntent]| {
        let mut symbols = Symbols::default();
        request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents,
                    source: ryo_values::domain::files::types::SourceView(b"abc"),
                    elements: &elements,
                    read: &read,
                    semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                    kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
                },
                &mut symbols,
            )
            .map(|file| {
                let _ = file.finish();
            })
    };
    let mut answers = BTreeSet::new();
    for program in &programs {
        for intent in every_intent() {
            // The incomplete and extra sets over one request, the complete set over a fresh one.
            for lengths in [&[0, 1, 3][..], &[2]] {
                let request = ExecutionUseCaseImpl::new().execution(program, &corpus, None.into());
                for &length in lengths {
                    answers.insert(probe(
                        format!("{length} intents of one kind for 2 rules"),
                        open(&request, &vec![intent; length]),
                    ));
                }
                request.finish();
            }
        }
    }
    answers.into_iter().collect()
}

/// The one-call file route: the same declared intent set as `file_route`, over the worker's facts; an incomplete or extra set
/// is refused before any rule runs.
pub(in super::super) fn run_file_route() -> Vec<Probe> {
    let engine = engine();
    let programs = [
        prepared_rules(&engine, 2),
        engine.prepare(named_request()).expect("named request"),
    ];
    let corpus = corpus();
    let elements = Elements::default();
    let facts = ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake::default();
    let run = |request: &RequestExecution, intents: &[FileRuleIntent]| {
        let mut symbols = Symbols::default();
        let mut arena = Vec::new();
        let mut places = Vec::new();
        let mut words = ryo_relations::domain::relations::types::Words::default();
        let worker = facts.open_worker().expect("a fake worker");
        request
            .run_file(
                FileRunInput {
                    language: SourceLanguage::Recognized,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents,
                    source: ryo_values::domain::files::types::SourceView(b"abc"),
                    elements: &elements,
                    semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                    kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
                    first: &[],
                },
                &mut symbols,
                CaptureContext {
                    worker: ryo_relations::domain::relations::types::WorkerNumber(0),
                    arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                    places: &mut places,
                    words: &mut words,
                },
                worker.as_ref(),
            )
            .end
    };
    let mut answers = BTreeSet::new();
    for program in &programs {
        for intent in every_intent() {
            let request = ExecutionUseCaseImpl::new().execution(program, &corpus, None.into());
            for length in [0, 1, 3] {
                answers.insert(probe(
                    format!("{length} intents of one kind for 2 rules"),
                    refusal(run(&request, &vec![intent; length])),
                ));
            }
            request.finish();
        }
    }
    answers.into_iter().collect()
}

/// The stop a file run ended with; `None` when it finished or was left unfinished.
fn refusal(end: FileEnd) -> Option<FileStop> {
    if let FileEnd::Refused(stop) = end {
        Some(stop)
    } else {
        None
    }
}
