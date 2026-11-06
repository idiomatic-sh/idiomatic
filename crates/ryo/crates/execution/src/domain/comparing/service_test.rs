//! The `comparing` service's cases: what is decided for a name or a text is remembered, and a comparison of what is not bound is refused.

use ryo_language::domain::symbols::service::intern;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{
    CellKind, Name, Offset, PlaceNumber, Relation, TextHash, Value as Cell,
};

use crate::domain::comparing::service::holds_all;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{NameCount, OperatorContext};
use crate::domain::table_model::types::Verdict;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::Applied;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::Function;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::ParameterId;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::service::push_row;

/// A scenario: what a row observes of the comparisons, as a number.
type Input = fn() -> Result<Value, OperatingError>;
/// The number a scenario observes: rows kept and decisions remembered, written as digits.
type Value = usize;
type Failure = OperatingError;

const FROM: PlanInput = PlanInput::Relation(Relation::Function);

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

/// Three rows hold two names between them; a glob against a fixed text is decided once for
/// each name, so two decisions are remembered, not three. (An equality to a fixed text is told by the hash of the
/// name, which needs no remembering.)
/// Names equal to a fixed text are told by their hash; a name of another text with the same length is not equal.
fn tells_a_name_equal_to_a_text_by_its_hash() -> Result<Value, OperatingError> {
    let source = b"mainmaiXother";
    let mut symbols = Symbols::default();
    let names = [(0, 4), (4, 8), (8, 13)].map(|(start, end)| {
        Cell::Symbol(intern(
            &mut symbols,
            ByteSlice(source),
            Offset(start),
            Offset(end),
        ))
    });
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    for name in names {
        push_row(&mut table, &[name]);
    }
    let mut count = |test: Test| -> Result<usize, OperatingError> {
        let comparison = PlanComparison {
            test,
            left: Operand::Column(column(0)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        };
        let operation = Operation::Select {
            from: FROM,
            comparisons: vec![comparison],
            kept: Vec::new(),
        };
        let mut context = OperatorContext {
            phase: crate::domain::operating::types::AfterTheFiles::Before,
            semantic: None,
            prepared_kinds: None,
            prepared_matchers: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(source),
            paths: &[],
            arenas: &[],
            memo: &mut Vec::new(),
            scratch: &mut OperatorScratch::default(),
        };
        let Operation::Select { comparisons, .. } = operation else {
            unreachable!()
        };
        let names = NameCount(
            ryo_language::domain::symbols::service::name_count(context.symbols.as_ref()).0,
        );
        let mut answers = Vec::new();
        for row in table.cells.chunks(table.kinds.len()) {
            answers.push(holds_all(
                &comparisons,
                row,
                &table.kinds,
                names,
                &mut context,
            )?);
        }
        Ok(answers
            .iter()
            .filter(|verdict| **verdict == Verdict::Holds)
            .count())
    };
    Ok(count(Test::Equal)? * 10 + count(Test::NotEqual)?)
}

fn decides_once_for_each_name() -> Result<Value, OperatingError> {
    let source = b"mainmainother";
    let mut symbols = Symbols::default();
    let names = [(0, 4), (4, 8), (8, 13)].map(|(start, end)| {
        Cell::Symbol(intern(
            &mut symbols,
            ByteSlice(source),
            Offset(start),
            Offset(end),
        ))
    });
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    for name in names {
        push_row(&mut table, &[name]);
    }
    let comparison = PlanComparison {
        test: Test::GlobAny,
        left: Operand::Column(column(0)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("{main}".into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let operation = Operation::Select {
        from: FROM,
        comparisons: vec![comparison],
        kept: Vec::new(),
    };
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(source),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let Operation::Select { comparisons, .. } = operation else {
        unreachable!()
    };
    let names =
        NameCount(ryo_language::domain::symbols::service::name_count(context.symbols.as_ref()).0);
    let mut answers = Vec::new();
    for row in table.cells.chunks(table.kinds.len()) {
        answers.push(holds_all(
            &comparisons,
            row,
            &table.kinds,
            names,
            &mut context,
        )?);
    }
    let kept = answers
        .iter()
        .filter(|verdict| **verdict == Verdict::Holds)
        .count();
    // Glob decisions are keyed by text in the shared semantic scratch.
    let decided = scratch.select_scratch.verdicts.len();
    Ok(kept * 10 + decided)
}

/// A name a comparison decided for one file is decided for the next file by its text: the verdict is
/// found by what the name says, not decided again, though the next file numbers its names otherwise.
fn remembers_what_a_text_was_decided_in_another_file() -> Result<Value, OperatingError> {
    let comparison = PlanComparison {
        test: Test::GlobAny,
        left: Operand::Column(column(0)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("{main,other}".into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let operation = Operation::Select {
        from: FROM,
        comparisons: vec![comparison],
        kept: Vec::new(),
    };
    let mut scratch = OperatorScratch::default();
    let mut kept = 0;
    // The first file has `skip` and `main`; the second `main` and `skip`, so the same text is a different name.
    for (source, order) in [
        (b"skipmain".as_slice(), [(0, 4), (4, 8)]),
        (b"mainskip".as_slice(), [(0, 4), (4, 8)]),
    ] {
        let mut symbols = Symbols::default();
        let names = order.map(|(start, end)| {
            Cell::Symbol(intern(
                &mut symbols,
                ByteSlice(source),
                Offset(start),
                Offset(end),
            ))
        });
        let mut table = Table {
            kinds: vec![CellKind::Symbol],
            ..Table::default()
        };
        for name in names {
            push_row(&mut table, &[name]);
        }
        let mut context = OperatorContext {
            phase: crate::domain::operating::types::AfterTheFiles::Before,
            semantic: None,
            prepared_kinds: None,
            prepared_matchers: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(source),
            paths: &[],
            arenas: &[],
            memo: &mut Vec::new(),
            scratch: &mut scratch,
        };
        let Operation::Select { comparisons, .. } = &operation else {
            unreachable!()
        };
        let names = NameCount(
            ryo_language::domain::symbols::service::name_count(context.symbols.as_ref()).0,
        );
        let mut answers = Vec::new();
        for row in table.cells.chunks(table.kinds.len()) {
            answers.push(holds_all(
                comparisons,
                row,
                &table.kinds,
                names,
                &mut context,
            )?);
        }
        kept += answers
            .iter()
            .filter(|verdict| **verdict == Verdict::Holds)
            .count();
    }
    Ok(kept * 10 + scratch.select_scratch.verdicts.len())
}

/// A pattern read from the row changes even when the text being tested stays the same.
fn reads_each_rows_pattern() -> Result<Value, OperatingError> {
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
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(source),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    let mut kept = 0;
    for pattern in [matches, differs] {
        kept += usize::from(
            holds_all(
                &comparisons,
                &[name, pattern],
                &[CellKind::Symbol, CellKind::Symbol],
                NameCount(3),
                &mut context,
            )? == Verdict::Holds,
        );
    }
    Ok(kept)
}

/// A comparison with a parameter nobody bound is refused, not decided.
fn refuses_an_unbound_parameter() -> Result<Value, OperatingError> {
    let comparison = PlanComparison {
        test: Test::Equal,
        left: Operand::Column(column(0)),
        right: Operand::Parameter(ParameterId(0)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    holds_all(
        &[comparison],
        &[Cell::Number(Integer(1))],
        &[CellKind::Number],
        NameCount(0),
        &mut context,
    )
    .map(|verdict| usize::from(verdict == Verdict::Holds))
}

/// A function of a name whose text is not held is not text: it equals no text, and starts and ends with none, as a span with no
/// place compares as unlocated.
fn a_function_of_an_unknown_name_is_unlocated() -> Result<Value, OperatingError> {
    let mut symbols = Symbols::default();
    let unknown = Cell::Name(Name {
        hash: TextHash(7),
        place: PlaceNumber(99),
    });
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
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
            &[CellKind::Symbol],
            NameCount(0),
            &mut context,
        )? == Verdict::Holds
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
        name: CaseName("a_name_equal_to_a_text_is_told_by_its_hash"),
        word: Word::Succeeds,
        input: || tells_a_name_equal_to_a_text_by_its_hash,
        expect: Expect::Value(12),
    },
    Case {
        name: CaseName("a_comparison_of_a_name_is_decided_once_for_each_distinct_name"),
        word: Word::Succeeds,
        input: || decides_once_for_each_name,
        expect: Expect::Value(22),
    },
    Case {
        name: CaseName("a_text_decided_in_one_file_is_not_decided_again_in_the_next"),
        word: Word::Succeeds,
        input: || remembers_what_a_text_was_decided_in_another_file,
        expect: Expect::Value(22),
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
        expect: Expect::Error(OperatingError::Unbound),
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
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::Table;
