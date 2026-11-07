//! Literal selection caches through the mandatory public execution owner.
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use super::super::super::fixtures as public_owner;
use crate::application::file_dag::program::file_session::types::SourceRows;
use crate::domain::operating::errors::OperatingError;
use crate::domain::stored_selection::fixtures::expected_columns;
use crate::domain::stored_selection::fixtures::program;
use crate::domain::stored_selection::types::Selector;
use crate::domain::table_model::service::materialize;
use crate::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::to_bits;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::ColumnsView;
use ryo_relations::domain::relations::types::FactColumn;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;

type Failure = OperatingError;
/// Generic cache policy data is projected from a genuine provider schema for
/// the independent public-owner oracle. No catalog reader receives a synthetic schema.
fn baseline(source: ColumnsView<'_>, program: &Selector) -> Result<Table, Failure> {
    baseline_with_symbols(source, program, &mut Symbols::default())
}
fn baseline_with_symbols(
    source: ColumnsView<'_>,
    program: &Selector,
    symbols: &mut Symbols,
) -> Result<Table, Failure> {
    use ryo_relations::domain::relations::{
        service::{kinds, node_id},
        types::{FileId, Flag, Offset, Position, Span},
    };
    use ryo_values::domain::{
        plan_parts::types::{ParameterName, StepId},
        query_bindings::types::{ParameterBinding, ParameterType, PlanParameter},
    };
    let mut logical = Table::default();
    materialize(&Rows::Stored(source), &mut logical, &mut Vec::new());
    let select = Operation::Select {
        from: Source::Step(StepId(0)),
        comparisons: program.identity.comparisons.clone(),
        kept: if program.distinct {
            Vec::new()
        } else {
            program.columns.clone()
        },
    };
    let (mut plan, hosted, facts) = if logical.kinds.iter().all(|kind| *kind == CellKind::Number) {
        let authored =
            inputs::table_inputs(select, &[&logical], &program.identity.bindings, symbols);
        (authored.plan, authored.hosted, authored.facts)
    } else {
        let (relation, projection) = match logical.kinds.as_slice() {
            [CellKind::Symbol] => (Relation::Defines, vec![1]),
            [CellKind::Symbol, CellKind::Number] => (Relation::BraceFormatField, vec![2, 1]),
            [CellKind::Number, CellKind::Flag, CellKind::Symbol] => {
                (Relation::FormatParse, vec![6, 8, 1])
            }
            _ => panic!("explicit cache-policy fixture projection required"),
        };
        for (&column, kind) in projection.iter().zip(&logical.kinds) {
            assert_eq!(kinds(relation)[column as usize], *kind);
        }
        let mut canonical = Table {
            kinds: kinds(relation).to_vec(),
            ..Table::default()
        };
        for row in logical.cells.chunks(logical.kinds.len()) {
            let mut cells = kinds(relation)
                .iter()
                .map(|kind| match kind {
                    CellKind::Node => Cell::Node(node_id(FileId(0), Position(0))),
                    CellKind::File => Cell::File(FileId(0)),
                    CellKind::Symbol => Cell::Symbol(Symbol(0)),
                    CellKind::Number => Cell::Number(Integer(0)),
                    CellKind::Flag => Cell::Flag(Flag::Lowered),
                    CellKind::Span => Cell::Span(Span {
                        start: Offset(0),
                        end: Offset(0),
                    }),
                })
                .collect::<Vec<_>>();
            for (&column, &value) in projection.iter().zip(row) {
                cells[column as usize] = value;
            }
            canonical.cells.extend(cells);
            canonical.count.0 += 1;
        }
        let plan = inputs::plan(vec![
            Operation::Select {
                from: Source::Relation(relation),
                comparisons: vec![],
                kept: projection.into_iter().map(ColumnIndex).collect(),
            },
            select,
        ]);
        (plan, vec![], vec![(relation, canonical)])
    };
    // Select keeps every input column when kept is empty; a zero-column
    // authored result requires the actual Project operation.
    if program.distinct || program.columns.is_empty() {
        plan.steps.push(ryo_values::domain::plans::types::PlanStep {
            scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
            operation: Operation::Project {
                from: Source::Step(plan.emit.step),
                columns: program.columns.clone(),
                distinct: if program.distinct {
                    RepeatedRows::Dropped
                } else {
                    RepeatedRows::Kept
                },
            },
        });
        plan.emit.step = StepId((plan.steps.len() - 1) as u32);
    }
    plan.parameters = program
        .identity
        .bindings
        .iter()
        .enumerate()
        .map(|(at, binding)| PlanParameter {
            name: ParameterName(format!("parameter-{at}").into()),
            kind: match binding {
                ParameterBinding::Number(_) => ParameterType::Number,
                ParameterBinding::Flag(_) => ParameterType::Flag,
                ParameterBinding::Text(_) => ParameterType::Text,
                ParameterBinding::Texts(_) => ParameterType::Texts,
                ParameterBinding::Numbers(_) => ParameterType::Numbers,
                ParameterBinding::TextSets(_) => ParameterType::TextSets,
                ParameterBinding::Tables(_) => ParameterType::Tables,
            },
        })
        .collect();
    let read = |relation| {
        facts
            .iter()
            .find(|(actual, _)| *actual == relation)
            .map(|(_, table)| SourceRows::Computed(table))
            .ok_or(OperatingError::NotRead)
    };
    public_owner::published_from(
        plan,
        program.identity.bindings.clone(),
        hosted,
        b"",
        &ryo_language::domain::elements::types::Elements::default(),
        symbols,
        &read,
    )
}
fn literal(text: &str, columns: Vec<ColumnIndex>) -> Selector {
    use ryo_values::domain::{
        plan_parts::types::{LiteralText, Operand, PlanComparison, QueryLiteral, Test},
        query_bindings::types::ParameterBinding,
    };
    Selector {
        relation: Relation::Defines,
        identity: Arc::new(crate::domain::stored_selection::types::SelectionIdentity {
            comparisons: vec![PlanComparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            }],
            bindings: Vec::<ParameterBinding>::new(),
        }),
        columns,
        distinct: false,
    }
}
type Input = fn() -> Result<Value, Failure>;
type Value = ();
fn literal_fanout_routes_aliases_once_and_refuses_incomplete_inputs() -> Result<(), Failure> {
    use ryo_language::domain::symbols::{service::intern_derived, types::ByteSlice};
    use ryo_relations::domain::terms::service::term_symbol;
    use ryo_relations::domain::declaration_kind::types::DeclarationKind;
    use ryo_relations::domain::terms::types::{NameText, Symbol};
    let mut symbols = Symbols::default();
    let dynamic = intern_derived(&mut symbols, ByteSlice(b""), NameText("class"));
    let extras = (0..7)
        .map(|number| {
            intern_derived(
                &mut symbols,
                ByteSlice(b""),
                NameText(&format!("word{number}")),
            )
        })
        .collect::<Vec<_>>();
    let mut bits = vec![to_bits(Cell::Symbol(term_symbol(DeclarationKind::Method))).0; 128];
    for row in [1, 4, 8] {
        bits[row] = to_bits(Cell::Symbol(term_symbol(DeclarationKind::Class))).0;
    }
    bits[5] = to_bits(Cell::Symbol(dynamic)).0;
    bits[127] = to_bits(Cell::Symbol(Symbol(u32::MAX))).0;
    for (number, &symbol) in extras.iter().enumerate() {
        bits[20 + number * 2] = to_bits(Cell::Symbol(symbol)).0;
    }
    let columns = [FactColumn(bits)];
    let source = ColumnsView {
        kinds: &[CellKind::Symbol],
        columns: &columns,
    };
    let mut words = vec![("class".to_owned(), vec![1, 4, 5, 8])];
    words.extend((0..7).map(|number| (format!("word{number}"), vec![20 + number * 2])));
    for (word, positions) in words {
        let selector = literal(&word, vec![ColumnIndex(0), ColumnIndex(0)]);
        assert_eq!(
            baseline_with_symbols(source, &selector, &mut symbols)?,
            expected_columns(source, &selector.columns, &positions)
        );
    }
    Ok(())
}
fn empty_literal_preserves_malformed_symbol_fallback() -> Result<(), Failure> {
    use ryo_relations::domain::terms::types::Symbol;
    let columns = [FactColumn(vec![
        to_bits(Cell::Symbol(Symbol(u32::MAX))).0;
        3
    ])];
    let source = ColumnsView {
        kinds: &[CellKind::Symbol],
        columns: &columns,
    };
    let selector = literal("", vec![ColumnIndex(0), ColumnIndex(0)]);
    assert_eq!(
        baseline_with_symbols(source, &selector, &mut Symbols::default())?,
        expected_columns(source, &selector.columns, &[0, 1, 2])
    );
    Ok(())
}
fn distinct_projections_preserve_rows_and_predicate_cache() -> Result<(), Failure> {
    let columns = [
        FactColumn(vec![7, 7, 7, 7, 7, 8]),
        FactColumn(vec![1, 2, 0, 7, 0, 1]),
        FactColumn(vec![10, 10, 20, 11, 20, 10]),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Flag, CellKind::Symbol],
        columns: &columns,
    };
    for (columns, positions) in [
        (
            vec![ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)],
            vec![0, 2, 3],
        ),
        (vec![ColumnIndex(1)], vec![0, 2]),
        (vec![], vec![0]),
    ] {
        let mut selector = (*program(7)).clone();
        selector.columns = columns;
        selector.distinct = true;
        assert_eq!(
            baseline(source, &selector)?,
            expected_columns(source, &selector.columns, &positions)
        );
    }
    let changed = [
        FactColumn(vec![7]),
        FactColumn(vec![0]),
        FactColumn(vec![20]),
    ];
    let changed = ColumnsView {
        columns: &changed,
        ..source
    };
    let mut selector = (*program(7)).clone();
    selector.columns = vec![ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)];
    selector.distinct = true;
    assert_eq!(
        baseline(changed, &selector)?,
        expected_columns(changed, &selector.columns, &[0])
    );
    Ok(())
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(literal_fanout_routes_aliases_once_and_refuses_incomplete_inputs)),
        word: Word::Succeeds,
        input: || literal_fanout_routes_aliases_once_and_refuses_incomplete_inputs,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(empty_literal_preserves_malformed_symbol_fallback)),
        word: Word::Succeeds,
        input: || empty_literal_preserves_malformed_symbol_fallback,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(distinct_projections_preserve_rows_and_predicate_cache)),
        word: Word::Succeeds,
        input: || distinct_projections_preserve_rows_and_predicate_cache,
        expect: Expect::Value(()),
    },];
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
