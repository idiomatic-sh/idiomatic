//! The interpreter's shared-walk counters of one file session.
use ryo_execution::inbound::use_cases::execution::interface::FileSession;

/// How many passes the file's walk made, over how many source rows.
pub(crate) fn walked(file: &FileSession<'_, '_>) -> (u64, u64) {
    let actual = file.fixture_walk_counts();
    (actual.passes.0, actual.source_rows.0)
}
