//! Closure outcomes through the mandatory public execution owner.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;

/// What a closure is asked over: one or more successive edge lists, each its own owner request; a fan of `n` two-step
/// chains out of node 0; or edges whose destination column is missing.
#[derive(Clone, Copy, Debug)]
enum Closure {
    Runs(&'static [&'static [[i64; 2]]]),
    Fan(i64),
    MissingColumn,
}
/// One closure's answer: its schema, its numeric pairs sorted, and any row that is not two numbers.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Pairs {
    kinds: Cow<'static, [CellKind]>,
    pairs: Cow<'static, [[i64; 2]]>,
    other_rows: usize,
}
#[derive(Debug, PartialEq, Eq)]
enum Observed {
    /// Each successive run's pairs.
    Runs(Cow<'static, [Pairs]>),
    /// The fan's distinct sources, rows, cells and distinct pairs.
    Fan {
        sources: usize,
        rows: usize,
        cells: usize,
        distinct: usize,
    },
}
type Input = Closure;
type Value = Observed;
type Failure = OperatingError;
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
/// The closure of `edges` from its `destination` column, through the completed owner: the caller never supplies
/// workspace or a permit.
fn closure(edges: &Table, destination: u32) -> Result<Table, OperatingError> {
    fixtures::requests::table_request(
        Operation::Closure {
            edges: Source::Relation(Relation::Function),
            keys: Vec::new(),
            source: column(0),
            destination: column(destination),
        },
        &[edges],
        Vec::<ParameterBinding>::new(),
        &mut Symbols::default(),
    )
}
fn pairs(out: Table) -> Pairs {
    let mut pairs: Vec<_> = out
        .cells
        .chunks_exact(2)
        .filter_map(|row| match row {
            [Cell::Number(source), Cell::Number(destination)] => Some([source.0, destination.0]),
            _ => None,
        })
        .collect();
    pairs.sort_unstable();
    Pairs {
        other_rows: out.count.0 - pairs.len(),
        kinds: out.kinds.into(),
        pairs: pairs.into(),
    }
}
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Closure::Runs(runs) => runs
            .iter()
            .map(|edges| {
                let rows: Vec<&[i64]> = edges.iter().map(<[i64; 2]>::as_slice).collect();
                let mut table = numbers(&rows);
                table.kinds = vec![CellKind::Number; 2];
                closure(&table, 1).map(pairs)
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|runs| Observed::Runs(runs.into())),
        Closure::Fan(n) => {
            let mut edges = Table {
                kinds: vec![CellKind::Number; 2],
                ..Table::default()
            };
            for i in 0..n {
                let middle = Cell::Number(Integer(2 * i + 1));
                push_row(&mut edges, &[Cell::Number(Integer(0)), middle]);
                push_row(&mut edges, &[middle, Cell::Number(Integer(2 * i + 2))]);
            }
            let out = closure(&edges, 1)?;
            let mut sources = out
                .cells
                .chunks_exact(2)
                .map(|row| row[0])
                .collect::<Vec<_>>();
            sources.sort_unstable();
            sources.dedup();
            let mut distinct: Vec<_> = out.cells.chunks_exact(2).collect();
            distinct.sort_unstable();
            distinct.dedup();
            Ok(Observed::Fan {
                sources: sources.len(),
                rows: out.count.0,
                cells: out.cells.len(),
                distinct: distinct.len(),
            })
        }
        Closure::MissingColumn => {
            closure(&numbers(&[&[1, 2]]), 5).map(|out| Observed::Runs(vec![pairs(out)].into()))
        }
    }
}
const NUMBERS: &[CellKind] = &[CellKind::Number; 2];
const fn run_of(pairs: &'static [[i64; 2]]) -> Pairs {
    Pairs {
        kinds: Cow::Borrowed(NUMBERS),
        pairs: Cow::Borrowed(pairs),
        other_rows: 0,
    }
}
const fn runs(runs: &'static [Pairs]) -> Expect<Value, Failure> {
    Expect::Value(Observed::Runs(Cow::Borrowed(runs)))
}
const CHAIN: &[Pairs] = &[run_of(&[[1, 2], [1, 3], [2, 3]])];
const REACHED: &[Pairs] = &[run_of(&[[1, 2], [1, 3], [1, 4], [2, 3]])];
const CYCLE: &[Pairs] = &[run_of(&[[1, 1], [1, 2], [2, 1], [2, 2]])];
const DISCONNECTED: &[Pairs] = &[run_of(&[[1, 2], [1, 5], [3, 4]])];
const SUCCESSIVE: &[Pairs] = &[
    run_of(&[[1, 2], [1, 3], [1, 4], [2, 3]]),
    run_of(&[[5, 6]]),
    run_of(&[]),
];
/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_reachable_pair_is_listed"),
        word: Word::Succeeds,
        input: || Closure::Runs(&[&[[1, 2], [2, 3]]]),
        expect: runs(CHAIN),
    },
    Case {
        name: CaseName("interleaved_sources_emit_each_pair_once"),
        word: Word::Succeeds,
        input: || Closure::Runs(&[&[[1, 2], [2, 3], [1, 4]]]),
        expect: runs(REACHED),
    },
    Case {
        name: CaseName("source_permutations_preserve_pairs"),
        word: Word::Succeeds,
        input: || Closure::Runs(&[&[[1, 2], [2, 3], [1, 4]]]),
        expect: runs(REACHED),
    },
    Case {
        name: CaseName("source_permutation_021_preserves_pairs"),
        word: Word::Succeeds,
        input: || Closure::Runs(&[&[[1, 2], [1, 4], [2, 3]]]),
        expect: runs(REACHED),
    },
    Case {
        name: CaseName("source_permutation_102_preserves_pairs"),
        word: Word::Succeeds,
        input: || Closure::Runs(&[&[[2, 3], [1, 2], [1, 4]]]),
        expect: runs(REACHED),
    },
    Case {
        name: CaseName("source_permutation_120_preserves_pairs"),
        word: Word::Succeeds,
        input: || Closure::Runs(&[&[[2, 3], [1, 4], [1, 2]]]),
        expect: runs(REACHED),
    },
    Case {
        name: CaseName("source_permutation_201_preserves_pairs"),
        word: Word::Succeeds,
        input: || Closure::Runs(&[&[[1, 4], [1, 2], [2, 3]]]),
        expect: runs(REACHED),
    },
    Case {
        name: CaseName("source_permutation_210_preserves_pairs"),
        word: Word::Succeeds,
        input: || Closure::Runs(&[&[[1, 4], [2, 3], [1, 2]]]),
        expect: runs(REACHED),
    },
    Case {
        name: CaseName("duplicate_edges_preserve_pairs"),
        word: Word::Succeeds,
        input: || Closure::Runs(&[&[[1, 2], [1, 2], [2, 3], [1, 4]]]),
        expect: runs(REACHED),
    },
    Case {
        name: CaseName("a_trailing_duplicate_edge_preserves_pairs"),
        word: Word::Succeeds,
        input: || Closure::Runs(&[&[[1, 2], [2, 3], [1, 4], [1, 2]]]),
        expect: runs(REACHED),
    },
    Case {
        name: CaseName("cycles_include_only_reachable_reflexive_pairs"),
        word: Word::Succeeds,
        input: || Closure::Runs(&[&[[1, 2], [2, 1], [1, 2], [1, 1]]]),
        expect: runs(CYCLE),
    },
    Case {
        name: CaseName("disconnected_acyclic_sources_have_no_reflexive_pairs"),
        word: Word::Succeeds,
        input: || Closure::Runs(&[&[[1, 2], [3, 4], [1, 5]]]),
        expect: runs(DISCONNECTED),
    },
    Case {
        name: CaseName("empty_actual_owner_keeps_its_output_empty"),
        word: Word::Succeeds,
        input: || Closure::Runs(&[&[[1, 2], [2, 3], [1, 4]], &[[5, 6]], &[]]),
        expect: runs(SUCCESSIVE),
    },
    Case {
        name: CaseName("interleaved_work_and_output_are_bounded"),
        word: Word::Succeeds,
        input: || Closure::Fan(1024),
        expect: Expect::Value(Observed::Fan {
            sources: 1025,
            rows: 3072,
            cells: 6144,
            distinct: 3072,
        }),
    },
    Case {
        name: CaseName("a_missing_column_is_refused"),
        word: Word::Refuses,
        input: || Closure::MissingColumn,
        expect: Expect::Error(OperatingError::Table(RowsError::NoSuchColumn { index: 5 })),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
