//! The `extend` item's cases.

use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_values::domain::plans::types::QueryOperator;

use ryo_language::domain::symbols::types::Symbols;

use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::extend::item::ExtendItem;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::ParameterId;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plans::types::Operation;

type Value = usize;
type Failure = OperatingError;
/// A scenario: what a row computes with the item, answering how many rows came out.
type Input = fn() -> Result<Value, OperatingError>;

const FROM: PlanInput = PlanInput::Relation(Relation::Function);

/// A table of numbers, one row for each slice.
fn numbers(rows: &[&[i64]]) -> Table {
    let width = rows.first().map_or(1, |row| row.len());
    let mut table = Table {
        kinds: vec![CellKind::Number; width],
        ..Table::default()
    };
    for row in rows {
        push_row(
            &mut table,
            &row.iter()
                .map(|number| Cell::Number(Integer(*number)))
                .collect::<Vec<_>>(),
        );
    }
    table
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
    ExtendItem
        .compute(operation, &rows, &mut context, &mut out)
        .map(|()| out.count.0)
}

fn appends_a_number_and_a_parameter() -> Result<Value, OperatingError> {
    let operation = Operation::Extend {
        from: FROM,
        values: vec![
            Operand::Literal(QueryLiteral::Number(Integer(1))),
            Operand::Parameter(ParameterId(0)),
        ],
    };
    compute(
        &operation,
        &[&numbers(&[&[1]])],
        &[ParameterBinding::Number(
            ryo_values::domain::plan_parts::types::WholeNumber(7),
        )],
    )
}

/// A text is appended as a name: the same text is one name, and its text is read back from the names.
fn appends_a_text_and_a_text_parameter() -> Result<Value, OperatingError> {
    let operation = Operation::Extend {
        from: FROM,
        values: vec![
            Operand::Literal(QueryLiteral::Text(LiteralText("here".into()))),
            Operand::Parameter(ParameterId(0)),
        ],
    };
    let table = numbers(&[&[1], &[2]]);
    let mut symbols = Symbols::default();
    let bindings = [ParameterBinding::Text(LiteralText("here".into()))];
    let mut out = Table::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &bindings,
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    ExtendItem.compute(
        &operation,
        &[Rows::Computed(&table)],
        &mut context,
        &mut out,
    )?;
    let text = |at: usize| match out.cells.get(at) {
        Some(Cell::Symbol(symbol)) => {
            ryo_language::domain::symbols::service::text_of(&symbols, ByteSlice(&[]), *symbol)
                .map(|text| text.0.to_owned())
        }
        _ => None,
    };
    // Two rows of three cells: the number, the literal, the parameter; the same text is the same name.
    Ok(usize::from(
        out.kinds == [CellKind::Number, CellKind::Symbol, CellKind::Symbol]
            && text(1).as_deref() == Some("here")
            && out.cells[1] == out.cells[2]
            && out.cells[2] == out.cells[4]
            && text(5).as_deref() == Some("here"),
    ))
}

/// After the files were read a text is appended as a name that left its file, which is the name the files hand over for the same text.
fn appends_a_name_that_compares_with_the_names_of_the_files() -> Result<Value, OperatingError> {
    let operation = Operation::Extend {
        from: FROM,
        values: vec![Operand::Literal(QueryLiteral::Text(LiteralText(
            "here".into(),
        )))],
    };
    let table = numbers(&[&[1]]);
    let mut symbols = Symbols::default();
    let arenas = vec![ryo_values::domain::stored_facts::types::ArenaBytes(
        b"here".to_vec(),
    )];
    let mut out = Table::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &arenas,
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    ExtendItem.compute(
        &operation,
        &[Rows::Computed(&table)],
        &mut context,
        &mut out,
    )?;
    // The file handed over `here` as text in worker 0's arena, with the hash of the text.
    let handed = ryo_language::domain::symbols::service::hash_in(
        &symbols,
        ryo_relations::domain::terms::types::Symbol(0),
    );
    let Some(Cell::Name(made)) = out.cells.get(1).copied() else {
        return Ok(0);
    };
    let texts = crate::domain::table_texts::types::Texts {
        symbols: &symbols,
        source: SourceView(&[]),
        paths: &[],
        arenas: &arenas,
    };
    let made_here = ryo_language::domain::symbols::service::place_of(made, &[], &symbols);
    Ok(usize::from(
        made_here.is_some_and(|place| {
            place.origin
                == ryo_relations::domain::relations::types::Origin::Derived(
                    ryo_relations::domain::relations::types::CROSS_FILE,
                )
        }) && Some(made.hash) == handed
            && crate::domain::table_texts::service::datum(Cell::Name(made), &texts)
                == crate::domain::table_texts::types::Datum::Text(
                    crate::domain::table_texts::types::TextView("here"),
                ),
    ))
}

fn reads_an_unbound_parameter() -> Result<Value, OperatingError> {
    let operation = Operation::Extend {
        from: FROM,
        values: vec![Operand::Parameter(ParameterId(0))],
    };
    compute(&operation, &[&numbers(&[&[1]])], &[])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("appends_columns"),
        word: Word::Succeeds,
        input: || appends_a_number_and_a_parameter,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_text_is_appended_as_a_name"),
        word: Word::Succeeds,
        input: || appends_a_text_and_a_text_parameter,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_name_made_after_the_files_were_read_compares_with_theirs"),
        word: Word::Succeeds,
        input: || appends_a_name_that_compares_with_the_names_of_the_files,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_unbound_parameter_is_refused"),
        word: Word::Refuses,
        input: || reads_an_unbound_parameter,
        expect: Expect::Error(OperatingError::Unbound),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    crate::domain::operator::collections::operators::fixtures::contract::check(
        QueryOperator::Extend,
    );
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::Table;

use ryo_values::domain::query_bindings::types::ParameterBinding;
