//! The reference workloads of the stage loops, the index build and the table fills, on synthetic columns: what the crate's
//! bench times as cost per row (scope-engine3 2.12) and what the loops' cases run. Test support, never a public API.

use self::workloads::{BATCH_ROWS, KernelWorkload, ReferenceKernel, plain};
use super::service::{compact, gather, keep, latest, probe, probe_keep, product, read, scan};
use super::types::{
    BatchKey, BatchValue, Batches, CellTest, FanCursor, HandleRun, KeptWhen, KeyPart,
};
use crate::domain::code::types::IndexSite;
use crate::domain::indexes::reading::service::read as index_of;
use crate::domain::indexes::service::{begin, clear_indexes, finish, key_column};
use crate::domain::indexes::types::FileStamp;
use crate::domain::made_tables::service::{
    clear_rows, count_groups, set_keys, sort_groups, write, write_new,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_orders::types::OrderBound;
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub mod workloads;

/// Runs `kernel` once over its workload, batch by batch, and answers how many rows it gave.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::stages::fixtures::run_kernel;
/// use ryo_engine::domain::stages::fixtures::workloads::{ReferenceKernel, workload};
///
/// let mut probes = workload(ReferenceKernel::ProbePresent, 1000);
/// assert_eq!(run_kernel(ReferenceKernel::ProbePresent, &mut probes), 504);
/// ```
pub fn run_kernel(kernel: ReferenceKernel, work: &mut KernelWorkload) -> usize {
    match kernel {
        ReferenceKernel::Scan
        | ReferenceKernel::Read
        | ReferenceKernel::KeepConstant
        | ReferenceKernel::KeepColumns
        | ReferenceKernel::ProbeRows
        | ReferenceKernel::ProbeIds
        | ReferenceKernel::ProbePresent
        | ReferenceKernel::ProbeAbsent
        | ReferenceKernel::Latest
        | ReferenceKernel::Write
        | ReferenceKernel::WriteNew
        | ReferenceKernel::FillRows => batched(kernel, work),
        ReferenceKernel::Product => {
            let side = work.rows.isqrt();
            let fan = &mut work.batch.3;
            fan.cursor = FanCursor::default();
            let mut given = 0_usize;
            loop {
                fan.up.clear();
                fan.out.clear();
                let done = product(Count(side), Count(0), Count(side), fan);
                given = given.saturating_add(fan.out.len());
                if done == Batches::Done {
                    break given;
                }
            }
        }
        ReferenceKernel::IndexOne
        | ReferenceKernel::IndexIds
        | ReferenceKernel::IndexTwo
        | ReferenceKernel::IndexOrdered => {
            clear_indexes(&mut work.index);
            let two = kernel == ReferenceKernel::IndexTwo;
            begin(
                &mut work.index,
                Count(work.rows),
                Count(if two { 2 } else { 1 }),
            );
            if kernel == ReferenceKernel::IndexOrdered {
                key_column(&mut work.index, Count(0), plain(&work.grouped));
            } else if kernel == ReferenceKernel::IndexIds {
                key_column(&mut work.index, Count(0), plain(&work.ids));
            } else {
                key_column(&mut work.index, Count(0), plain(&work.unique));
            }
            if two {
                key_column(&mut work.index, Count(1), plain(&work.small));
            }
            let order = (kernel == ReferenceKernel::IndexOrdered).then(|| plain(&work.small));
            finish(&mut work.index, IndexSite(0), FileStamp(0), order);
            index_of(&work.index, IndexSite(0)).rows.len()
        }
        ReferenceKernel::FillGroupCount | ReferenceKernel::FillSorted => {
            let [out, input] = &mut work.tables;
            set_keys(&mut work.room, std::iter::once(ColumnIndex(0)));
            if kernel == ReferenceKernel::FillGroupCount {
                clear_rows(out, Count(2));
                if count_groups(input, None, &mut work.room, out).is_err() {
                    return 0;
                }
            } else {
                clear_rows(out, Count(3));
                sort_groups(input, Count(1), &mut work.room, out);
            }
            out.count.0
        }
    }
}

/// The rows `first..end` of a column.
fn part(column: &[CellWord], first: usize, end: usize) -> &[CellWord] {
    column.get(first..end).unwrap_or_default()
}

/// Runs a loop that reads its input batch by batch, and answers how many rows it gave.
fn batched(kernel: ReferenceKernel, work: &mut KernelWorkload) -> usize {
    let KernelWorkload {
        rows,
        unique,
        ids,
        small,
        probes,
        index,
        batch: (handles, cells, selection, fan),
        tables: [table, sorted],
        ..
    } = work;
    let [every, half] = probes;
    let (unique, small, every, half): (&[CellWord], &[CellWord], &[CellWord], &[CellWord]) =
        (unique, small, every, half);
    let ids: &[CellWord] = ids;
    let index = index_of(index, IndexSite(0));
    clear_rows(table, Count(3));
    let mut given = 0_usize;
    let mut first = 0_usize;
    while first < *rows {
        let end = first.saturating_add(BATCH_ROWS).min(*rows);
        let count = end.saturating_sub(first);
        let mut key = [CellWord(0)];
        match kernel {
            ReferenceKernel::Scan => {
                scan(
                    HandleRun {
                        first: Count(first),
                        end: Count(end),
                        tag: CellWord(0),
                    },
                    handles,
                );
                given = given.saturating_add(handles.len());
            }
            ReferenceKernel::Read => {
                scan(
                    HandleRun {
                        first: Count(first),
                        end: Count(end),
                        tag: CellWord(0),
                    },
                    handles,
                );
                read(plain(small), handles, cells);
                given = given.saturating_add(cells.len());
            }
            ReferenceKernel::KeepConstant | ReferenceKernel::KeepColumns => {
                let right = if kernel == ReferenceKernel::KeepConstant {
                    BatchValue::Constant(CellWord(7))
                } else {
                    BatchValue::Column(part(unique, first, end))
                };
                let test = if kernel == ReferenceKernel::KeepConstant {
                    CellTest::Equal
                } else {
                    CellTest::Less
                };
                keep(
                    test,
                    BatchValue::Column(part(small, first, end)),
                    right,
                    Count(count),
                    selection,
                );
                cells.clear();
                cells.extend_from_slice(part(unique, first, end));
                compact(cells, selection);
                given = given.saturating_add(cells.len());
            }
            ReferenceKernel::ProbeRows
            | ReferenceKernel::ProbeIds
            | ReferenceKernel::Latest
            | ReferenceKernel::FillRows => {
                let column = if kernel == ReferenceKernel::Latest {
                    small
                } else if kernel == ReferenceKernel::ProbeIds {
                    ids
                } else {
                    every
                };
                let parts = [KeyPart {
                    place: Count(0),
                    column: part(column, first, end),
                }];
                fan.cursor = FanCursor::default();
                loop {
                    fan.up.clear();
                    fan.out.clear();
                    let probed = BatchKey {
                        cells: &mut key,
                        parts: &parts,
                    };
                    let done = if kernel == ReferenceKernel::Latest {
                        latest(
                            index,
                            &sorted.orders,
                            probed,
                            BatchValue::Column(part(unique, first, end)),
                            OrderBound::AtOrBefore,
                            Count(count),
                            fan,
                        )
                    } else {
                        probe(index, probed, Count(count), fan)
                    };
                    gather(part(small, first, end), &fan.up, cells);
                    if kernel == ReferenceKernel::FillRows {
                        let sources = [
                            BatchValue::Column(&fan.out),
                            BatchValue::Column(cells),
                            BatchValue::Constant(CellWord(1)),
                        ];
                        write(table, &sources, Count(fan.out.len()));
                    }
                    given = given.saturating_add(fan.out.len());
                    if done == Batches::Done {
                        break;
                    }
                }
            }
            ReferenceKernel::ProbePresent | ReferenceKernel::ProbeAbsent => {
                let parts = [KeyPart {
                    place: Count(0),
                    column: part(half, first, end),
                }];
                let kept = if kernel == ReferenceKernel::ProbePresent {
                    KeptWhen::Present
                } else {
                    KeptWhen::Absent
                };
                let probed = BatchKey {
                    cells: &mut key,
                    parts: &parts,
                };
                probe_keep(index, probed, Count(count), kept, selection);
                given = given.saturating_add(selection.len());
            }
            ReferenceKernel::Write | ReferenceKernel::WriteNew => {
                let sources = [
                    BatchValue::Column(part(half, first, end)),
                    BatchValue::Column(part(small, first, end)),
                    BatchValue::Constant(CellWord(1)),
                ];
                if kernel == ReferenceKernel::Write {
                    write(table, &sources, Count(count));
                } else {
                    write_new(table, &sources, Count(count));
                }
                given = table.count.0;
            }
            ReferenceKernel::Product
            | ReferenceKernel::IndexOne
            | ReferenceKernel::IndexIds
            | ReferenceKernel::IndexTwo
            | ReferenceKernel::IndexOrdered
            | ReferenceKernel::FillGroupCount
            | ReferenceKernel::FillSorted => {}
        }
        first = end;
    }
    given
}
