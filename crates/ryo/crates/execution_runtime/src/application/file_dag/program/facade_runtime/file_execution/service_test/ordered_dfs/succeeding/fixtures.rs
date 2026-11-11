//! The support of the `succeeding` cases.
use super::super::fixtures::{Observed, Ran, answered, compute, numbers, operation, work};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::domain::rows::service::push_row;

pub(super) fn numeric_order_and_diamond() -> Result<Observed, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(
            4,
            &[
                &[1, 2, 20, 12],
                &[2, 4, 0, 24],
                &[1, 3, -10, 13],
                &[3, 4, 0, 34],
            ],
        ),
        &mut scratch,
    )?;
    observe(out, &scratch)
}

pub(super) fn cycles_preserve_every_edge_visit() -> Result<Observed, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(
            4,
            &[
                &[1, 1, 0, 11],
                &[1, 2, 1, 12],
                &[2, 1, 0, 21],
                &[2, 2, 1, 22],
            ],
        ),
        &mut scratch,
    )?;
    observe(out, &scratch)
}

pub(super) fn parallel_distinct_orders_preserve_multiplicity() -> Result<Observed, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(4, &[&[1, 2, 1, 12], &[1, 2, 0, 12]]),
        &mut scratch,
    )?;
    observe(out, &scratch)
}

pub(super) fn roots_are_independent_and_duplicates_are_deduplicated()
-> Result<Observed, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[2], &[1], &[1]]),
        &numbers(4, &[&[2, 3, 0, 23], &[1, 2, 0, 12]]),
        &mut scratch,
    )?;
    observe(out, &scratch)
}

pub(super) fn partitions_and_multiple_keys_are_exact() -> Result<Observed, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[2, 1], &[5, 4]),
        &numbers(
            3,
            &[&[1, 10, 100], &[1, 20, 100], &[1, 10, 200], &[1, 10, 100]],
        ),
        &numbers(
            6,
            &[
                &[1, 2, 0, 12, 10, 100],
                &[2, 3, 0, 23, 20, 100],
                &[1, 4, 0, 14, 20, 100],
                &[1, 5, 0, 15, 10, 200],
            ],
        ),
        &mut scratch,
    )?;
    observe(out, &scratch)
}

pub(super) fn entry_node_encodes_initial_empty_class_seen() -> Result<Observed, OperatingError> {
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[10]]),
        &numbers(
            4,
            &[
                &[10, 2, 0, 102],
                &[2, 1, 0, 21],
                &[1, 3, 0, 13],
                &[10, 3, 1, 103],
            ],
        ),
        &mut scratch,
    )?;
    observe(out, &scratch)
}

pub(super) fn deep_chain_and_work_are_bounded() -> Result<Observed, OperatingError> {
    let n = 20_000;
    let mut edges = numbers(4, &[]);
    for i in 0..n {
        push_row(
            &mut edges,
            &[
                Cell::Number(Integer(i)),
                Cell::Number(Integer(i + 1)),
                Cell::Number(Integer(0)),
                Cell::Number(Integer(i)),
            ],
        );
    }
    let mut scratch = ExecutionObservation::default();
    let out = compute(
        &operation(&[], &[]),
        &numbers(1, &[&[0]]),
        &edges,
        &mut scratch,
    )?;
    observe(out, &scratch)
}

pub(super) fn many_interleaved_partitions_bound_expansions() -> Result<Observed, OperatingError> {
    let n = 1024;
    let mut roots = numbers(2, &[]);
    let mut edges = numbers(5, &[]);
    for partition in 0..n {
        push_row(
            &mut roots,
            &[Cell::Number(Integer(1)), Cell::Number(Integer(partition))],
        );
    }
    for [source, dest, order, id] in [[1, 2, 0, 12], [2, 4, 0, 24], [1, 3, 1, 13], [3, 4, 0, 34]] {
        for partition in 0..n {
            push_row(
                &mut edges,
                &[
                    Cell::Number(Integer(source)),
                    Cell::Number(Integer(dest)),
                    Cell::Number(Integer(order)),
                    Cell::Number(Integer(id)),
                    Cell::Number(Integer(partition)),
                ],
            );
        }
    }
    let mut scratch = ExecutionObservation::default();
    let out = compute(&operation(&[1], &[4]), &roots, &edges, &mut scratch)?;
    observe(out, &scratch)
}

/// The traversal's answer, work and retained bytes, as a row states them.
fn observe(out: Ran, scratch: &ExecutionObservation) -> Result<Observed, OperatingError> {
    Ok(Observed {
        answered: answered(&out.answer?),
        work: work(scratch),
        retained: out.retained,
    })
}
