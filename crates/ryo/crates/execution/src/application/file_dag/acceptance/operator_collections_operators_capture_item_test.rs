//! The `capture` item's cases.

use ryo_language::domain::symbols::service::intern;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Offset, Relation, Value as Cell};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::CapturePattern;
use ryo_values::domain::plans::types::QueryOperator;

use crate::application::file_dag::testing::TestOperator as _;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::capture::item::CaptureItem;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::service::push_row;

type Value = usize;
type Failure = OperatingError;
/// A scenario: what a row computes with the item, answering how many rows came out.
type Input = fn() -> Result<Value, OperatingError>;

const FROM: PlanInput = PlanInput::Relation(Relation::Function);

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

fn compute(
    operation: &Operation,
    inputs: &[&Table],
    bindings: &[ParameterBinding],
) -> Result<Value, OperatingError> {
    let mut symbols = Symbols::default();
    let rows: Vec<Rows<'_>> = inputs.iter().map(|table| Rows::Computed(table)).collect();
    let mut out = Table::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings,
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    CaptureItem
        .compute(operation, &rows, &mut context, &mut out)
        .map(|()| out.count.0)
}

fn captures_the_module_of_a_path() -> Result<Value, OperatingError> {
    let source = b"src/lib.rssrc/deep/mod.rsREADME.md";
    let mut symbols = Symbols::default();
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    for (start, end) in [(0, 10), (10, 25), (25, 34)] {
        push_row(
            &mut table,
            &[Cell::Symbol(intern(
                &mut symbols,
                ByteSlice(source),
                Offset(start),
                Offset(end),
            ))],
        );
    }
    let operation = Operation::Capture {
        from: FROM,
        path: column(0),
        pattern: CapturePattern("src/{module}.rs".into()),
    };
    let mut out = Table::default();
    let mut context = OperatorContext {
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
    CaptureItem
        .compute(
            &operation,
            &[Rows::Computed(&table)],
            &mut context,
            &mut out,
        )
        .map(|()| out.count.0)
}

/// A slot written `{*name}` takes text with `/` in it; the others stop at one.
fn a_starred_slot_crosses_directories() -> Result<Value, OperatingError> {
    let source = b"a/b/languages/py/src/x.rslanguages/py/x.rsa/languages/py";
    let mut symbols = Symbols::default();
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    for (start, end) in [(0, 25), (25, 41), (41, 57)] {
        push_row(
            &mut table,
            &[Cell::Symbol(intern(
                &mut symbols,
                ByteSlice(source),
                Offset(start),
                Offset(end),
            ))],
        );
    }
    let operation = Operation::Capture {
        from: FROM,
        path: column(0),
        pattern: CapturePattern("{*above}/languages/{name}/{*below}".into()),
    };
    let mut out = Table::default();
    let mut context = OperatorContext {
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
    CaptureItem.compute(
        &operation,
        &[Rows::Computed(&table)],
        &mut context,
        &mut out,
    )?;
    let text = |at: usize| match out.cells.get(at) {
        Some(Cell::Symbol(symbol)) => {
            ryo_language::domain::symbols::service::text_of(&symbols, ByteSlice(source), *symbol)
                .map(|text| text.0.to_owned())
        }
        _ => None,
    };
    // Only the first has a directory above `languages`; the second has none, the third nothing below the name.
    Ok(usize::from(
        out.count.0 == 1
            && text(1).as_deref() == Some("a/b")
            && text(2).as_deref() == Some("py")
            && text(3).as_deref() == Some("src/x.rs"),
    ))
}

/// Every way a pattern matches a path is a row: two stars side by side split a path at each `/`.
fn every_split_of_a_path_is_a_row() -> Result<Value, OperatingError> {
    let source = b"x/y/z.rs";
    let mut symbols = Symbols::default();
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    push_row(
        &mut table,
        &[Cell::Symbol(intern(
            &mut symbols,
            ByteSlice(source),
            Offset(0),
            Offset(8),
        ))],
    );
    let operation = Operation::Capture {
        from: FROM,
        path: column(0),
        pattern: CapturePattern("{*ancestor}/{*rest}".into()),
    };
    let mut out = Table::default();
    let mut context = OperatorContext {
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
    CaptureItem.compute(
        &operation,
        &[Rows::Computed(&table)],
        &mut context,
        &mut out,
    )?;
    let text = |at: usize| match out.cells.get(at) {
        Some(Cell::Symbol(symbol)) => {
            ryo_language::domain::symbols::service::text_of(&symbols, ByteSlice(source), *symbol)
                .map(|text| text.0.to_owned())
        }
        _ => None,
    };
    // Two rows of three cells: the path, the ancestor, the rest.
    Ok(usize::from(
        out.count.0 == 2
            && text(1).as_deref() == Some("x")
            && text(2).as_deref() == Some("y/z.rs")
            && text(4).as_deref() == Some("x/y")
            && text(5).as_deref() == Some("z.rs"),
    ))
}

fn captures_without_an_input() -> Result<Value, OperatingError> {
    let operation = Operation::Capture {
        from: FROM,
        path: column(0),
        pattern: CapturePattern(Box::default()),
    };
    compute(&operation, &[], &[])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("only_matching_paths_remain"),
        word: Word::Succeeds,
        input: || captures_the_module_of_a_path,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_starred_slot_takes_directories"),
        word: Word::Succeeds,
        input: || a_starred_slot_crosses_directories,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("every_way_a_pattern_matches_is_a_row"),
        word: Word::Succeeds,
        input: || every_split_of_a_path_is_a_row,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_missing_input_is_refused"),
        word: Word::Refuses,
        input: || captures_without_an_input,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, OperatingError::MissingInput { .. }))
        }),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    crate::domain::operator::collections::operators::fixtures::contract::check(
        QueryOperator::Capture,
    );
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::Table;

use ryo_values::domain::query_bindings::types::ParameterBinding;
