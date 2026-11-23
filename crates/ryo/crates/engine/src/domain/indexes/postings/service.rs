//! Build a dense or sparse posting directory once per table and key.
use super::constants::{INDEX_CELLS, KEY_CELLS};
use super::types::{
    PackedPositions, PostingBuffers, PostingIndex, PostingInterval, PostingPosition, PostingRows,
    SortedPosting,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// Mix one column of a compound key.
const fn mixed(hash: CellWord, value: CellWord) -> CellWord {
    CellWord((hash.0.rotate_left(5) ^ value.0).wrapping_mul(0x517c_c1b7_2722_0a95))
}

/// Hash composite keys in projection order. A one-column key keeps its value.
pub fn key(values: impl Iterator<Item = CellWord>) -> CellWord {
    let mut hash = 0_u64;
    let mut count = 0_usize;
    let mut first = 0;
    for value in values {
        if count == 0 {
            first = value.0;
        }
        hash = mixed(CellWord(hash), value).0;
        count += 1;
    }
    CellWord(if count == 1 { first } else { hash })
}

/// Take a cleared posting vector from this worker.
fn take(buffers: &mut PostingBuffers) -> PackedPositions {
    let rows = buffers.indexes.pop().unwrap_or_default();
    buffers.index_capacity.0 -= rows.0.capacity();
    rows
}

/// Count dense keys, then scatter in source order.
fn dense(buffers: &mut PostingBuffers, maximum: Count) -> Option<PostingIndex> {
    let mut offsets = take(buffers);
    offsets.0.resize(maximum.0 + 2, 0);
    for &value in &buffers.keys {
        *offsets.0.get_mut(
            usize::try_from(value.0)
                .unwrap_or(usize::MAX)
                .checked_add(1)?,
        )? += 1;
    }
    let mut total = 0_u32;
    for slot in &mut offsets.0 {
        total += *slot;
        *slot = total;
    }
    buffers.cursor.0.clear();
    buffers
        .cursor
        .0
        .extend_from_slice(offsets.0.get(..=maximum.0)?);
    let mut rows = take(buffers);
    rows.0.resize(buffers.keys.len(), 0);
    for (row, &value) in buffers.keys.iter().enumerate() {
        let cursor = buffers
            .cursor
            .0
            .get_mut(usize::try_from(value.0).unwrap_or(usize::MAX))?;
        *rows
            .0
            .get_mut(usize::try_from(*cursor).unwrap_or(usize::MAX))? =
            u32::try_from(row).unwrap_or(u32::MAX);
        *cursor += 1;
    }
    Some(PostingIndex {
        offsets,
        rows,
        ..PostingIndex::default()
    })
}

/// Sort sparse key and row pairs and record each group once.
fn sparse(buffers: &mut PostingBuffers) -> PostingIndex {
    buffers.order.clear();
    buffers.order.extend(
        buffers
            .keys
            .iter()
            .enumerate()
            .map(|(row, &value)| SortedPosting {
                key: value,
                row: PostingPosition(u32::try_from(row).unwrap_or(u32::MAX)),
            }),
    );
    buffers.order.sort_unstable();
    let mut ranges = buffers.directories.pop().unwrap_or_default();
    ranges.reserve(buffers.order.len().min(KEY_CELLS));
    let mut start = 0_u32;
    for group in buffers.order.chunk_by(|left, right| left.key == right.key) {
        if let Some(first) = group.first() {
            let end = start + u32::try_from(group.len()).unwrap_or(u32::MAX);
            let _previous = ranges.insert(
                first.key,
                PostingInterval {
                    start: PostingPosition(start),
                    end: PostingPosition(end),
                },
            );
            start = end;
        }
    }
    let mut rows = take(buffers);
    rows.0.extend(buffers.order.iter().map(|entry| entry.row.0));
    PostingIndex {
        ranges,
        rows,
        ..PostingIndex::default()
    }
}

/// Build an index over the bound columns. An invalid input or unrepresentable row count is refused.
pub fn build(
    buffers: &mut PostingBuffers,
    count: Count,
    mut key_at: impl FnMut(ryo_values::domain::rows::types::RowIndex) -> CellWord,
) -> Option<PostingIndex> {
    if u32::try_from(count.0).is_err() {
        return None;
    }
    buffers.keys.clear();
    buffers
        .keys
        .extend((0..count.0).map(|row| key_at(ryo_values::domain::rows::types::RowIndex(row))));
    let max = buffers.keys.iter().copied().max().unwrap_or_default().0;
    let result = if count.0 > 0 && max < (4 * u64::try_from(count.0).unwrap_or(u64::MAX)).max(1024)
    {
        dense(buffers, Count(usize::try_from(max).unwrap_or_default()))
    } else {
        Some(sparse(buffers))
    };
    if buffers.keys.capacity() > KEY_CELLS || buffers.order.capacity() > KEY_CELLS {
        buffers.keys = Vec::new();
        buffers.order = Vec::new();
        buffers.cursor = PackedPositions::default();
    }
    result
}

/// Borrow one key's matching rows without allocating.
#[must_use]
#[inline]
pub fn probe(index: &PostingIndex, key: CellWord) -> Option<PostingRows<'_>> {
    let (start, end) = if index.offsets.0.is_empty() {
        let span = index.ranges.get(&key)?;
        (span.start.0, span.end.0)
    } else {
        let Ok(at) = usize::try_from(key.0) else {
            return None;
        };
        let start = *index.offsets.0.get(at)?;
        let end = *index.offsets.0.get(at.checked_add(1)?)?;
        (start, end)
    };
    if start == end {
        return None;
    }
    index
        .rows
        .0
        .get(
            usize::try_from(start).unwrap_or(usize::MAX)
                ..usize::try_from(end).unwrap_or(usize::MAX),
        )
        .map(PostingRows)
}

/// Return the index buffers after the file releases its last borrow.
pub fn retire(buffers: &mut PostingBuffers, index: PostingIndex) {
    for mut rows in [index.offsets, index.rows] {
        if rows.0.capacity() > 0 && buffers.index_capacity.0 + rows.0.capacity() <= INDEX_CELLS {
            rows.0.clear();
            buffers.index_capacity.0 += rows.0.capacity();
            buffers.indexes.push(rows);
        }
    }
    let mut ranges = index.ranges;
    if ranges.capacity() > 0 && ranges.capacity() <= 4096 && buffers.directories.len() < 64 {
        ranges.clear();
        buffers.directories.push(ranges);
    }
}

/// Publish into a uniquely owned header retained from an earlier file.
pub fn share(buffers: &mut PostingBuffers, index: PostingIndex) -> std::rc::Rc<PostingIndex> {
    if let Some(mut kept) = buffers.shared.pop()
        && let Some(slot) = std::rc::Rc::get_mut(&mut kept)
    {
        *slot = index;
        return kept;
    }
    std::rc::Rc::new(index)
}

/// Recover the header and vectors when the last file reader lets go.
pub fn release(buffers: &mut PostingBuffers, mut index: std::rc::Rc<PostingIndex>) {
    if let Some(slot) = std::rc::Rc::get_mut(&mut index) {
        retire(buffers, std::mem::take(slot));
        if buffers.shared.len() < 4096 {
            buffers.shared.push(index);
        }
    }
}

/// Test original columns after a composite hash selected a candidate row.
pub fn equal<Cell: PartialEq>(
    left: impl Iterator<Item = Cell>,
    right: impl Iterator<Item = Cell>,
) -> bool {
    left.eq(right)
}

/// Compute ordered posting keys directly on borrowed columns of a made table.
pub fn build_columns<Word>(
    buffers: &mut PostingBuffers,
    count: Count,
    columns: &[Vec<Word>],
    partition: super::types::PostingColumns<'_>,
    word: impl Fn(&Word) -> CellWord,
) -> Option<PostingIndex> {
    if let [only] = partition.0 {
        let column = columns.get(*only)?.get(..count.0)?;
        return build(buffers, count, |row| word(&column[row.0]));
    }
    if !partition
        .0
        .iter()
        .all(|&col| columns.get(col).is_some_and(|c| c.len() >= count.0))
    {
        return None;
    }
    build(buffers, count, |row| {
        key(partition.0.iter().map(|&col| word(&columns[col][row.0])))
    })
}

/// Gather candidate ordinals in probe order. Repeated keys repeat their entire posting.
pub fn collect_rows(
    index: &PostingIndex,
    keys: impl Iterator<Item = CellWord>,
    out: &mut PackedPositions,
) {
    out.0.clear();
    for key in keys {
        if let Some(rows) = probe(index, key) {
            out.0.extend_from_slice(rows.0);
        }
    }
}
