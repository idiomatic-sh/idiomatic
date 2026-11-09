//! Three rules over three files, answered as the recorded fixture.
use super::super::super::*;

/// Three rules over three files through the one-call file route answer exactly what `tests/fixtures/smoke/file_rules.txt`
/// recorded: each rule's emitted root and how its run ended, file by file. A change to the engine behind the seam that changes
/// any answer changes this text.
#[test]
fn three_rules_over_three_files_answer_the_recorded_file_rules() {
    use ryo_execution::inbound::use_cases::execution::interface::{
        CaptureContext, EmittedRoot, FileEnd, FileRunInput, RuleAfter,
    };
    use std::fmt::Write as _;
    let engine = engine();
    let mut input = authored_request(3);
    let plans = crate::boundary_fixture::plans(&mut input);
    plans[1].steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::GroupCount {
            from: Input::Step(StepId(0)),
            keys: vec![],
            distinct: None,
        },
    });
    plans[1].emit.step = StepId(1);
    plans[2].steps[0].operation = Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![ColumnIndex(2), ColumnIndex(0)],
    };
    let program = engine.prepare(input).expect("prepare three rules");
    let mut corpus = corpus();
    for name in ["second.py", "third.py"] {
        let mut more = corpus[0].clone();
        if let ryo_values::domain::files::types::CorpusInput::Available(entry) = &mut more {
            entry.path = ryo_language::domain::files::types::FilePath(name.into());
        }
        corpus.push(more);
    }
    let facts = ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake::default();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let mut answered = String::new();
    for (file, ids) in [(0, vec![1, 2]), (1, vec![]), (2, vec![7, 8, 9])] {
        let worker = super::super::fixtures::function_rows(&facts, FileId(file), &ids);
        let elements = ryo_language::domain::elements::types::Elements::default();
        let mut symbols = Symbols::default();
        let (mut arena, mut places, mut words) = (
            Vec::new(),
            Vec::new(),
            ryo_relations::domain::relations::types::Words::default(),
        );
        let run = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        };
        let rules = request.run_file(
            FileRunInput {
                language: SourceLanguage::Recognized,
                file: FileId(file),
                evaluation: &[],
                publication: &[],
                intents: &[run, run, run],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
                first: &[ryo_values::domain::stored_facts::types::Bucket(0); 3],
            },
            &mut symbols,
            CaptureContext {
                worker: ryo_relations::domain::relations::types::WorkerNumber(0),
                arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                places: &mut places,
                words: &mut words,
            },
            worker.as_ref(),
        );
        for ran in rules.rules {
            let emitted = match ran.emit {
                EmittedRoot::Delivered(RootOutcome::Complete(table)) => format!("complete {table:?}"),
                EmittedRoot::Delivered(RootOutcome::Partial { table, affected }) => {
                    format!("partial {table:?} {affected:?}")
                }
                EmittedRoot::Delivered(RootOutcome::Failed(error)) => format!("failed {error:?}"),
                EmittedRoot::Delivered(RootOutcome::Unknown(error)) => format!("unknown {error:?}"),
                EmittedRoot::Delivered(RootOutcome::Deferred { phase }) => format!("deferred {phase:?}"),
                EmittedRoot::Delivered(RootOutcome::Skipped(_)) => "skipped".into(),
                EmittedRoot::Delivered(RootOutcome::NoQuery) => "no query".into(),
                EmittedRoot::Withheld => "withheld".into(),
            };
            let after = match ran.after {
                RuleAfter::Ran { captures } => format!("ran, {} captures", captures.len()),
                RuleAfter::Unreached => "unreached".into(),
                RuleAfter::Stopped(stop) => format!("stopped {stop:?}"),
            };
            writeln!(answered, "file {file} rule {}: {emitted}; {after}", ran.rule.0).unwrap();
        }
        let end = match rules.end {
            FileEnd::Finished(_) => "finished".to_owned(),
            FileEnd::Unfinished => "unfinished".to_owned(),
            FileEnd::Refused(stop) => format!("refused {stop:?}"),
        };
        writeln!(answered, "file {file}: {end}").unwrap();
    }
    request.finish();
    assert_eq!(
        answered,
        include_str!("../../fixtures/smoke/file_rules.txt"),
        "the file rules the engine answers"
    );
}
