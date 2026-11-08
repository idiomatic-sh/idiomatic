//! Closure outcomes through the mandatory public execution owner.
use super::super::super::fixtures;
use ryo_values::selects;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_language::domain::symbols::types::Symbols;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::query_bindings::types::ParameterBinding;

type Value = usize;
type Failure = OperatingError;
/// A scenario: what a row computes with the item, answering how many rows came out.
type Input = fn() -> Result<Value, OperatingError>;
const FROM: Source = Source::Relation(Relation::Function);
fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}
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
    Ok(table(operation, inputs, bindings)?.count.0)
}
/// The caller receives the completed owner's answer; it never supplies workspace or a permit.
fn table(
    operation: &Operation,
    inputs: &[&Table],
    bindings: &[ParameterBinding],
) -> Result<Table, OperatingError> {
    fixtures::requests::table_request(
        operation.clone(),
        inputs,
        bindings.to_vec(),
        &mut Symbols::default(),
    )
}
fn reaches_across_a_chain() -> Result<Value, OperatingError> {
    let operation = Operation::Closure {
        edges: FROM,
        keys: Vec::new(),
        source: column(0),
        destination: column(1),
    };
    compute(&operation, &[&numbers(&[&[1, 2], &[2, 3]])], &[])
}
fn closure(edges: &Table) -> Result<Table, OperatingError> {
    table(
        &Operation::Closure {
            edges: FROM,
            keys: Vec::new(),
            source: column(0),
            destination: column(1),
        },
        &[edges],
        &[],
    )
}
fn assert_pairs(out: &Table, expected: &[[i64; 2]]) {
    let mut actual: Vec<_> = out
        .cells
        .chunks_exact(2)
        .map(|row| {
            [
                *row.first().expect("source column"),
                *row.get(1).expect("destination column"),
            ]
        })
        .collect();
    actual.sort_unstable();
    let mut expected: Vec<_> = expected
        .iter()
        .map(|pair| pair.map(|n| Cell::Number(Integer(n))))
        .collect();
    expected.sort_unstable();
    assert_eq!(out.kinds, [CellKind::Number; 2]);
    assert_eq!(out.count.0, expected.len());
    assert_eq!(actual, expected);
}
fn interleaved_sources() -> Result<Value, OperatingError> {
    let out = closure(
        &numbers(&[&[1, 2], &[2, 3], &[1, 4]]),
    )?;
    assert_pairs(&out, &[[1, 2], [1, 3], [1, 4], [2, 3]]);
    Ok(out.count.0)
}
fn source_permutations() -> Result<Value, OperatingError> {
    let edges = [[1, 2], [2, 3], [1, 4]];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let rows: Vec<&[i64]> = order.iter().map(|i| edges[*i].as_slice()).collect();
        let out = closure(&numbers(&rows))?;
        assert_pairs(&out, &[[1, 2], [1, 3], [1, 4], [2, 3]]);
    }
    Ok(6)
}
fn repeated_edges() -> Result<Value, OperatingError> {
    for rows in [
        [&[1, 2][..], &[1, 2], &[2, 3], &[1, 4]],
        [&[1, 2][..], &[2, 3], &[1, 4], &[1, 2]],
    ] {
        let out = closure(&numbers(&rows))?;
        assert_pairs(&out, &[[1, 2], [1, 3], [1, 4], [2, 3]]);
    }
    Ok(4)
}
fn cycles_and_self_edges() -> Result<Value, OperatingError> {
    let out = closure(
        &numbers(&[&[1, 2], &[2, 1], &[1, 2], &[1, 1]]),
    )?;
    assert_pairs(&out, &[[1, 1], [1, 2], [2, 1], [2, 2]]);
    Ok(out.count.0)
}
fn disconnected_sources() -> Result<Value, OperatingError> {
    let out = closure(
        &numbers(&[&[1, 2], &[3, 4], &[1, 5]]),
    )?;
    assert_pairs(&out, &[[1, 2], [1, 5], [3, 4]]);
    Ok(out.count.0)
}
fn successive_actual_owners_do_not_carry_prior_rows() -> Result<Value, OperatingError> {
    let initial = closure(&numbers(&[&[1, 2], &[2, 3], &[1, 4]]))?;
    assert_pairs(&initial, &[[1, 2], [1, 3], [1, 4], [2, 3]]);
    let later = closure(&numbers(&[&[5, 6]]))?;
    assert_pairs(&later, &[[5, 6]]);
    let empty = Table {
        kinds: vec![CellKind::Number; 2],
        ..Table::default()
    };
    let cleared = closure(&empty)?;
    assert_pairs(&cleared, &[]);
    Ok(cleared.count.0)
}
fn interleaved_work_and_output_are_bounded() -> Result<Value, OperatingError> {
    let n = 1024;
    let mut edges = Table {
        kinds: vec![CellKind::Number; 2],
        ..Table::default()
    };
    for i in 0..n {
        let middle = Cell::Number(Integer(2 * i + 1));
        push_row(&mut edges, &[Cell::Number(Integer(0)), middle]);
        push_row(&mut edges, &[middle, Cell::Number(Integer(2 * i + 2))]);
    }
    let out = closure(&edges)?;
    let mut sources = out
        .cells
        .chunks_exact(2)
        .map(|row| row[0])
        .collect::<Vec<_>>();
    sources.sort_unstable();
    sources.dedup();
    assert_eq!(sources.len(), 1025);
    assert_eq!(out.count.0, 3072);
    assert_eq!(out.cells.len(), 6144);
    let mut pairs: Vec<_> = out.cells.chunks_exact(2).collect();
    pairs.sort_unstable();
    pairs.dedup();
    assert_eq!(pairs.len(), out.count.0);
    Ok(out.count.0)
}
fn closes_over_a_missing_column() -> Result<Value, OperatingError> {
    let operation = Operation::Closure {
        edges: FROM,
        keys: Vec::new(),
        source: column(0),
        destination: column(5),
    };
    compute(&operation, &[&numbers(&[&[1, 2]])], &[])
}
/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_reachable_pair_is_listed"),
        word: Word::Succeeds,
        input: || reaches_across_a_chain,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("interleaved_sources_emit_each_pair_once"),
        word: Word::Succeeds,
        input: || interleaved_sources,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("source_permutations_preserve_pairs"),
        word: Word::Succeeds,
        input: || source_permutations,
        expect: Expect::Value(6),
    },
    Case {
        name: CaseName("duplicate_edges_preserve_pairs"),
        word: Word::Succeeds,
        input: || repeated_edges,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("cycles_include_only_reachable_reflexive_pairs"),
        word: Word::Succeeds,
        input: || cycles_and_self_edges,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("disconnected_acyclic_sources_have_no_reflexive_pairs"),
        word: Word::Succeeds,
        input: || disconnected_sources,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("empty_actual_owner_keeps_its_output_empty"),
        word: Word::Succeeds,
        input: || successive_actual_owners_do_not_carry_prior_rows,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("interleaved_work_and_output_are_bounded"),
        word: Word::Succeeds,
        input: || interleaved_work_and_output_are_bounded,
        expect: Expect::Value(3072),
    },
    Case {
        name: CaseName("a_missing_column_is_refused"),
        word: Word::Refuses,
        input: || closes_over_a_missing_column,
        expect: Expect::Refusal(|error| selects!(matches!(error, OperatingError::Table(..)))),
    },];
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
