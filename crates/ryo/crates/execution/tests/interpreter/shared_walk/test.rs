//! The `shared_walk` cases.
use crate::boundary_fixture::fixtures::{corpus, demand};
use crate::interpreter::shared_walk::fixtures::counts;
use crate::shared_walk::fixtures::no_catalog;
use crate::shared_walk::fixtures::*;
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityReason;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution;
use ryo_relations::domain::relations::types::FileId;

#[test]
fn two_distinct_singleton_selectors_share_one_actual_native_walk() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::Kind("word"), Selector::Kind("other")]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let mut file = request
        .open_file(
            input(
                FileId(0),
                b"abc",
                &elements,
                &[
                    FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::Disabled,
                    },
                    FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::Disabled,
                    },
                ],
                &no_catalog,
            ),
            &mut symbols,
        )
        .unwrap();
    counts(&file, 0, 0);
    let words = complete(file.rule(RuleIndex(0), demand(), &mut symbols).unwrap());
    assert_eq!(words, expected(FileId(0), &elements, &[1, 3]));
    counts(&file, 1, 4);
    let other = complete(file.rule(RuleIndex(1), demand(), &mut symbols).unwrap());
    assert_eq!(other, expected(FileId(0), &elements, &[2]));
    counts(&file, 1, 4);
    assert_eq!(
        semantics.selects(),
        0,
        "neither native class rescans through ordinary Select dispatch"
    );
    file.finish().unwrap();
    request.finish();
}

#[test]
fn inactive_file_has_zero_shared_walk_work() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::Kind("word"), Selector::BadRegex]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let file = request
        .open_file(
            input(
                FileId(0),
                b"abc",
                &elements,
                &[FileRuleIntent::Undemanded, FileRuleIntent::Undemanded],
                &no_catalog,
            ),
            &mut symbols,
        )
        .unwrap();
    counts(&file, 0, 0);
    assert_eq!(semantics.selects(), 0);
    file.finish().unwrap();
    request.finish();
}

#[test]
fn skipping_all_demanded_rules_does_not_start_the_walk() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::BadRegex, Selector::Kind("other")]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let mut file = request
        .open_file(
            input(
                FileId(0),
                b"abc",
                &elements,
                &[FileRuleIntent::Skipped(ApplicabilityReason::DiagnosticNotApplicable); 2],
                &no_catalog,
            ),
            &mut symbols,
        )
        .unwrap();
    counts(&file, 0, 0);
    assert_eq!(semantics.selects(), 0);
    file.finish().unwrap();
    request.finish();
}

#[test]
fn equal_predicates_with_different_parameters_have_independent_native_results() {
    let (engine, semantics) = engine();
    let program = prepared(
        &engine,
        &[Selector::Parameter("a"), Selector::Parameter("c")],
    );
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let mut file = request
        .open_file(
            input(
                FileId(0),
                b"abc",
                &elements,
                &[
                    FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::Disabled,
                    },
                    FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::Disabled,
                    },
                ],
                &no_catalog,
            ),
            &mut symbols,
        )
        .unwrap();
    assert_eq!(
        complete(file.rule(RuleIndex(1), demand(), &mut symbols).unwrap()),
        expected(FileId(0), &elements, &[3])
    );
    assert_eq!(
        complete(file.rule(RuleIndex(0), demand(), &mut symbols).unwrap()),
        expected(FileId(0), &elements, &[1])
    );
    counts(&file, 1, 4);
    assert_eq!(semantics.selects(), 0);
    file.finish().unwrap();
    request.finish();
}

#[test]
fn a_fault_in_another_active_class_is_delayed_until_that_rule_is_requested() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::BadRegex, Selector::Kind("other")]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let mut file = request
        .open_file(
            input(
                FileId(0),
                b"abc",
                &elements,
                &[
                    FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::Disabled,
                    },
                    FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::Disabled,
                    },
                ],
                &no_catalog,
            ),
            &mut symbols,
        )
        .unwrap();
    assert_eq!(
        complete(file.rule(RuleIndex(1), demand(), &mut symbols).unwrap()),
        expected(FileId(0), &elements, &[2])
    );
    counts(&file, 1, 4);
    failed(
        file.rule(RuleIndex(0), demand(), &mut symbols).unwrap(),
        regex_error("["),
    );
    assert_eq!(
        complete(file.rule(RuleIndex(1), demand(), &mut symbols).unwrap()),
        expected(FileId(0), &elements, &[2])
    );
    counts(&file, 1, 4);
    assert_eq!(semantics.selects(), 0);
    file.finish().unwrap();
    request.finish();
}

#[test]
fn the_second_file_starts_with_fresh_walk_state_and_its_own_source_rows() {
    let (engine, semantics) = engine();
    let program = prepared(
        &engine,
        &[Selector::Parameter("a"), Selector::Parameter("c")],
    );
    let mut corpus = corpus();
    let mut second = corpus[0].clone();
    let ryo_values::domain::files::types::CorpusInput::Available(entry) = &mut second else {
        panic!("available authored corpus")
    };
    entry.path = ryo_language::domain::files::types::FilePath("second.py".into());
    corpus.push(second);
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    for (file_id, source, first, last) in [
        (FileId(0), b"abc".as_slice(), 1, 3),
        (FileId(1), b"cba".as_slice(), 3, 1),
    ] {
        let (elements, mut symbols) = tree(file_id, source);
        let mut file = request
            .open_file(
                input(
                    file_id,
                    source,
                    &elements,
                    &[
                        FileRuleIntent::Run {
                            diagnostic: DiagnosticDelivery::Collected,
                            observations: ObservationDemand::NotRequired,
                            capture: CaptureIntent::Disabled,
                        },
                        FileRuleIntent::Run {
                            diagnostic: DiagnosticDelivery::Collected,
                            observations: ObservationDemand::NotRequired,
                            capture: CaptureIntent::Disabled,
                        },
                    ],
                    &no_catalog,
                ),
                &mut symbols,
            )
            .unwrap();
        counts(&file, 0, 0);
        assert_eq!(
            complete(file.rule(RuleIndex(0), demand(), &mut symbols).unwrap()),
            expected(file_id, &elements, &[first])
        );
        assert_eq!(
            complete(file.rule(RuleIndex(1), demand(), &mut symbols).unwrap()),
            expected(file_id, &elements, &[last])
        );
        counts(&file, 1, 4);
        file.finish().unwrap();
    }
    assert_eq!(semantics.selects(), 0);
    request.finish();
}
