//! The `growth` service: the fan a join's proven keys answer, and the one algebra that composes a plan's declarations into
//! bounds.
use super::types::{Bound, Degree, Fan, Growth, NodeBound};
use ryo_values::domain::plan_parts::types::KeyFan;

/// The fan a join's proven keys answer for each left row: the one mapping a probe and a band join read.
#[must_use]
pub(crate) const fn fan_of(key: KeyFan) -> Fan {
    match key {
        KeyFan::Unique => Fan::PerLeft,
        KeyFan::Shared => Fan::Repeats,
        KeyFan::FileOnly => Fan::FileProduct,
        KeyFan::AllPairs => Fan::DeclaredPairs,
    }
}

/// `rows^1`: the bound of a relation's rows.
pub(crate) const ROWS: Bound = Bound {
    rows: Degree(1),
    depth: Degree(0),
    repeats: Degree(0),
};

/// The larger of two bounds, degree by degree.
fn larger(a: Bound, b: Bound) -> Bound {
    Bound {
        rows: a.rows.max(b.rows),
        depth: a.depth.max(b.depth),
        repeats: a.repeats.max(b.repeats),
    }
}

/// The product of two bounds: their degrees added.
fn times(a: Bound, b: Bound) -> Bound {
    Bound {
        rows: Degree(a.rows.0.saturating_add(b.rows.0)),
        depth: Degree(a.depth.0.saturating_add(b.depth.0)),
        repeats: Degree(a.repeats.0.saturating_add(b.repeats.0)),
    }
}

/// The bound of a node of `growth` over inputs bounded by `inputs`, in the order it reads them; a node that reads no node (a
/// scan, a lookup) reads the partition's rows. The one algebra: work is the inputs and the output together, the log of a sort not
/// counted.
pub(crate) fn composed(growth: Growth, inputs: &[Bound]) -> NodeBound {
    let first = inputs.first().copied().unwrap_or(ROWS);
    let second = inputs.get(1).copied().unwrap_or(first);
    let read = inputs.iter().copied().fold(first, larger);
    let output = match growth {
        Growth::AtMostInput => read,
        Growth::PerRowText => ROWS,
        Growth::OnePerLeftRow | Growth::GroupOutput => first,
        Growth::Keyed(fan) | Growth::Banded(fan) => match fan {
            Fan::PerLeft => first,
            Fan::Repeats => times(
                first,
                Bound {
                    repeats: Degree(1),
                    ..Bound::default()
                },
            ),
            Fan::FileProduct | Fan::DeclaredPairs => times(first, second),
        },
        Growth::PerStartWalk => times(
            first,
            Bound {
                depth: Degree(1),
                ..Bound::default()
            },
        ),
    };
    NodeBound {
        output,
        work: larger(read, output),
    }
}
