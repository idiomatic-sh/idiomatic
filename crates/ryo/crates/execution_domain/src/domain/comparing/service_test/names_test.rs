//! How a comparison decides a name against a text: by the text's hash, once for each name, and across the files of a run.
use super::{FROM, column, holds_all};
use crate::domain::comparing::types::RowShape;
use crate::domain::operating::{errors::OperatingError, types::NameCount};
use crate::domain::table_model::types::Verdict;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::names::service::name_count;
use ryo_language::domain::symbols::service::intern;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{CellKind, Offset, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    LiteralText, Operand, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;

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
    let count = |test: Test| -> Result<usize, OperatingError> {
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
        let mut memo = Vec::new();
        let mut scratch = OperatorScratch::default();
        let Operation::Select { comparisons, .. } = operation else {
            panic!("the fixture builds a Select")
        };
        let names = NameCount(name_count(&symbols).0);
        let mut answers = Vec::new();
        for row in table.cells.chunks(table.kinds.len()) {
            answers.push(holds_all(
                &comparisons,
                row,
                RowShape {
                    kinds: &table.kinds,
                    names,
                },
                &symbols,
                source,
                &mut scratch,
                &mut memo,
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
    let Operation::Select { comparisons, .. } = operation else {
        panic!("the fixture builds a Select")
    };
    let names = NameCount(name_count(&symbols).0);
    let mut answers = Vec::new();
    for row in table.cells.chunks(table.kinds.len()) {
        answers.push(holds_all(
            &comparisons,
            row,
            RowShape {
                kinds: &table.kinds,
                names,
            },
            &symbols,
            source,
            &mut scratch,
            &mut memo,
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
        let mut memo = Vec::new();
        let Operation::Select { comparisons, .. } = &operation else {
            panic!("the fixture builds a Select")
        };
        let names = NameCount(name_count(&symbols).0);
        let mut answers = Vec::new();
        for row in table.cells.chunks(table.kinds.len()) {
            answers.push(holds_all(
                comparisons,
                row,
                RowShape {
                    kinds: &table.kinds,
                    names,
                },
                &symbols,
                source,
                &mut scratch,
                &mut memo,
            )?);
        }
        kept += answers
            .iter()
            .filter(|verdict| **verdict == Verdict::Holds)
            .count();
    }
    Ok(kept * 10 + scratch.select_scratch.verdicts.len())
}

type Input = fn() -> Result<Value, OperatingError>;
type Value = usize;
type Failure = OperatingError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

const CASES: &[Case<Input, Value, Failure>] = &[
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
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
