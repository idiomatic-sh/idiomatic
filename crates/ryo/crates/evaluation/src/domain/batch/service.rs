//! The batch meanings: the same rows the interpreter's one-row-at-a-time meanings answer, made from whole inputs. Repeated rows
//! follow the operators' policy: `union`, `intersection`, `difference` and a dropping `project` keep a row once; every other
//! operation keeps what its inputs repeat.

use super::types::Rows;
use ryo_relations::domain::relations::types::{Count, Integer, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, JoinColumn, RepeatedRows, Side};
use rustc_hash::{FxHashMap, FxHashSet};

/// The row at `index` of `rows`.
#[must_use]
pub fn row(rows: &Rows, index: usize) -> &[Value] {
    rows.cells
        .get(index * rows.width..(index + 1) * rows.width)
        .unwrap_or_default()
}

/// Every row of `rows`, in order.
pub fn each(rows: &Rows) -> impl Iterator<Item = &[Value]> {
    (0..rows.count).map(move |index| row(rows, index))
}

/// No rows, `width` cells wide.
#[must_use]
pub fn empty(width: usize) -> Rows {
    Rows {
        width,
        count: 0,
        cells: Vec::new(),
    }
}

/// Adds `cells` as one more row of `rows`.
pub fn push(rows: &mut Rows, cells: &[Value]) {
    if rows.count == 0 {
        rows.width = cells.len();
    }
    rows.cells.extend_from_slice(cells);
    rows.count += 1;
}

/// The cell at `column` of `row`, or `None` when it has no such column.
#[must_use]
pub fn cell(row: &[Value], column: ColumnIndex) -> Option<Value> {
    row.get(usize::try_from(column.0).ok()?).copied()
}

/// The cells of `row` at `columns`, in order, appended to `into`; `false` when a column is missing.
pub fn pick(row: &[Value], columns: &[ColumnIndex], into: &mut Vec<Value>) -> bool {
    for column in columns {
        let Some(value) = cell(row, *column) else {
            return false;
        };
        into.push(value);
    }
    true
}

/// Keeps a row the first time it comes.
#[derive(Debug, Default)]
pub struct Distinct(FxHashSet<Box<[Value]>>);

/// Whether `row` comes for the first time to `seen`; it is remembered.
pub fn first_time(seen: &mut Distinct, row: &[Value]) -> bool {
    if seen.0.contains(row) {
        return false;
    }
    seen.0.insert(row.into())
}

/// `project`: the cells at `columns` of each row, a repeated row dropped when `distinct` says so.
#[must_use]
pub fn project(from: &Rows, columns: &[ColumnIndex], distinct: RepeatedRows) -> Rows {
    let mut answer = empty(columns.len());
    let mut seen = Distinct::default();
    let mut made = Vec::with_capacity(columns.len());
    for each in each(from) {
        made.clear();
        if !pick(each, columns, &mut made) {
            continue;
        }
        if distinct == RepeatedRows::Dropped && !first_time(&mut seen, &made) {
            continue;
        }
        push(&mut answer, &made);
    }
    answer
}

/// `extend`: each row with `appended` after it.
#[must_use]
pub fn extend(from: &Rows, appended: &[Value]) -> Rows {
    let mut answer = empty(from.width + appended.len());
    let mut made = Vec::with_capacity(answer.width);
    for each in each(from) {
        made.clear();
        made.extend_from_slice(each);
        made.extend_from_slice(appended);
        push(&mut answer, &made);
    }
    answer
}

/// `union`: the rows of `left` then of `right`, each once.
#[must_use]
pub fn union(left: &Rows, right: &Rows) -> Rows {
    let mut answer = empty(left.width.max(right.width));
    let mut seen = Distinct::default();
    for each in each(left).chain(each(right)) {
        if first_time(&mut seen, each) {
            push(&mut answer, each);
        }
    }
    answer
}

/// `intersection` (`keep` true) or `difference` (`keep` false): the rows of `left` that `right` does, or does not, hold, each once.
#[must_use]
pub fn membership(left: &Rows, right: &Rows, keep: bool) -> Rows {
    let held: FxHashSet<&[Value]> = each(right).collect();
    let mut answer = empty(left.width);
    let mut seen = Distinct::default();
    for each in each(left) {
        if held.contains(each) == keep && first_time(&mut seen, each) {
            push(&mut answer, each);
        }
    }
    answer
}

/// `antijoin`: the rows of `left` whose key at `left_on` no row of `right` has at `right_on`, repeated as `left` repeats them.
#[must_use]
pub fn antijoin(
    left: &Rows,
    right: &Rows,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
) -> Rows {
    if right.count == 0 {
        return left.clone();
    }
    let mut keys: FxHashSet<Vec<Value>> = FxHashSet::default();
    for each in each(right) {
        let mut key = Vec::with_capacity(right_on.len());
        if pick(each, right_on, &mut key) {
            let _: bool = keys.insert(key);
        }
    }
    let mut answer = empty(left.width);
    let mut key = Vec::with_capacity(left_on.len());
    for each in each(left) {
        key.clear();
        if pick(each, left_on, &mut key) && !keys.contains(&key) {
            push(&mut answer, each);
        }
    }
    answer
}

/// The rows of `right` by their key at `on`, each key with its rows in order.
fn index(right: &Rows, on: &[ColumnIndex]) -> FxHashMap<Vec<Value>, Vec<usize>> {
    let mut index: FxHashMap<Vec<Value>, Vec<usize>> = FxHashMap::default();
    for at in 0..right.count {
        let mut key = Vec::with_capacity(on.len());
        if pick(row(right, at), on, &mut key) {
            index.entry(key).or_default().push(at);
        }
    }
    index
}

/// `join`: for each row of `left` in order, each row of `right` with the same key in order, as the columns `output` names.
/// `view` maps the right input's logical columns onto its physical ones.
#[must_use]
pub fn join(
    left: &Rows,
    right: &Rows,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
    output: &[JoinColumn],
    view: &[ColumnIndex],
) -> Rows {
    let physical = |column: ColumnIndex| {
        view.get(usize::try_from(column.0).unwrap_or(usize::MAX))
            .copied()
            .unwrap_or(column)
    };
    let right_on: Vec<ColumnIndex> = right_on.iter().map(|column| physical(*column)).collect();
    let index = index(right, &right_on);
    let mut answer = empty(output.len());
    let mut key = Vec::with_capacity(left_on.len());
    let mut made = Vec::with_capacity(output.len());
    for each in each(left) {
        key.clear();
        if !pick(each, left_on, &mut key) {
            continue;
        }
        let Some(matched) = index.get(&key) else {
            continue;
        };
        for at in matched {
            let other = row(right, *at);
            made.clear();
            let complete = output.iter().all(|column| {
                let value = match column.side {
                    Side::Left => cell(each, column.column),
                    Side::Right => cell(other, physical(column.column)),
                };
                value.map(|value| made.push(value)).is_some()
            });
            if complete {
                push(&mut answer, &made);
            }
        }
    }
    answer
}

/// `group-count`: one row for each group of equal `keys`, in the order groups first come, with the group's count of rows, or of
/// distinct values of `distinct`, after its keys.
#[must_use]
pub fn group_count(from: &Rows, keys: &[ColumnIndex], distinct: Option<ColumnIndex>) -> Rows {
    let mut groups: FxHashMap<Vec<Value>, usize> = FxHashMap::default();
    let mut order: Vec<(Vec<Value>, Count, FxHashSet<Value>)> = Vec::new();
    for each in each(from) {
        let mut key = Vec::with_capacity(keys.len());
        if !pick(each, keys, &mut key) {
            continue;
        }
        let place = *groups.entry(key.clone()).or_insert_with(|| {
            order.push((key, Count(0), FxHashSet::default()));
            order.len() - 1
        });
        if let Some((_, count, values)) = order.get_mut(place) {
            match distinct.and_then(|column| cell(each, column)) {
                Some(value) => {
                    if values.insert(value) {
                        count.0 += 1;
                    }
                }
                None => count.0 += 1,
            }
        }
    }
    let mut answer = empty(keys.len() + 1);
    for (mut key, count, _) in order {
        key.push(Value::Number(Integer(
            i64::try_from(count.0).unwrap_or(i64::MAX),
        )));
        push(&mut answer, &key);
    }
    answer
}

/// `closure`: within each partition of equal `keys`, every pair of a source and a vertex it reaches over one or more edges, the
/// partition's keys first.
#[must_use]
pub fn closure(
    edges: &Rows,
    keys: &[ColumnIndex],
    source: ColumnIndex,
    destination: ColumnIndex,
) -> Rows {
    let mut partitions: FxHashMap<Vec<Value>, usize> = FxHashMap::default();
    let mut graphs: Vec<(Vec<Value>, Vec<Value>, FxHashMap<Value, Vec<Value>>)> = Vec::new();
    for each in each(edges) {
        let mut key = Vec::with_capacity(keys.len());
        let (Some(from), Some(to)) = (cell(each, source), cell(each, destination)) else {
            continue;
        };
        if !pick(each, keys, &mut key) {
            continue;
        }
        let place = *partitions.entry(key.clone()).or_insert_with(|| {
            graphs.push((key, Vec::new(), FxHashMap::default()));
            graphs.len() - 1
        });
        if let Some((_, sources, adjacency)) = graphs.get_mut(place) {
            let next = adjacency.entry(from).or_insert_with(|| {
                sources.push(from);
                Vec::new()
            });
            if !next.contains(&to) {
                next.push(to);
            }
        }
    }
    let mut answer = empty(keys.len() + 2);
    let mut made = Vec::new();
    for (key, sources, adjacency) in &graphs {
        for start in sources {
            let mut seen: FxHashSet<Value> = FxHashSet::default();
            let mut stack: Vec<Value> = adjacency.get(start).cloned().unwrap_or_default();
            stack.reverse();
            while let Some(vertex) = stack.pop() {
                if !seen.insert(vertex) {
                    continue;
                }
                made.clear();
                made.extend_from_slice(key);
                made.extend_from_slice(&[*start, vertex]);
                push(&mut answer, &made);
                if let Some(next) = adjacency.get(&vertex) {
                    stack.extend(next.iter().rev());
                }
            }
        }
    }
    answer
}

/// The numeric order of `row` at `column`.
fn order_of(row: &[Value], column: ColumnIndex) -> Option<i64> {
    match cell(row, column)? {
        Value::Number(Integer(number)) => Some(number),
        Value::Node(_)
        | Value::Symbol(_)
        | Value::Name(_)
        | Value::File(_)
        | Value::Span(_)
        | Value::Flag(_) => None,
    }
}

/// The arguments of `ordered-match` past its inputs.
#[derive(Clone, Copy, Debug)]
pub struct Ordering<'a> {
    /// The left key columns.
    pub left_on: &'a [ColumnIndex],
    /// The right key columns.
    pub right_on: &'a [ColumnIndex],
    /// The left order column.
    pub left_order: ColumnIndex,
    /// The right order column.
    pub right_order: ColumnIndex,
    /// The columns of the answer.
    pub output: &'a [JoinColumn],
}

/// `ordered-match`: within each group of equal keys, each left row in ascending order takes the earliest right row not yet taken
/// whose order is greater than its own.
#[must_use]
pub fn ordered_match(left: &Rows, right: &Rows, ordering: Ordering<'_>) -> Rows {
    let mut rights: FxHashMap<Vec<Value>, Vec<(i64, usize)>> = FxHashMap::default();
    for at in 0..right.count {
        let each = row(right, at);
        let mut key = Vec::new();
        if let (true, Some(order)) = (
            pick(each, ordering.right_on, &mut key),
            order_of(each, ordering.right_order),
        ) {
            rights.entry(key).or_default().push((order, at));
        }
    }
    for group in rights.values_mut() {
        group.sort_unstable();
    }
    let mut lefts: Vec<(Vec<Value>, i64, usize)> = Vec::new();
    for at in 0..left.count {
        let each = row(left, at);
        let mut key = Vec::new();
        if let (true, Some(order)) = (
            pick(each, ordering.left_on, &mut key),
            order_of(each, ordering.left_order),
        ) {
            lefts.push((key, order, at));
        }
    }
    lefts.sort_unstable();
    let mut next: FxHashMap<&[Value], usize> = FxHashMap::default();
    let mut answer = empty(ordering.output.len());
    let mut made = Vec::new();
    for (key, order, at) in &lefts {
        let Some(group) = rights.get(key) else {
            continue;
        };
        let cursor = next.entry(key.as_slice()).or_insert(0);
        while group.get(*cursor).is_some_and(|(right, _)| right <= order) {
            *cursor += 1;
        }
        let Some((_, taken)) = group.get(*cursor) else {
            continue;
        };
        *cursor += 1;
        let (one, other) = (row(left, *at), row(right, *taken));
        made.clear();
        let complete = ordering.output.iter().all(|column| {
            let value = match column.side {
                Side::Left => cell(one, column.column),
                Side::Right => cell(other, column.column),
            };
            value.map(|value| made.push(value)).is_some()
        });
        if complete {
            push(&mut answer, &made);
        }
    }
    answer
}

/// The arguments of `axis` past its inputs.
#[derive(Clone, Copy, Debug)]
pub struct Walking {
    /// Which way to walk.
    pub direction: Direction,
    /// The starting entity's column in `from`.
    pub id: ColumnIndex,
    /// The parent column in the edges.
    pub parent: ColumnIndex,
    /// The child column in the edges.
    pub child: ColumnIndex,
}

/// `axis` over an index of edges: each row of `from` once for each entity its walk reaches that `domain` holds, the entity after
/// the row, in walk order; a nearest ancestor is the first only. The start is not among what it reaches.
#[must_use]
pub fn axis(from: &Rows, edges: &Rows, domain: &FxHashSet<Value>, walking: Walking) -> Rows {
    let mut parents: FxHashMap<Value, Vec<Value>> = FxHashMap::default();
    let mut children: FxHashMap<Value, Vec<Value>> = FxHashMap::default();
    for each in each(edges) {
        if let (Some(parent), Some(child)) = (cell(each, walking.parent), cell(each, walking.child)) {
            parents.entry(child).or_default().push(parent);
            children.entry(parent).or_default().push(child);
        }
    }
    let mut answer = empty(from.width + 1);
    let mut made = Vec::with_capacity(from.width + 1);
    let mut reached = Vec::new();
    for each in each(from) {
        let Some(start) = cell(each, walking.id) else {
            continue;
        };
        reached.clear();
        match walking.direction {
            Direction::Parent => reached.extend(parents.get(&start).into_iter().flatten().copied()),
            Direction::Child => reached.extend(children.get(&start).into_iter().flatten().copied()),
            Direction::Ancestor | Direction::NearestAncestor => {
                let mut seen: FxHashSet<Value> = FxHashSet::default();
                let mut at = start;
                while let Some(parent) = parents.get(&at).and_then(|up| up.first()).copied() {
                    if !seen.insert(parent) {
                        break;
                    }
                    reached.push(parent);
                    at = parent;
                }
            }
            Direction::Descendant => {
                let mut seen: FxHashSet<Value> = FxHashSet::default();
                let mut stack: Vec<Value> = children.get(&start).cloned().unwrap_or_default();
                stack.reverse();
                while let Some(vertex) = stack.pop() {
                    if !seen.insert(vertex) {
                        continue;
                    }
                    reached.push(vertex);
                    if let Some(next) = children.get(&vertex) {
                        stack.extend(next.iter().rev());
                    }
                }
            }
            Direction::Deep | Direction::PreviousSibling | Direction::NextSibling => {}
        }
        for vertex in &reached {
            if !domain.contains(vertex) {
                continue;
            }
            made.clear();
            made.extend_from_slice(each);
            made.push(*vertex);
            push(&mut answer, &made);
            if walking.direction == Direction::NearestAncestor {
                break;
            }
        }
    }
    answer
}
