//! Schedule consecutive loans from the existing column-file adapter on one worker thread.
use ryo::languages::NodeTable;
use ryo_python::fixtures::columns::{ColumnFile, ColumnWalk};

/// Lend two adjacent files to a prefix consumer followed by an evaluator.
/// Both borrowed parses stay valid through evaluation, in request order.
/// Only supported files reach either callback, just as in `walked`.
pub(super) fn walked_pair<S>(
    walk: &mut ColumnWalk,
    table: &mut NodeTable,
    other: &mut ColumnWalk,
    other_table: &mut NodeTable,
    first: (&str, &[u8]),
    second: Option<(&str, &[u8])>,
    state: &mut [S; 2],
    prepare: &mut impl FnMut(usize, &mut ColumnFile<'_, '_>, &mut S),
    consume: &mut impl FnMut(usize, &mut ColumnFile<'_, '_>, &mut S),
) {
    let [here, there] = state;
    let mut visited = false;
    let _ = walk.walked(first.0, first.1, table, &mut |a| {
        visited = true;
        prepare(0, a, here);
        let mut consumed = false;
        if let Some((path, bytes)) = second {
            let _ = other.walked(path, bytes, other_table, &mut |b| {
                prepare(1, b, there);
                consume(0, a, here);
                consumed = true;
                consume(1, b, there);
            });
        }
        if !consumed {
            consume(0, a, here);
        }
    });
    if !visited && let Some((path, bytes)) = second {
        let _ = other.walked(path, bytes, other_table, &mut |b| {
            prepare(1, b, there);
            consume(1, b, there);
        });
    }
}
