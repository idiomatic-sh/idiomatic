//! The `ordering` service: free functions named by verb that put the findings of files in the order
//! they leave in and let the files leave in turn, with no IO and no ports.

use super::constants::TURN_OFFERED_ONCE;

use ryo_values::domain::row_texts::types::RowTotal;

use super::types::{Admittance, Ordered, PendingBytes, Turn};
use ryo_values::domain::files::types::FileSize;

use crate::domain::render::service::{finding_parts, lent_row, row_cells};
use crate::domain::render::types::{Rendered, RowLimit, RowNumber, RowOrder};
use ryo_values::domain::row_texts::types::{Cell, RowEnds, RowsTextAt, RowsTextView, TextRange};

/// A kept cell whose text moved `shift` bytes further into the text of its rows.
fn shifted(cell: Cell<TextRange>, shift: RowsTextAt) -> Cell<TextRange> {
    let moved = |range: TextRange| TextRange {
        start: RowsTextAt(range.start.0.saturating_add(shift.0)),
        end: RowsTextAt(range.end.0.saturating_add(shift.0)),
    };
    match cell {
        Cell::File(range) => Cell::File(moved(range)),
        Cell::Text(range) => Cell::Text(moved(range)),
        Cell::Number(number) => Cell::Number(number),
        Cell::Span(span) => Cell::Span(span),
    }
}

/// Adds the rows of `from` after those of `into`.
pub(crate) fn append(into: &mut Rendered, from: &Rendered) {
    let rows = u32::try_from(into.ends.0.len()).unwrap_or(u32::MAX);
    let cells = u32::try_from(into.cells.len()).unwrap_or(u32::MAX);
    let shift = RowsTextAt(u32::try_from(into.text.len()).unwrap_or(u32::MAX));
    into.text.push_str(&from.text);
    into.cells.extend(
        from.cells
            .iter()
            .map(|cell| cell.map(|kept| shifted(kept, shift))),
    );
    into.ends
        .0
        .extend(from.ends.0.iter().map(|end| end.saturating_add(cells)));
    into.order.extend(from.order.iter().map(|each| RowOrder {
        row: RowNumber(each.row.0.saturating_add(rows)),
        ..*each
    }));
    into.total.0 += from.total.0;
}

/// Puts the rows of each stretch of `rendered` that share one place in the order of their cells, so rows made in any order leave
/// in one: a stretch begins at each of `starts`, in order, and the last runs to the end.
pub(crate) fn order_alike(rendered: &mut Rendered, starts: &[RowNumber]) {
    let Rendered {
        text,
        cells,
        ends,
        order,
        ..
    } = rendered;
    let text = RowsTextView(&text.0);
    let cells: &[Option<Cell<TextRange>>] = cells;
    let ends: &RowEnds = ends;
    let row_of = |row: RowNumber| lent_row(text, row_cells(cells, ends, row));
    let mut bounds: Vec<usize> = starts
        .iter()
        .map(|start| usize::try_from(start.0).unwrap_or(usize::MAX))
        .collect();
    bounds.push(order.len());
    for pair in bounds.windows(2) {
        let (Some(from), Some(to)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        if let Some(stretch) = order.get_mut(*from..*to) {
            stretch.sort_by(|left, right| row_of(left.row).cmp(row_of(right.row)));
        }
    }
}

/// Puts the rows of a file of findings in the order they leave in: by subject, then rule, then key; a finding is its rule, its
/// subject and its key, so rows that give the same three are one (a join that fans out gives a row for each match).
pub(crate) fn sort_findings(rendered: &mut Rendered) {
    let Rendered {
        text,
        cells,
        ends,
        order,
        total,
        ..
    } = rendered;
    let text = RowsTextView(&text.0);
    let cells: &[Option<Cell<TextRange>>] = cells;
    let ends: &RowEnds = ends;
    let parts = |row: RowNumber| finding_parts(row_cells(cells, ends, row));
    let subject = |row: RowNumber| parts(row).map(|found| std::slice::from_ref(found.subject));
    let key = |row: RowNumber| parts(row).map(|found| found.key);
    let columns = |row: RowNumber| parts(row).map(|found| found.columns);
    // A row that is not a finding's sorts first; two findings' cells compare as the values they hold.
    let compare = |left: Option<&[Option<Cell<TextRange>>]>,
                   right: Option<&[Option<Cell<TextRange>>]>| match (left, right)
    {
        (Some(left), Some(right)) => lent_row(text, left).cmp(lent_row(text, right)),
        (left, right) => left.is_some().cmp(&right.is_some()),
    };
    order.sort_unstable_by(|left, right| {
        compare(subject(left.row), subject(right.row))
            .then(left.rule.cmp(&right.rule))
            .then_with(|| compare(key(left.row), key(right.row)))
            .then(left.line.cmp(&right.line))
            .then(left.column.cmp(&right.column))
            .then_with(|| compare(columns(left.row), columns(right.row)))
            .then(left.row.cmp(&right.row))
    });
    let before = order.len();
    order.dedup_by(|later, earlier| {
        later.rule == earlier.rule
            && compare(subject(later.row), subject(earlier.row)).is_eq()
            && compare(key(later.row), key(earlier.row)).is_eq()
    });
    total.0 = total
        .0
        .saturating_sub(u64::try_from(before - order.len()).unwrap_or(0));
}

/// Empties `rendered`, keeping its room.
pub(crate) fn clear(rendered: &mut Rendered) {
    rendered.text.clear();
    rendered.cells.clear();
    rendered.ends.0.clear();
    rendered.order.clear();
    rendered.total = RowTotal(0);
}

/// How many emptied buffers wait for a worker to render its next file in: about as many as there are workers.
const SPARES: usize = 16;

/// How many bytes of source the files read at the same time may hold between them: a file costs
/// memory in proportion to its size, so the biggest files wait for one another and the rest go on.
const ADMISSION_BYTES: u64 = 1 << 20;

/// Whether a file of `size` bytes may be read now: when nothing is being read, any file may; when
/// something is, only one that keeps the files being read within the limit.
#[must_use]
pub(crate) fn admitted(ordered: &Ordered, size: FileSize) -> Admittance {
    if ordered.inflight.0 == 0 || ordered.inflight.0.saturating_add(size.0) <= ADMISSION_BYTES {
        Admittance::Admitted
    } else {
        Admittance::Waits
    }
}

/// How many bytes of rows may wait for their turn before the workers that finished ahead of them wait too, so
/// the memory a run holds does not grow with the rows it answers.
pub(crate) const PENDING_LIMIT: usize = 16 << 20;

/// How many bytes the rows of `rendered` hold: their cells and the text of their file and text cells.
fn held_bytes(rendered: &Rendered) -> PendingBytes {
    PendingBytes(
        rendered
            .cells
            .len()
            .saturating_mul(size_of::<Option<Cell<TextRange>>>())
            .saturating_add(rendered.text.len()),
    )
}

/// Queue an out-of-order file, or return it when it is ready for the caller to publish.
///
/// # Panics
///
/// When the file's turn was offered before: each file is rendered once, so a second offer is a bug that must crash, not text
/// counted twice and a file's rows lost.
#[expect(
    clippy::expect_used,
    reason = "each file is rendered and offered once; a second offer of a turn is a bug that must crash, not text counted twice"
)]
pub(crate) fn offer(ordered: &mut Ordered, file: Turn, rendered: Rendered) -> Option<Rendered> {
    if file == ordered.next {
        return Some(rendered);
    }
    ordered.held.0 += held_bytes(&rendered).0;
    let replaced = ordered.pending.insert(file, rendered);
    replaced.is_none().then_some(()).expect(TURN_OFFERED_ONCE);
    None
}

/// Account for a ready file: all its rows to the total, and those the limit leaves room for to the rows shown.
pub(crate) fn account(ordered: &mut Ordered, rendered: &Rendered) {
    // How many rows of the ready file the limit leaves room for.
    let allowed = usize::try_from(ordered.limit.0.saturating_sub(ordered.shown.0))
        .unwrap_or(usize::MAX)
        .min(rendered.order.len());
    ordered.total.0 += rendered.total.0;
    ordered.shown.0 += u64::try_from(allowed).unwrap_or(u64::MAX);
}

/// Account for a ready file and give the caller its allowed row count.
pub(crate) fn allowance(ordered: &mut Ordered, rendered: &Rendered) -> RowLimit {
    // How many rows of the ready file the limit leaves room for.
    let allowed = usize::try_from(ordered.limit.0.saturating_sub(ordered.shown.0))
        .unwrap_or(usize::MAX)
        .min(rendered.order.len());
    account(ordered, rendered);
    RowLimit(allowed)
}

/// Advance after publication, returning the next waiting file if it is ready.
pub(crate) fn next_ready(ordered: &mut Ordered) -> Option<Rendered> {
    ordered.next.0 += 1;
    let waiting = ordered.pending.remove(&ordered.next)?;
    ordered.held.0 = ordered.held.0.saturating_sub(held_bytes(&waiting).0);
    Some(waiting)
}

/// Retain a bounded number of cleared output buffers for the next file.
pub(crate) fn recycle(ordered: &mut Ordered, mut rendered: Rendered) {
    if ordered.spares.len() < SPARES {
        clear(&mut rendered);
        ordered.spares.push(rendered);
    }
}
