//! The synthetic inputs of the reference workloads: which loops are timed, their names, and the columns each runs over.

use crate::domain::code::types::IndexSite;
use crate::domain::indexes::service::{begin, finish, key_column};
use crate::domain::indexes::types::{ColumnView, FileStamp, IndexArena};
use crate::domain::made_tables::service::clear_rows;
use crate::domain::made_tables::types::{GroupRoom, TableRows};
use crate::domain::stages::types::{BatchRow, FanRows};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{Count, Integer};

/// How many rows a batch holds in the workloads: what a pipeline hands from stage to stage, the fill side's one size.
pub(crate) const BATCH_ROWS: usize = crate::domain::made_tables::constants::BATCH_ROWS.0;

/// The high half of a node's id in the workloads: its file.
const FILE_HALF: u64 = 7 << 32;

/// How many rows one key of a grouped workload has.
const GROUP_ROWS: usize = 8;

/// One loop the bench times.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceKernel {
    /// `Scan`: the handles of a source's rows.
    Scan,
    /// `Read`: one column of a source by the batch's handles.
    Read,
    /// `Keep` of a column against a constant, and the kept rows of one column moved up.
    KeepConstant,
    /// `Keep` of two columns compared, and the kept rows of one column moved up.
    KeepColumns,
    /// `Probe` in rows mode, each key holding one row, and one column gathered.
    ProbeRows,
    /// `Probe` in rows mode over a column of ids in order, each key holding one row, and one column gathered.
    ProbeIds,
    /// `Probe` in present mode, half the keys held.
    ProbePresent,
    /// `Probe` in absent mode, half the keys held.
    ProbeAbsent,
    /// `Product`: per pair given.
    Product,
    /// `Latest`: one search per probe in a group of eight.
    Latest,
    /// An index built by one column.
    IndexOne,
    /// An index built by one column of ids in order.
    IndexIds,
    /// An index built by two columns as one key.
    IndexTwo,
    /// An index built by one column, each key's rows sorted by another.
    IndexOrdered,
    /// An index built by one column of 43 different codes, many rows a code.
    IndexCodes,
    /// An index built by one column whose cells lie far apart: every row its own key, a million apart.
    IndexFar,
    /// An index over no row, built and then asked once for every row of the workload.
    ProbeEmpty,
    /// The table's write, three columns, every row kept.
    Write,
    /// The table's write with repeats dropped, three columns, half the rows repeated.
    WriteNew,
    /// A rows fill: a scan, a read, a probe and a write.
    FillRows,
    /// A counted grouping, eight rows a key.
    FillGroupCount,
    /// A sorted table, eight rows a key.
    FillSorted,
}

/// Every loop the bench times, in the order of its table.
pub const KERNELS: &[ReferenceKernel] = &[
    ReferenceKernel::Scan,
    ReferenceKernel::Read,
    ReferenceKernel::KeepConstant,
    ReferenceKernel::KeepColumns,
    ReferenceKernel::ProbeRows,
    ReferenceKernel::ProbeIds,
    ReferenceKernel::ProbePresent,
    ReferenceKernel::ProbeAbsent,
    ReferenceKernel::Product,
    ReferenceKernel::Latest,
    ReferenceKernel::IndexOne,
    ReferenceKernel::IndexIds,
    ReferenceKernel::IndexTwo,
    ReferenceKernel::IndexOrdered,
    ReferenceKernel::IndexCodes,
    ReferenceKernel::IndexFar,
    ReferenceKernel::ProbeEmpty,
    ReferenceKernel::Write,
    ReferenceKernel::WriteNew,
    ReferenceKernel::FillRows,
    ReferenceKernel::FillGroupCount,
    ReferenceKernel::FillSorted,
];

/// The name a loop's measure is printed under.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::stages::fixtures::workloads::{ReferenceKernel, kernel_name};
///
/// assert_eq!(kernel_name(ReferenceKernel::Scan), "scan");
/// ```
#[must_use]
pub const fn kernel_name(kernel: ReferenceKernel) -> &'static str {
    match kernel {
        ReferenceKernel::Scan => "scan",
        ReferenceKernel::Read => "read",
        ReferenceKernel::KeepConstant => "keep_constant",
        ReferenceKernel::KeepColumns => "keep_columns",
        ReferenceKernel::ProbeRows => "probe_rows",
        ReferenceKernel::ProbeIds => "probe_rows_ids_in_order",
        ReferenceKernel::ProbePresent => "probe_present",
        ReferenceKernel::ProbeAbsent => "probe_absent",
        ReferenceKernel::Product => "product",
        ReferenceKernel::Latest => "latest",
        ReferenceKernel::IndexOne => "index_one_column",
        ReferenceKernel::IndexIds => "index_ids_in_order",
        ReferenceKernel::IndexTwo => "index_two_columns",
        ReferenceKernel::IndexOrdered => "index_ordered",
        ReferenceKernel::IndexCodes => "index_codes",
        ReferenceKernel::IndexFar => "index_cells_far_apart",
        ReferenceKernel::ProbeEmpty => "probe_index_of_no_row",
        ReferenceKernel::Write => "write",
        ReferenceKernel::WriteNew => "write_repeats_dropped",
        ReferenceKernel::FillRows => "fill_rows",
        ReferenceKernel::FillGroupCount => "fill_group_count",
        ReferenceKernel::FillSorted => "fill_sorted",
    }
}

/// A loop's synthetic input and the room it runs in, made once and run many times.
#[derive(Debug, Default)]
pub struct KernelWorkload {
    /// How many rows the loop reads.
    pub(crate) rows: usize,
    /// The source: a key column in which every row has its own key, in a scattered order.
    pub(crate) unique: Vec<CellWord>,
    /// The source: a key column of ids in order, as a file's nodes are: the file in the high half, the place in the low.
    pub(crate) ids: Vec<CellWord>,
    /// The source: a key column of eight rows a key.
    pub(crate) grouped: Vec<CellWord>,
    /// The source: a column of small numbers, 43 different.
    pub(crate) small: Vec<CellWord>,
    /// The source: a key column in which every row has its own key, the keys a million apart.
    pub(crate) far: Vec<CellWord>,
    /// The probes: a column whose every cell is a key of `unique`, and one of which half are.
    pub(crate) probes: [Vec<CellWord>; 2],
    /// The index the probes read.
    pub(crate) index: IndexArena,
    /// The batch: its handles, a column read, a selection, and the rows a loop gave.
    pub(crate) batch: (Vec<CellWord>, Vec<CellWord>, Vec<BatchRow>, FanRows),
    /// A table written, and the input of a grouping fill.
    pub(crate) tables: [TableRows; 2],
    /// The room of a grouping fill.
    pub(crate) room: GroupRoom,
}

/// A cell holding a number.
fn cell(number: usize) -> CellWord {
    CellWord(u64::try_from(number).unwrap_or(u64::MAX))
}

/// The workload of `kernel` over `rows` rows.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::stages::fixtures::run_kernel;
/// use ryo_engine::domain::stages::fixtures::workloads::{ReferenceKernel, workload};
///
/// let mut scan = workload(ReferenceKernel::Scan, 100);
/// assert_eq!(run_kernel(ReferenceKernel::Scan, &mut scan), 100);
/// ```
#[must_use]
pub fn workload(kernel: ReferenceKernel, rows: usize) -> KernelWorkload {
    let scattered = |row: usize, step: usize, among: usize| row.wrapping_mul(step) % among.max(1);
    let mut made = KernelWorkload {
        rows,
        unique: (0..rows)
            .map(|row| cell(scattered(row, 7919, rows)))
            .collect(),
        ids: (0..rows)
            .map(|row| CellWord(cell(row).0 | FILE_HALF))
            .collect(),
        grouped: (0..rows).map(|row| cell(row / GROUP_ROWS)).collect(),
        small: (0..rows).map(|row| cell(scattered(row, 31, 43))).collect(),
        far: (0..rows)
            .map(|row| cell(scattered(row, 7919, rows).wrapping_mul(1_000_003)))
            .collect(),
        probes: [
            (0..rows)
                .map(|row| cell(scattered(row, 104_729, rows)))
                .collect(),
            (0..rows)
                .map(|row| cell(scattered(row, 104_729, rows.saturating_mul(2))))
                .collect(),
        ],
        ..KernelWorkload::default()
    };
    made.batch.3.limit = Count(BATCH_ROWS);
    let keys = if kernel == ReferenceKernel::Latest {
        &made.grouped
    } else if kernel == ReferenceKernel::ProbeIds {
        &made.ids
    } else {
        &made.unique
    };
    begin(&mut made.index, Count(rows), Count(1));
    key_column(&mut made.index, Count(0), plain(keys));
    finish(&mut made.index, IndexSite(0), FileStamp(0), None);
    // The input of the grouping fills, and the sorted table `Latest` reads: a key of eight rows, an order, a payload.
    let [_, input] = &mut made.tables;
    clear_rows(input, Count(3));
    for row in 0..rows {
        input.cells.extend([
            cell(row / GROUP_ROWS),
            cell(scattered(row, 31, 43)),
            cell(row),
        ]);
    }
    input.count = Count(rows);
    input.orders.extend(
        (0..rows)
            .map(|row| Integer(i64::try_from((row % GROUP_ROWS).wrapping_mul(10)).unwrap_or(0))),
    );
    made
}

/// A column that lies cell after cell.
pub(crate) const fn plain(cells: &[CellWord]) -> ColumnView<'_> {
    ColumnView {
        cells,
        first: Count(0),
        stride: Count(1),
    }
}
