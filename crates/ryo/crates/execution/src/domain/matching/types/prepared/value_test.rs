//! Finite matcher slots retain exact identity, laziness and concurrent initialization.
use super::value::PreparedMatchers;
use crate::domain::{
    matching::service::matches_prepared,
    operating::errors::OperatingError,
    table_model::types::Verdict,
    table_scratch::types::Patterns,
    table_texts::types::{Datum, TextView},
};
use ryo_matching::inbound::use_cases::matching::{
    interface::MatchingUseCase,
    types::{MatchOutcome, SearchText},
    use_case::MatchingUseCaseImpl,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{
            ColumnIndex, Input as Source, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
            WholeNumber,
        },
        plans::types::Operation,
    },
};
use std::sync::atomic::Ordering;

/// The matching semantics the cases run through.
fn matcher() -> crate::domain::matching::types::PatternMatcher<'static> {
    crate::domain::matching::types::PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl)
}
#[derive(Clone, Copy)]
enum Scenario {
    SharedLiteral,
    InvalidLazy,
    GlobFamilies,
    Undeclared,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
fn select(test: Test, pattern: &str) -> Operation {
    Operation::Select {
        from: Source::Relation(Relation::Function),
        kept: vec![],
        comparisons: vec![PlanComparison {
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(pattern.into()))),
            test,
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        }],
    }
}
fn prepared(operations: &[Operation]) -> PreparedMatchers {
    PreparedMatchers::from_operations(operations.iter())
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::SharedLiteral => {
            let operations = [
                select(Test::Matches, r"\s+"),
                select(Test::Matches, r"\s+"),
                select(Test::GlobAny, "*.rs"),
                Operation::Replace {
                    from: Source::Relation(Relation::Function),
                    text: ColumnIndex(0),
                    pattern: LiteralText(r"\s+".into()),
                    replacement: LiteralText("$1".into()),
                },
            ];
            let slots = prepared(&operations);
            assert_eq!(slots.builds.load(Ordering::Relaxed), 0);
            for _file in 0..3 {
                let regex = slots.regex(TextView(r"\s+"), matcher()).unwrap()?;
                assert_eq!(
                    MatchingUseCaseImpl::new().matches_regex(regex, SearchText("é\u{2003}雪")),
                    MatchOutcome::Matches
                );
                assert!(std::ptr::eq(regex, slots.regex(TextView(r"\s+"), matcher()).unwrap()?));
                assert_eq!(
                    MatchingUseCaseImpl::new().matches_text_globs(
                        slots.glob(Test::GlobAny, TextView("*.rs"), matcher()).unwrap(),
                        SearchText("src/lib.rs")
                    ),
                    MatchOutcome::Matches
                );
            }
            assert_eq!(
                slots.builds.load(Ordering::Relaxed),
                2,
                "comparison and replacement share the exact regex slot"
            );
        }
        Scenario::InvalidLazy => {
            let slots = prepared(&[select(Test::Matches, "[")]);
            assert_eq!(slots.builds.load(Ordering::Relaxed), 0);
            assert_eq!(
                matches_prepared(
                    &Datum::Number(WholeNumber(1)),
                    &Datum::Text(TextView("[")),
                    Some(&slots),
                    &mut Patterns::default(),
                    matcher()
                )?,
                Verdict::Fails
            );
            assert_eq!(
                slots.builds.load(Ordering::Relaxed),
                0,
                "wrong-typed operands never initialize the slot"
            );
            let expected = super::value::compile_regex(TextView("["), matcher()).unwrap_err();
            for _ in 0..2 {
                assert!(matches!(slots.regex(TextView("["), matcher()).unwrap(),Err(ref error) if *error==expected));
            }
            assert_eq!(
                slots.builds.load(Ordering::Relaxed),
                1,
                "failed compilation is retained exactly once"
            );
        }
        Scenario::GlobFamilies => {
            let slots = prepared(&[
                select(Test::GlobAny, "*"),
                select(Test::NotGlobAny, "*"),
                select(Test::GlobAny, "α?"),
                select(Test::Matches, "*"),
            ]);
            for _ in 0..2 {
                let positive = slots.glob(Test::GlobAny, TextView("*"), matcher()).unwrap();
                let negative = slots.glob(Test::NotGlobAny, TextView("*"), matcher()).unwrap();
                assert!(
                    !std::ptr::eq(positive, negative),
                    "polarity retains a separate finite slot"
                );
                assert_eq!(
                    MatchingUseCaseImpl::new().matches_text_globs(positive, SearchText("αβ")),
                    MatchOutcome::Matches
                );
                assert_eq!(
                    MatchingUseCaseImpl::new().matches_text_globs(
                        slots.glob(Test::GlobAny, TextView("α?"), matcher()).unwrap(),
                        SearchText("αβ")
                    ),
                    MatchOutcome::Matches
                );
            }
            assert_eq!(
                slots.builds.load(Ordering::Relaxed),
                3,
                "Unicode interpreter outcome is retained"
            );
            assert!(slots.regex(TextView("*"), matcher()).unwrap().is_err());
            assert_eq!(
                slots.builds.load(Ordering::Relaxed),
                4,
                "regex and glob families cannot alias"
            );
        }
        Scenario::Undeclared => {
            let slots = prepared(&[select(Test::GlobAny, "b*")]);
            assert!(slots.glob(Test::GlobAny, TextView("*"), matcher()).is_none());
            assert!(slots.regex(TextView("b*"), matcher()).is_none());
            assert_eq!(
                slots.builds.load(Ordering::Relaxed),
                0,
                "a separate request cannot acquire another request's literal slot"
            );
        }
    }
    Ok(())
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("literal_regex_replace_and_glob_slots_survive_repeated_users"),
        word: Word::Succeeds,
        input: || Scenario::SharedLiteral,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("invalid_literals_stay_lazy_and_fail_once_on_demand"),
        word: Word::Succeeds,
        input: || Scenario::InvalidLazy,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("glob_polarity_unicode_and_regex_families_do_not_alias"),
        word: Word::Succeeds,
        input: || Scenario::GlobFamilies,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("undeclared_literals_do_not_enter_another_request"),
        word: Word::Succeeds,
        input: || Scenario::Undeclared,
        expect: Expect::Value(()),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn concurrent_literal_users_initialize_one_slot() {
    let slots = prepared(&[select(Test::Matches, "α+")]);
    std::thread::scope(|scope| {
        let slots = &slots;
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(move || {
                    MatchingUseCaseImpl::new()
                        .matches_regex(slots.regex(TextView("α+"), matcher()).unwrap().unwrap(), SearchText("αα"))
                        == MatchOutcome::Matches
                })
            })
            .collect();
        for handle in handles {
            assert!(handle.join().unwrap());
        }
    });
    assert_eq!(slots.builds.load(Ordering::Relaxed), 1);
}
