//! Behavioral controls retained from the former application resource constructors.
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operator::contract::views::types::{
    ClosureParts, PartitionCount, PartitionIndex,
};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use ryo_relations::domain::relations::types::{CellKind, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;
use {
    ryo_execution_domain::domain::derived_views::types::*,
    ryo_execution_domain::domain::operating::errors::OperatingError,
    ryo_execution_domain::domain::operation_schema::types::InputShape,
    ryo_values::domain::rows::service::push_row,
};
fn prepare_cycles(
    input: RepresentationInput<'_>,
    source: ColumnIndex,
    destination: ColumnIndex,
) -> Result<ryo_execution_store::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView, OperatingError>{
    let owner = ryo_execution_store::application::file_dag::relations::service::RelationOwner::fixture(
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(ryo_execution_store::application::file_dag::fixtures::derived()),
    );
    let RepresentationInput::Computed(table) = input else {
        panic!("computed fixture")
    };
    let publication = owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Computed(table.clone()))?;
    let held = owner.rows(&publication, &|_| Err(OperatingError::NotRead))?;
    let input = RepresentationInput::from(&held);
    let proof = owner
        .admit(&publication, input)
        .and_then(|admitted| admitted)?;
    let plan = std::sync::Arc::new(owner.derived().prepare_view(
        &ViewRequest::CycleAdjacency {
            source,
            destination,
        },
        &[InputShape {
            kinds: input.kinds(),
            tree: None,
        }],
    )?);
    let admitted = owner.derived().admit_view(&plan, &owner, &[&proof])?;
    owner.derived().acquire_stored(
        &plan,
        &admitted,
        ryo_execution_store::application::file_dag::derived::owner::service::ViewContext::default(),
        DerivedInputs::Rows(&[proof.rows()]),
        &mut DerivedScratch::default(),
        None,
    )
}
fn prepare_closure(
    input: RepresentationInput<'_>,
    keys: &[ColumnIndex],
    source: ColumnIndex,
    destination: ColumnIndex,
) -> Result<crate::application::file_dag::invocation::types::AcquiredPartitions, OperatingError> {
    let owner = std::rc::Rc::new(ryo_execution_store::application::file_dag::relations::service::RelationOwner::fixture(
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(
            ryo_execution_store::application::file_dag::fixtures::derived(),
        ),
    ));
    let RepresentationInput::Computed(table) = input else {
        return Err(OperatingError::WrongOperation);
    };
    let publication = owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Computed(table.clone()))?;
    let rows = owner.rows(&publication, &|_| Err(OperatingError::NotRead))?;
    let input = RepresentationInput::from(&rows);
    let mut cursor =
        ryo_execution_store::application::file_dag::closure_partitions::service::prepare(
            &publication,
            input,
            keys,
            source,
            destination,
            owner,
        )?;
    let mut parts = crate::application::file_dag::invocation::types::AcquiredPartitions::default();
    while cursor.advance()? == ryo_execution_store::application::file_dag::closure_partitions::types::PartitionAdvance::Advanced {
        parts.push(cursor.take_current()?);
    }
    Ok(parts)
}
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::table_words::types::ColumnCount;
fn number(value: Integer) -> Cell {
    Cell::Number(value)
}
fn table(width: ColumnCount, rows: &[&[Integer]]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; width.0],
        ..Table::default()
    };
    for row in rows {
        push_row(
            &mut table,
            &row.iter().copied().map(number).collect::<Vec<_>>(),
        );
    }
    table
}
/// The graph resource a request prepares over a table's columns.
#[derive(Clone, Copy, Debug)]
enum Request {
    /// The cycle adjacency from the source to the destination column.
    Cycles { source: u32, destination: u32 },
    /// The closure partitions keyed by one column, from the source to the destination column.
    Closure {
        key: u32,
        source: u32,
        destination: u32,
    },
}

/// A request over number rows.
#[derive(Clone, Copy, Debug)]
struct Graph {
    width: usize,
    rows: &'static [&'static [i64]],
    request: Request,
}

/// One closure partition: its key, its sorted sources and its cells.
#[derive(Clone, Debug, PartialEq)]
struct Partition {
    key: Cow<'static, [i64]>,
    sources: Cow<'static, [i64]>,
    cells: Cow<'static, [i64]>,
}

/// What the prepared resource holds.
#[derive(Debug, PartialEq)]
enum Prepared {
    /// The adjacency: entity values in numbering order, each entity's first edge, each edge's target.
    Adjacency {
        values: Cow<'static, [i64]>,
        starts: Cow<'static, [usize]>,
        leaving: Cow<'static, [usize]>,
    },
    /// The closure partitions in order.
    Partitions(Cow<'static, [Partition]>),
}

type Input = Graph;
type Value = Prepared;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("adjacency_keeps_parallel_edges_and_first_entity_numbering"),
        word: Word::Succeeds,
        input: || Graph {
            width: 2,
            rows: &[&[9, 1], &[9, 1], &[1, 9], &[7, 7]],
            request: Request::Cycles {
                source: 0,
                destination: 1,
            },
        },
        expect: Expect::Value(Prepared::Adjacency {
            values: Cow::Borrowed(&[9, 1, 7]),
            starts: Cow::Borrowed(&[0, 2, 3, 4]),
            leaving: Cow::Borrowed(&[1, 1, 0, 2]),
        }),
    },
    Case {
        name: CaseName("closure_preparation_preserves_clustered_partition_order_and_sorts_sources"),
        word: Word::Succeeds,
        input: || Graph {
            width: 3,
            rows: &[&[9, 4, 3], &[9, 2, 4], &[1, 7, 8]],
            request: Request::Closure {
                key: 0,
                source: 1,
                destination: 2,
            },
        },
        expect: Expect::Value(Prepared::Partitions(Cow::Borrowed(&[
            Partition {
                key: Cow::Borrowed(&[9]),
                sources: Cow::Borrowed(&[2, 4]),
                cells: Cow::Borrowed(&[4, 3, 2, 4]),
            },
            Partition {
                key: Cow::Borrowed(&[1]),
                sources: Cow::Borrowed(&[7]),
                cells: Cow::Borrowed(&[7, 8]),
            },
        ]))),
    },
];

fn integers(cells: &[Cell]) -> Cow<'static, [i64]> {
    cells
        .iter()
        .map(|cell| {
            let Cell::Number(Integer(value)) = cell else {
                panic!("a number cell, not {cell:?}");
            };
            *value
        })
        .collect()
}

/// Prepares the requested graph resource over the rows.
fn subject(graph: Input) -> Result<Value, Failure> {
    let rows: Vec<Vec<Integer>> = graph
        .rows
        .iter()
        .map(|row| row.iter().copied().map(Integer).collect())
        .collect();
    let refs: Vec<&[Integer]> = rows.iter().map(Vec::as_slice).collect();
    let input = table(ColumnCount(graph.width), &refs);
    match graph.request {
        Request::Cycles {
            source,
            destination,
        } => {
            let acquired = prepare_cycles(
                RepresentationInput::Computed(&input),
                ColumnIndex(source),
                ColumnIndex(destination),
            )?;
            let ryo_derived::domain::contract::types::DerivedView::CycleAdjacency(adjacency) =
                acquired.view()?
            else {
                return Err(OperatingError::WrongOperation);
            };
            Ok(Prepared::Adjacency {
                values: integers(&adjacency.values),
                starts: adjacency.starts.0.clone().into(),
                leaving: adjacency.leaving.0.clone().into(),
            })
        }
        Request::Closure {
            key,
            source,
            destination,
        } => {
            let parts = prepare_closure(
                RepresentationInput::Computed(&input),
                &[ColumnIndex(key)],
                ColumnIndex(source),
                ColumnIndex(destination),
            )?;
            let PartitionCount(count) = ClosureParts::len(&parts);
            let mut partitions = Vec::new();
            for index in 0..count {
                let part = ClosureParts::get(&parts, PartitionIndex(index))?;
                partitions.push(Partition {
                    key: integers(&part.key),
                    sources: integers(&part.sources),
                    cells: integers(&part.table.cells),
                });
            }
            Ok(Prepared::Partitions(partitions.into()))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
