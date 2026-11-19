//! Spike (ryo-fmx3): the `pointer_chase` item of the `kernels` collection: a chain of `steps` inner self probes, each of the
//! rows the one before made with themselves on one key, run as one kernel. Its rows are the chain's last probe's, in its
//! order, with its repeats.
//!
//! When each step follows a pointer (the left key is the partition columns and a `to` column, the right key the same
//! partition columns and a `from` column, the output is the left row with `to` taken from the right row) and no two rows share
//! a partition and `from`, every step keeps each left row at most once, in order: after `steps` steps a row is kept with
//! `to` replaced by the `to` of the row `2^steps - 1` pointer steps on, when every step on the way has a row. That is
//! answered by one walk of the pointer graph (each row's successor found once), its cycles numbered. Any other rows run the
//! probes one after the other.

use super::super::scan_facts::item::{count_of, kinds_of, row_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::InputRows;
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, RepeatedRows, Side};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// Spike (ryo-fmx3): the chain a pointer chase runs: the probe every step runs, and how many steps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChaseParts {
    /// The left key columns of each step.
    pub(crate) left_on: Vec<ColumnIndex>,
    /// The right key columns of each step.
    pub(crate) right_on: Vec<ColumnIndex>,
    /// The output columns of each step.
    pub(crate) output: Vec<JoinColumn>,
    /// Whether a step drops a repeated row.
    pub(crate) repeated: RepeatedRows,
    /// How many steps.
    pub(crate) steps: u32,
}

impl ChaseParts {
    /// The partition width and the `to` and `from` columns when each step follows a pointer.
    fn pointer(&self) -> Option<(usize, usize, usize)> {
        let width = self.left_on.len().checked_sub(1)?;
        if self.right_on.len() != width + 1 || self.left_on[..width] != self.right_on[..width] || self.steps >= 63 {
            return None;
        }
        let to = usize::from(self.left_on[width]);
        let from = usize::from(self.right_on[width]);
        let partition = &self.left_on[..width];
        // The partition is the first columns, in order.
        if to == from || to < width || from < width || partition.iter().enumerate().any(|(at, column)| usize::from(*column) != at) {
            return None;
        }
        let identity = self.output.iter().enumerate().all(|(at, column)| {
            usize::from(column.column) == at && (column.side == Side::Right) == (at == to)
        });
        (identity && to < self.output.len() && from < self.output.len()).then_some((width, to, from))
    }
}

/// Writes into `output` the rows of the chain over `input`.
///
/// # Errors
///
/// `NoSuchColumn` for a key or output column past its rows.
pub(crate) fn chased(
    chase: &ChaseParts,
    input: InputRows<'_>,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let kinds = kinds_of(input);
    let count = count_of(input).0;
    let mut cells: Vec<Value> = Vec::with_capacity(count * kinds.len());
    let mut row = Vec::new();
    for at in 0..count {
        let read = row_of(input, RowIndex(at), &mut row);
        if read.len() != kinds.len() {
            return Err(PortableOperatorFault::ShapeMismatch);
        }
        cells.extend_from_slice(read);
    }
    let mut table = Table {
        kinds: kinds.to_vec(),
        cells,
        count: RowCount(count),
    };
    if let Some((width, to, from)) = chase.pointer()
        && table.kinds.len() == chase.output.len()
        && (count as u64) < (1_u64 << chase.steps)
        && walked(&table, width, to, from, chase.steps, output)
    {
        return Ok(None);
    }
    for _ in 0..chase.steps {
        table = stepped(chase, &table)?;
    }
    *output = table;
    Ok(None)
}

/// The fast path: answers false, writing nothing, when two rows share a partition and `from`.
fn walked(table: &Table, width: usize, to: usize, from: usize, steps: u32, output: &mut Table) -> bool {
    let columns = table.kinds.len();
    let count = table.count.0;
    let row = |at: usize| &table.cells[at * columns..(at + 1) * columns];
    let mut by_key: hashbrown::HashMap<(&[Value], Value), u32> = hashbrown::HashMap::with_capacity(count);
    for at in 0..count {
        let cells = row(at);
        if by_key.insert((&cells[..width], cells[from]), at as u32).is_some() {
            return false;
        }
    }
    const NONE: u32 = u32::MAX;
    let next: Vec<u32> = (0..count)
        .map(|at| {
            let cells = row(at);
            by_key.get(&(&cells[..width], cells[to])).copied().unwrap_or(NONE)
        })
        .collect();
    // Each row: the row `2^steps - 1` successor steps on, or none.
    let far = (1_u64 << steps) - 1;
    #[derive(Clone, Copy)]
    enum State {
        Unknown,
        OnPath(u32),
        Dead,
        // The cycle's first place in `cycles`, its length, the place its walk enters it, and the steps before it does.
        Cycle { base: u32, length: u32, entry: u32, before: u64 },
    }
    let mut state = vec![State::Unknown; count];
    let mut cycles: Vec<u32> = Vec::new();
    let mut path: Vec<u32> = Vec::new();
    for start in 0..count {
        if !matches!(state[start], State::Unknown) {
            continue;
        }
        path.clear();
        let mut at = start as u32;
        loop {
            if at == NONE {
                for place in &path {
                    state[*place as usize] = State::Dead;
                }
                break;
            }
            match state[at as usize] {
                State::Unknown => {
                    state[at as usize] = State::OnPath(path.len() as u32);
                    path.push(at);
                    at = next[at as usize];
                }
                State::Dead => {
                    for place in &path {
                        state[*place as usize] = State::Dead;
                    }
                    break;
                }
                State::OnPath(first) => {
                    let base = cycles.len() as u32;
                    let length = path.len() as u32 - first;
                    cycles.extend_from_slice(&path[first as usize..]);
                    for (offset, place) in path[first as usize..].iter().enumerate() {
                        state[*place as usize] = State::Cycle { base, length, entry: offset as u32, before: 0 };
                    }
                    for (offset, place) in path[..first as usize].iter().enumerate() {
                        state[*place as usize] =
                            State::Cycle { base, length, entry: 0, before: u64::from(first) - offset as u64 };
                    }
                    break;
                }
                State::Cycle { base, length, entry, before } => {
                    let reached = path.len() as u64;
                    for (offset, place) in path.iter().enumerate() {
                        state[*place as usize] =
                            State::Cycle { base, length, entry, before: before + reached - offset as u64 };
                    }
                    break;
                }
            }
        }
    }
    output.kinds = table.kinds.clone();
    output.cells.clear();
    let mut kept = 0;
    for at in 0..count {
        let State::Cycle { base, length, entry, before } = state[at] else {
            continue;
        };
        // `before <= count - 1 < far`, so the far row lies on the cycle.
        let on = (u64::from(entry) + (far - before)) % u64::from(length);
        let last = cycles[base as usize + on as usize] as usize;
        let start = output.cells.len();
        output.cells.extend_from_slice(row(at));
        output.cells[start + to] = row(last)[to];
        kept += 1;
    }
    output.count = RowCount(kept);
    true
}

/// One probe of `table` with itself, as the chain's probe runs it: each left row with each right row of its key, the right
/// rows ascending.
fn stepped(chase: &ChaseParts, table: &Table) -> Result<Table, PortableOperatorFault> {
    let columns = table.kinds.len();
    let count = table.count.0;
    let row = |at: usize| &table.cells[at * columns..(at + 1) * columns];
    let cell = |at: usize, column: ColumnIndex| {
        row(at)
            .get(usize::from(column))
            .copied()
            .ok_or(PortableOperatorFault::NoSuchColumn { index: column })
    };
    let kinds: Vec<CellKind> = chase
        .output
        .iter()
        .map(|column| {
            table
                .kinds
                .get(usize::from(column.column))
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn { index: column.column })
        })
        .collect::<Result<_, _>>()?;
    let mut by_key: hashbrown::HashMap<Vec<Value>, Vec<usize>> = hashbrown::HashMap::new();
    for at in 0..count {
        let key = chase.right_on.iter().map(|column| cell(at, *column)).collect::<Result<Vec<_>, _>>()?;
        by_key.entry(key).or_default().push(at);
    }
    let mut out = Table {
        kinds,
        cells: Vec::new(),
        count: RowCount(0),
    };
    let mut seen: hashbrown::HashSet<Vec<Value>> = hashbrown::HashSet::new();
    for left in 0..count {
        let key = chase.left_on.iter().map(|column| cell(left, *column)).collect::<Result<Vec<_>, _>>()?;
        let Some(rights) = by_key.get(&key) else {
            continue;
        };
        for &right in rights {
            let made: Vec<Value> = chase
                .output
                .iter()
                .map(|column| cell(if column.side == Side::Left { left } else { right }, column.column))
                .collect::<Result<_, _>>()?;
            if chase.repeated == RepeatedRows::Dropped && !seen.insert(made.clone()) {
                continue;
            }
            out.cells.extend_from_slice(&made);
            out.count = RowCount(out.count.0 + 1);
        }
    }
    Ok(out)
}
