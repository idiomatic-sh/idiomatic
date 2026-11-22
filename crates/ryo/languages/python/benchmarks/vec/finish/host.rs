//! THROWAWAY (redo-honest): the host's own rows of a file, as the shipping engine's host writes them
//! (core `file_analysis::engine_source`): `filesystem.executable`, the file's POSIX executable bit (any of the mode's
//! three execute bits, as core's corpus adapter reads it), and `filesystem.read-failure`, a failed read of the file (none:
//! a file the harness lints was read). Not the language provider's: it says what the file system says.

use super::{R, Run, Table};
use std::os::unix::fs::PermissionsExt as _;

/// The host's rows of `rel` for file `file_no`, or `None` when `rel` is not a host relation.
pub(in super::super) fn host_fill(run: &mut Run, rel: &str, file_no: u32) -> Option<R<Table>> {
    if rel != "filesystem.executable" && rel != "filesystem.read-failure" {
        return None;
    }
    let (_, names, tys) = run.catalog(rel)?;
    let mut t = Table::with(names, tys);
    if rel == "filesystem.executable" {
        let path = format!("{}/{}", run.root_dir, run.paths[file_no as usize]);
        match std::fs::metadata(&path) {
            Ok(m) => t.push_row(&[u64::from(file_no), u64::from(m.permissions().mode() & 0o111 != 0)]),
            Err(e) => return Some(Err(format!("filesystem.executable: {path}: {e}"))),
        }
    }
    Some(Ok(t))
}
