//! The interpreter's shared-walk counters of one file session.
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;

/// The file's walk made `passes` passes over `rows` source rows.
pub(crate) fn counts(file: &FileSession<'_, '_>, passes: u64, rows: u64) {
    let actual = file.fixture_walk_counts();
    assert_eq!(actual.passes.0, passes);
    assert_eq!(actual.source_rows.0, rows);
}
