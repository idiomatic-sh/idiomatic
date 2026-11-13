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
use std::borrow::Cow;
use std::sync::atomic::Ordering;

/// The matching semantics the cases run through.
fn matcher() -> crate::domain::matching::types::PatternMatcher<'static> {
    crate::domain::matching::types::PatternMatcher(&MatchingUseCaseImpl)
}
/// An operation of the request that declares a literal pattern.
#[derive(Clone, Copy, Debug)]
enum Declared {
    /// A select over `function` comparing by the test with the pattern.
    Select(Test, &'static str),
    /// A replacement of the pattern.
    Replace(&'static str),
    /// A join whose right input is filtered by a regex match of the pattern.
    Join(&'static str),
    /// An axis walk whose domain is filtered by a regex match of the pattern.
    Axis(&'static str),
}

/// A finite slot of the request, by its family, test and pattern.
#[derive(Clone, Copy, Debug)]
enum Slot {
    Regex(&'static str),
    Glob(Test, &'static str),
}

/// One use of the request's slots, in order.
#[derive(Clone, Copy, Debug)]
enum Lookup {
    /// The regex slot of the pattern, run over the text.
    Regex(&'static str, &'static str),
    /// The glob slot of the test and pattern, run over the text.
    Glob(Test, &'static str, &'static str),
    /// Whether two slots are one.
    Same(Slot, Slot),
    /// The verdict of a regex match of the number 1 by the pattern, through the prepared slots.
    Number(&'static str),
}

/// The operations a request declares, and the uses of its slots in order.
#[derive(Clone, Copy, Debug)]
struct Request {
    declared: &'static [Declared],
    lookups: &'static [Lookup],
}

/// What one use of a slot found.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Found {
    /// The request declares no such slot.
    Undeclared,
    /// The slot holds a pattern, which answered this over the text.
    Outcome(MatchOutcome),
    /// The slot holds a compilation failure, and whether it is the failure compiling the pattern gives.
    Refused { as_compiled: bool },
    /// Whether the two slots are one.
    Same(bool),
    /// The comparison's verdict.
    Verdict(Verdict),
}

/// How many slots were built before any use, and what each use found with the builds after it.
#[derive(Debug, PartialEq)]
struct Observed {
    built: usize,
    found: Cow<'static, [(Found, usize)]>,
}

type Input = Request;
type Value = Observed;
type Failure = OperatingError;
fn literal(test: Test, pattern: &str) -> PlanComparison {
    PlanComparison {
        left: Operand::Column(ColumnIndex(0)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(pattern.into()))),
        test,
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}
fn select(test: Test, pattern: &str) -> Operation {
    Operation::Select {
        from: Source::Relation(Relation::Function),
        kept: vec![],
        comparisons: vec![literal(test, pattern)],
    }
}
/// A join whose right input is filtered by `comparison`.
fn join(comparison: PlanComparison) -> Operation {
    Operation::Join {
        left: Source::Relation(Relation::Function),
        right: Source::Relation(Relation::SyntaxTree),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![],
        filter: vec![comparison],
        view: vec![],
        reach: None,
    }
}
/// An axis walk whose domain is filtered by `comparison`.
fn axis(comparison: PlanComparison) -> Operation {
    Operation::Axis {
        from: Source::Relation(Relation::Function),
        edges: Source::Relation(Relation::SyntaxChild),
        domain: Source::Relation(Relation::SyntaxTree),
        direction: ryo_values::domain::plan_parts::types::Direction::Child,
        id: ColumnIndex(0),
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(0),
        domain_filter: vec![comparison],
        domain_view: vec![],
        output: vec![],
    }
}
fn prepared(operations: &[Operation]) -> PreparedMatchers {
    PreparedMatchers::from_operations(operations.iter())
}
fn operation(declared: Declared) -> Operation {
    match declared {
        Declared::Select(test, pattern) => select(test, pattern),
        Declared::Replace(pattern) => Operation::Replace {
            from: Source::Relation(Relation::Function),
            text: ColumnIndex(0),
            pattern: LiteralText(pattern.into()),
            replacement: LiteralText("$1".into()),
        },
        Declared::Join(pattern) => join(literal(Test::Matches, pattern)),
        Declared::Axis(pattern) => axis(literal(Test::Matches, pattern)),
    }
}
/// Where a slot's pattern lives, to tell whether two slots are one.
fn address(slots: &PreparedMatchers, slot: Slot) -> Option<*const ()> {
    match slot {
        Slot::Regex(pattern) => slots
            .regex(TextView(pattern), matcher())?
            .ok()
            .map(|regex| std::ptr::from_ref(regex).cast()),
        Slot::Glob(test, pattern) => slots
            .glob(test, TextView(pattern), matcher())
            .map(|glob| std::ptr::from_ref(glob).cast()),
    }
}
fn found(slots: &PreparedMatchers, lookup: Lookup) -> Result<Found, Failure> {
    let matching = MatchingUseCaseImpl::new();
    Ok(match lookup {
        Lookup::Regex(pattern, text) => match slots.regex(TextView(pattern), matcher()) {
            None => Found::Undeclared,
            Some(Ok(regex)) => Found::Outcome(matching.matches_regex(regex, SearchText(text))),
            Some(Err(error)) => Found::Refused {
                as_compiled: super::value::compile_regex(TextView(pattern), matcher()).err()
                    == Some(error),
            },
        },
        Lookup::Glob(test, pattern, text) => match slots.glob(test, TextView(pattern), matcher()) {
            None => Found::Undeclared,
            Some(glob) => Found::Outcome(matching.matches_text_globs(glob, SearchText(text))),
        },
        Lookup::Same(left, right) => match (address(slots, left), address(slots, right)) {
            (Some(left), Some(right)) => Found::Same(std::ptr::eq(left, right)),
            _ => Found::Undeclared,
        },
        Lookup::Number(pattern) => Found::Verdict(matches_prepared(
            &Datum::Number(WholeNumber(1)),
            &Datum::Text(TextView(pattern)),
            Some(slots),
            &mut Patterns::default(),
            matcher(),
        )?),
    })
}
fn subject(request: Input) -> Result<Value, Failure> {
    let operations: Vec<Operation> = request.declared.iter().copied().map(operation).collect();
    let slots = prepared(&operations);
    let built = slots.builds.load(Ordering::Relaxed);
    let found = request
        .lookups
        .iter()
        .map(|lookup| {
            Ok((
                found(&slots, *lookup)?,
                slots.builds.load(Ordering::Relaxed),
            ))
        })
        .collect::<Result<Vec<_>, Failure>>()?;
    Ok(Observed {
        built,
        found: found.into(),
    })
}
/// One file's uses of the shared literal: the regex over Unicode space, the regex again, the glob over a path.
const SHARED_LITERAL_USES: [Lookup; 3] = [
    Lookup::Regex(r"\s+", "é\u{2003}雪"),
    Lookup::Same(Slot::Regex(r"\s+"), Slot::Regex(r"\s+")),
    Lookup::Glob(Test::GlobAny, "*.rs", "src/lib.rs"),
];
/// One pass over the glob families: the two polarities of `*` are separate, `*` and `α?` match `αβ`.
const GLOB_FAMILY_USES: [Lookup; 3] = [
    Lookup::Same(
        Slot::Glob(Test::GlobAny, "*"),
        Slot::Glob(Test::NotGlobAny, "*"),
    ),
    Lookup::Glob(Test::GlobAny, "*", "αβ"),
    Lookup::Glob(Test::GlobAny, "α?", "αβ"),
];
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("literal_regex_replace_and_glob_slots_survive_repeated_users"),
        word: Word::Succeeds,
        input: || Request {
            declared: &[
                Declared::Select(Test::Matches, r"\s+"),
                Declared::Select(Test::Matches, r"\s+"),
                Declared::Select(Test::GlobAny, "*.rs"),
                Declared::Replace(r"\s+"),
            ],
            lookups: &[
                SHARED_LITERAL_USES[0],
                SHARED_LITERAL_USES[1],
                SHARED_LITERAL_USES[2],
                SHARED_LITERAL_USES[0],
                SHARED_LITERAL_USES[1],
                SHARED_LITERAL_USES[2],
                SHARED_LITERAL_USES[0],
                SHARED_LITERAL_USES[1],
                SHARED_LITERAL_USES[2],
            ],
        },
        expect: Expect::Value(Observed {
            built: 0,
            found: Cow::Borrowed(&[
                (Found::Outcome(MatchOutcome::Matches), 1),
                (Found::Same(true), 1),
                (Found::Outcome(MatchOutcome::Matches), 2),
                (Found::Outcome(MatchOutcome::Matches), 2),
                (Found::Same(true), 2),
                (Found::Outcome(MatchOutcome::Matches), 2),
                (Found::Outcome(MatchOutcome::Matches), 2),
                (Found::Same(true), 2),
                (Found::Outcome(MatchOutcome::Matches), 2),
            ]),
        }),
    },
    Case {
        name: CaseName("invalid_literals_stay_lazy_and_fail_once_on_demand"),
        word: Word::Succeeds,
        input: || Request {
            declared: &[Declared::Select(Test::Matches, "[")],
            lookups: &[
                Lookup::Number("["),
                Lookup::Regex("[", ""),
                Lookup::Regex("[", ""),
            ],
        },
        expect: Expect::Value(Observed {
            built: 0,
            found: Cow::Borrowed(&[
                (Found::Verdict(Verdict::Fails), 0),
                (Found::Refused { as_compiled: true }, 1),
                (Found::Refused { as_compiled: true }, 1),
            ]),
        }),
    },
    Case {
        name: CaseName("glob_polarity_unicode_and_regex_families_do_not_alias"),
        word: Word::Succeeds,
        input: || Request {
            declared: &[
                Declared::Select(Test::GlobAny, "*"),
                Declared::Select(Test::NotGlobAny, "*"),
                Declared::Select(Test::GlobAny, "α?"),
                Declared::Select(Test::Matches, "*"),
            ],
            lookups: &[
                GLOB_FAMILY_USES[0],
                GLOB_FAMILY_USES[1],
                GLOB_FAMILY_USES[2],
                GLOB_FAMILY_USES[0],
                GLOB_FAMILY_USES[1],
                GLOB_FAMILY_USES[2],
                Lookup::Regex("*", ""),
            ],
        },
        expect: Expect::Value(Observed {
            built: 0,
            found: Cow::Borrowed(&[
                (Found::Same(false), 2),
                (Found::Outcome(MatchOutcome::Matches), 2),
                (Found::Outcome(MatchOutcome::Matches), 3),
                (Found::Same(false), 3),
                (Found::Outcome(MatchOutcome::Matches), 3),
                (Found::Outcome(MatchOutcome::Matches), 3),
                (Found::Refused { as_compiled: true }, 4),
            ]),
        }),
    },
    Case {
        name: CaseName("undeclared_literals_do_not_enter_another_request"),
        word: Word::Succeeds,
        input: || Request {
            declared: &[Declared::Select(Test::GlobAny, "b*")],
            lookups: &[
                Lookup::Glob(Test::GlobAny, "*", ""),
                Lookup::Regex("b*", ""),
            ],
        },
        expect: Expect::Value(Observed {
            built: 0,
            found: Cow::Borrowed(&[(Found::Undeclared, 0), (Found::Undeclared, 0)]),
        }),
    },
    Case {
        name: CaseName("join_literals_are_declared_before_any_worker"),
        word: Word::Succeeds,
        input: || Request {
            declared: &[Declared::Join("α+")],
            lookups: &[Lookup::Regex("α+", "αα")],
        },
        expect: Expect::Value(Observed {
            built: 0,
            found: Cow::Borrowed(&[(Found::Outcome(MatchOutcome::Matches), 1)]),
        }),
    },
    Case {
        name: CaseName("axis_literals_are_declared_before_any_worker"),
        word: Word::Succeeds,
        input: || Request {
            declared: &[Declared::Axis("α+")],
            lookups: &[Lookup::Regex("α+", "αα")],
        },
        expect: Expect::Value(Observed {
            built: 0,
            found: Cow::Borrowed(&[(Found::Outcome(MatchOutcome::Matches), 1)]),
        }),
    },
    Case {
        name: CaseName("replace_literals_are_declared_before_any_worker"),
        word: Word::Succeeds,
        input: || Request {
            declared: &[Declared::Replace("α+")],
            lookups: &[Lookup::Regex("α+", "αα")],
        },
        expect: Expect::Value(Observed {
            built: 0,
            found: Cow::Borrowed(&[(Found::Outcome(MatchOutcome::Matches), 1)]),
        }),
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
                    MatchingUseCaseImpl::new().matches_regex(
                        slots.regex(TextView("α+"), matcher()).unwrap().unwrap(),
                        SearchText("αα"),
                    ) == MatchOutcome::Matches
                })
            })
            .collect();
        for handle in handles {
            assert!(handle.join().unwrap());
        }
    });
    assert_eq!(slots.builds.load(Ordering::Relaxed), 1);
}
