//! What one worker's buffers count while the run cases read them: the indexes built, the chunks handed on, the node tables
//! held and the work the ends of files did.

/// The work the releases and the ends of a worker's files did: each node released, each shared index it visited.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct EndWork(pub(crate) usize);

/// How many probe indexes a worker built.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct IndexBuilds(pub(crate) usize);

/// The node tables a worker held: how many it formed, the rows they hold now, and the most rows they held at once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TableMeter {
    /// How many node tables were held.
    pub(crate) formed: TablesFormed,
    /// The rows the held node tables hold now.
    pub(crate) live_rows: ryo_values::domain::rows::types::RowCount,
    /// The most rows the held node tables held at once.
    pub(crate) peak_rows: ryo_values::domain::rows::types::RowCount,
}

/// How many node tables a worker held.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TablesFormed(pub(crate) u32);

impl TableMeter {
    /// A node table of `rows` rows is held.
    pub(crate) fn formed(&mut self, rows: ryo_values::domain::rows::types::RowCount) {
        self.formed.0 += 1;
        self.live_rows.0 += rows.0;
        self.peak_rows.0 = self.peak_rows.0.max(self.live_rows.0);
    }

    /// A held node table of `rows` rows is dropped.
    pub(crate) fn dropped(&mut self, rows: ryo_values::domain::rows::types::RowCount) {
        self.live_rows.0 = self.live_rows.0.saturating_sub(rows.0);
    }
}

/// Whether every chunk a worker's chains handed on held less than the bound before the input row that wrote its last rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ChunkMeter {
    /// Every chunk did.
    #[default]
    Within,
    /// A chunk held the bound's cells or more before it.
    Over,
}
