//! The test support of the `table_model` concept: the rows its reads have handed out on this thread.

thread_local! {
    /// Actual full-row reads for same-binary kernel differential tests only.
    static READ_ROW_COUNTS: std::cell::Cell<(u64, u64)> = const { std::cell::Cell::new((0, 0)) };
}

/// Records one full-row read of `cells` cells.
pub(crate) fn note_read_row(cells: usize) {
    READ_ROW_COUNTS.with(|counts| {
        let (calls, total) = counts.get();
        counts.set((calls + 1, total + cells as u64));
    });
}

pub(crate) fn read_row_counts() -> (u64, u64) {
    READ_ROW_COUNTS.with(std::cell::Cell::get)
}

pub(crate) fn take_read_row_counts() -> (u64, u64) {
    READ_ROW_COUNTS.with(|counts| counts.replace((0, 0)))
}
