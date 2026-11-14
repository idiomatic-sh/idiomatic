//! The `comparing` service's cases: what is decided for a name or a text is remembered, and a comparison of what is not bound is refused.

use crate::domain::table_scratch::types::Memo;
use ryo_language::domain::symbols::service::intern;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{
    CellKind, Name, Offset, PlaceNumber, Relation, TextHash, Value as Cell,
};

use crate::domain::comparing::{
    service::holds_bound,
    types::{DecisionWorkspace, RowShape},
};
use crate::domain::patterns::types::PatternMatcher;
use crate::domain::comparing::{
    errors::ComparingError,
    types::NameCount,
};
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::Texts;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    Applied, ColumnIndex, Function, Input as PlanInput, LiteralText, Operand, ParameterId,
    PlanComparison, QueryLiteral, Test,
};

/// A scenario: what a row observes of the comparisons, as a number.
type Input = fn() -> Result<Value, ComparingError>;
/// The number a scenario observes: rows kept and decisions remembered, written as digits.
type Value = usize;
type Failure = ComparingError;

pub(super) const FROM: PlanInput = PlanInput::Relation(Relation::Function);

/// Whether every comparison holds for the row of a run over `symbols` and `source`, with nothing bound.
pub(super) fn holds_all(
    comparisons: &[PlanComparison],
    row: &[Cell],
    shape: RowShape<'_>,
    symbols: &Symbols,
    source: &[u8],
    scratch: &mut OperatorScratch,
    memo: &mut Vec<Memo>,
) -> Result<PredicateOutcome, ComparingError> {
    let texts = Texts {
        symbols,
        source: SourceView(source),
        paths: &[],
        arenas: &[],
    };
    let matcher = PatternMatcher(&MatchingUseCaseImpl);
    let work = DecisionWorkspace {
        select: &mut scratch.select_scratch,
        memo,
    };
    holds_bound(comparisons, row, shape, &texts, &[], None, matcher, work)
}

pub(super) fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

/// A pattern read from the row changes even when the text being tested stays the same.
fn reads_each_rows_pattern() -> Result<Value, ComparingError> {
    let source = b"mainm*o*";
    let mut symbols = Symbols::default();
    let [name, matches, differs] = [(0, 4), (4, 6), (6, 8)].map(|(start, end)| {
        Cell::Symbol(intern(
            &mut symbols,
            ByteSlice(source),
            Offset(start),
            Offset(end),
        ))
    });
    let comparisons = [PlanComparison {
        test: Test::GlobAny,
        left: Operand::Column(column(0)),
        right: Operand::Column(column(1)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }];
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut kept = 0;
    for pattern in [matches, differs] {
        kept += usize::from(
            holds_all(
                &comparisons,
                &[name, pattern],
                RowShape {
                    kinds: &[CellKind::Symbol, CellKind::Symbol],
                    names: NameCount(3),
                },
                &symbols,
                source,
                &mut scratch,
                &mut memo,
            )? == PredicateOutcome::Holds,
        );
    }
    Ok(kept)
}

/// A comparison with a parameter nobody bound is refused, not decided.
fn refuses_an_unbound_parameter() -> Result<Value, ComparingError> {
    let comparison = PlanComparison {
        test: Test::Equal,
        left: Operand::Column(column(0)),
        right: Operand::Parameter(ParameterId(0)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let symbols = Symbols::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    holds_all(
        &[comparison],
        &[Cell::Number(Integer(1))],
        RowShape {
            kinds: &[CellKind::Number],
            names: NameCount(0),
        },
        &symbols,
        &[],
        &mut scratch,
        &mut memo,
    )
    .map(|verdict| usize::from(verdict == PredicateOutcome::Holds))
}

/// A parameter bound to sets of text is no side of an equality: the binding is refused, not decided.
fn refuses_a_parameter_bound_to_sets_of_text() -> Result<Value, ComparingError> {
    let comparison = PlanComparison {
        test: Test::Equal,
        left: Operand::Column(column(0)),
        right: Operand::Parameter(ParameterId(0)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let symbols = Symbols::default();
    let texts = Texts {
        symbols: &symbols,
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
    };
    let bindings = [ryo_values::domain::query_bindings::types::ParameterBinding::TextSets(
        Vec::new(),
    )];
    let mut scratch = OperatorScratch::default();
    let work = DecisionWorkspace {
        select: &mut scratch.select_scratch,
        memo: &mut Vec::new(),
    };
    holds_bound(
        &[comparison],
        &[Cell::Number(Integer(1))],
        RowShape {
            kinds: &[CellKind::Number],
            names: NameCount(0),
        },
        &texts,
        &bindings,
        None,
        PatternMatcher(&MatchingUseCaseImpl),
        work,
    )
    .map(|verdict| usize::from(verdict == PredicateOutcome::Holds))
}

/// A pattern read from the row that is no regular expression fails the comparison with the matcher's reason.
fn fails_on_a_pattern_that_does_not_compile() -> Result<Value, ComparingError> {
    let source = b"[";
    let mut symbols = Symbols::default();
    let pattern = Cell::Symbol(intern(&mut symbols, ByteSlice(source), Offset(0), Offset(1)));
    let comparison = PlanComparison {
        test: Test::Matches,
        left: Operand::Column(column(0)),
        right: Operand::Column(column(0)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let mut scratch = OperatorScratch::default();
    holds_all(
        &[comparison],
        &[pattern],
        RowShape {
            kinds: &[CellKind::Symbol],
            names: NameCount(0),
        },
        &symbols,
        source,
        &mut scratch,
        &mut Vec::new(),
    )
    .map(|verdict| usize::from(verdict == PredicateOutcome::Holds))
}

/// A function of a name whose text is not held is not text: it equals no text, and starts and ends with none, as a span with no
/// place compares as unlocated.
fn a_function_of_an_unknown_name_is_unlocated() -> Result<Value, ComparingError> {
    let symbols = Symbols::default();
    let unknown = Cell::Name(Name {
        hash: TextHash(7),
        place: PlaceNumber(99),
    });
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut held = 0;
    for (place, test) in [
        Test::Equal,
        Test::NotEqual,
        Test::StartsWith,
        Test::EndsWith,
    ]
    .into_iter()
    .enumerate()
    {
        let comparison = PlanComparison {
            test,
            left: Operand::Function(Applied {
                function: Function::Snake,
                arguments: vec![Operand::Column(column(0))],
            }),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(String::new().into()))),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        };
        if holds_all(
            &[comparison],
            &[unknown],
            RowShape {
                kinds: &[CellKind::Symbol],
                names: NameCount(0),
            },
            &symbols,
            &[],
            &mut scratch,
            &mut memo,
        )? == PredicateOutcome::Holds
        {
            held += 10_usize.pow(u32::try_from(place).unwrap_or(0));
        }
    }
    Ok(held)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_pattern_read_from_a_row_can_change_between_rows"),
        word: Word::Succeeds,
        input: || reads_each_rows_pattern,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_function_of_a_name_whose_text_is_not_held_matches_no_text"),
        word: Word::Succeeds,
        input: || a_function_of_an_unknown_name_is_unlocated,
        expect: Expect::Value(10),
    },
    Case {
        name: CaseName("an_unbound_parameter_is_refused"),
        word: Word::Refuses,
        input: || refuses_an_unbound_parameter,
        expect: Expect::Error(ComparingError::Unbound),
    },
    Case {
        name: CaseName("a_parameter_bound_to_sets_of_text_is_refused"),
        word: Word::Refuses,
        input: || refuses_a_parameter_bound_to_sets_of_text,
        expect: Expect::Error(ComparingError::WrongBinding),
    },
    Case {
        name: CaseName("a_pattern_that_does_not_compile_fails_the_comparison"),
        word: Word::Fails,
        input: || fails_on_a_pattern_that_does_not_compile,
        expect: Expect::Refusal(|error| {
            ryo_values::selects!(matches!(error, ComparingError::InvalidRegex { .. }))
        }),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[cfg(test)]
mod names_test;
