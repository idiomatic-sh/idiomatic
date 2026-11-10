//! Three rules over three files, answered as the recorded fixture.
use super::super::super::*;

/// Three rules over three files through the one-call file route answer exactly what `tests/fixtures/smoke/file_rules.txt`
/// recorded: each rule's emitted root and how its run ended, file by file. A change to the engine behind the seam that changes
/// any answer changes this text.
#[test]
fn three_rules_over_three_files_answer_the_recorded_file_rules() {
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
        let run = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        };
        let (ran, end) = crate::file_intents::ran_file(
            &request,
            FileId(file),
            &[run, run, run],
            (&elements, &mut symbols),
            worker.as_ref(),
        );
        for (rule, ran) in ran.iter().enumerate() {
            writeln!(
                answered,
                "file {file} rule {rule}: {}; {}",
                ran.emitted, ran.after
            )
            .unwrap();
        }
        writeln!(answered, "file {file}: {end}").unwrap();
    }
    request.finish();
    assert_eq!(
        answered,
        include_str!("../../fixtures/smoke/file_rules.txt"),
        "the file rules the engine answers"
    );
}
