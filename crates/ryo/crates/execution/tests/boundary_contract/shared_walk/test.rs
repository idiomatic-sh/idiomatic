//! The public file owner shares one measured outer walk among distinct selectors.
use super::fixtures::*;
use crate::boundary_fixture::fixtures::corpus;
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityReason;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileEnd;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_relations::domain::relations::types::FileId;

#[test]
fn two_distinct_singleton_selectors_share_one_actual_native_walk() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::Kind("word"), Selector::Kind("other")]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let mut rules = run(
        &request,
        (FileId(0), b"abc"),
        &elements,
        &[COLLECTED, COLLECTED],
        &mut symbols,
    );
    let words = complete(&mut rules, 0);
    assert_eq!(words, expected(FileId(0), &elements, &[1, 3]));
    let other = complete(&mut rules, 1);
    assert_eq!(other, expected(FileId(0), &elements, &[2]));
    assert_eq!(
        semantics.selects(),
        0,
        "neither native class rescans through ordinary Select dispatch"
    );
    assert!(matches!(rules.end, FileEnd::Finished(_)));
    request.finish();
}

#[test]
fn inactive_file_has_zero_shared_walk_work() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::Kind("word"), Selector::BadRegex]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let rules = run(
        &request,
        (FileId(0), b"abc"),
        &elements,
        &[FileRuleIntent::Undemanded, FileRuleIntent::Undemanded],
        &mut symbols,
    );
    assert!(rules.rules.is_empty());
    assert_eq!(semantics.selects(), 0);
    assert!(matches!(rules.end, FileEnd::Finished(_)));
    request.finish();
}

#[test]
fn skipping_all_demanded_rules_does_not_start_the_walk() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::BadRegex, Selector::Kind("other")]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let rules = run(
        &request,
        (FileId(0), b"abc"),
        &elements,
        &[FileRuleIntent::Skipped(ApplicabilityReason::DiagnosticNotApplicable); 2],
        &mut symbols,
    );
    assert!(rules.rules.is_empty());
    assert_eq!(semantics.selects(), 0);
    assert!(matches!(rules.end, FileEnd::Finished(_)));
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
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let mut rules = run(
        &request,
        (FileId(0), b"abc"),
        &elements,
        &[COLLECTED, COLLECTED],
        &mut symbols,
    );
    assert_eq!(
        complete(&mut rules, 1),
        expected(FileId(0), &elements, &[3])
    );
    assert_eq!(
        complete(&mut rules, 0),
        expected(FileId(0), &elements, &[1])
    );
    assert_eq!(semantics.selects(), 0);
    assert!(matches!(rules.end, FileEnd::Finished(_)));
    request.finish();
}

/// A fault in one active class stays with its own rule: the rule before it, which shares the walk, delivers its rows, and the
/// faulty rule answers its own error.
#[test]
fn a_fault_in_another_active_class_is_delayed_until_that_rule_is_requested() {
    let (engine, semantics) = engine();
    let program = prepared(&engine, &[Selector::Kind("other"), Selector::BadRegex]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let mut rules = run(
        &request,
        (FileId(0), b"abc"),
        &elements,
        &[COLLECTED, COLLECTED],
        &mut symbols,
    );
    assert_eq!(
        complete(&mut rules, 0),
        expected(FileId(0), &elements, &[2])
    );
    assert_eq!(failed(&mut rules, 1), Some(regex_error("[")));
    assert_eq!(semantics.selects(), 0);
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
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    for (file_id, source, first, last) in [(FileId(0), b"abc", 1, 3), (FileId(1), b"cba", 3, 1)] {
        let (elements, mut symbols) = tree(file_id, source);
        let mut rules = run(
            &request,
            (file_id, source),
            &elements,
            &[COLLECTED, COLLECTED],
            &mut symbols,
        );
        assert_eq!(
            complete(&mut rules, 0),
            expected(file_id, &elements, &[first])
        );
        assert_eq!(
            complete(&mut rules, 1),
            expected(file_id, &elements, &[last])
        );
        assert!(matches!(rules.end, FileEnd::Finished(_)));
    }
    assert_eq!(semantics.selects(), 0);
    request.finish();
}

#[test]
fn the_fixture_tree_has_four_real_elements_and_its_whole_vocabulary_named() {
    use ryo_language::domain::elements::fixtures::{kind_names, real_count};
    let (elements, _) = tree(FileId(0), b"abc");
    assert_eq!(real_count(&elements).0, 4);
    assert!(
        kind_names(&elements).len() > 3,
        "fully completed vocabulary enables the physical shared pass"
    );
}
