//! The support of the `routes` cases and of the groups nested in it.
use super::super::inventory::fixtures::every_intent;
use super::super::witness::fixtures::{Probe, probe};
use crate::boundary_fixture::fixtures::{corpus, engine, prepared_rules};
use crate::file_intents::fixtures::named_request;
use ryo_execution::inbound::use_cases::execution::interface::CaptureContext;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileEnd;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::FileRunInput;
use ryo_execution::inbound::use_cases::execution::interface::FileStop;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RequestExecution;
use ryo_execution::inbound::use_cases::execution::interface::SourceLanguage;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::FileId;
use std::collections::BTreeSet;

/// The one-call file route: one intent per prepared rule, for every intent kind, over independent and named programs, over the
/// worker's facts; an incomplete or extra set is refused before any rule runs, each distinct answer once, by the number of
/// intents given.
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
        let mut worker = facts.open_worker().expect("a fake worker");
        request
            .run_file(
                FileRunInput {
                    language: SourceLanguage::Recognized,
                    file: FileId(0),
                    intents,
                    source: ryo_values::domain::files::types::SourceView(b"abc"),
                    sources: ryo_execution_runtime::outbound::facts::types::fact_sources::value::FactSources::unread(&elements),
                    wanted: &[],
                    active: &[],
                    said: &[],
                    recovery: ryo_execution::inbound::use_cases::metadata::interface::Recovery::Whole,
                    answering: ryo_execution::inbound::use_cases::execution::interface::Partiality::Physical,
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
                worker.as_mut(),
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
