//! THROWAWAY (redo-honest): the host's own rows of a file, as the shipping engine's host writes them
//! (core `file_analysis::engine_source`): `filesystem.executable`, the file's POSIX executable bit (any of the mode's
//! three execute bits, as core's corpus adapter reads it), and `filesystem.read-failure`, a failed read of the file (none:
//! a file the harness lints was read). Not the language provider's: it says what the file system says.

use super::{R, Run, Table};
use super::super::host_columns;

/// The only row of this relation in a file-local execution identifies this file.
pub(in super::super) const CURRENT_FILE: (&str, &str) = ("filesystem.file", "path");

/// redo-fills pass 2: how many rows the host has of `rel` in a file without reading it: a file the harness lints was read,
/// so it has no read failure. None: not known (or not the host's).
pub(in super::super) fn host_len(rel: &str) -> Option<usize> {
    match rel {
        "filesystem.read-failure" => Some(0),
        // redo-binding c32 (merged by redo-int): the current file's one row
        name if name == CURRENT_FILE.0 => Some(1),
        _ => None,
    }
}

/// The host's rows of `rel` for file `file_no`, or `None` when `rel` is not a host relation.
pub(in super::super) fn host_fill(run: &mut Run, rel: &str, file_no: u32) -> Option<R<Table>> {
    if rel != "filesystem.executable" && rel != "filesystem.read-failure" && rel != CURRENT_FILE.0 {
        return None;
    }
    let (_, names, tys) = run.catalog(rel)?;
    let mut t = Table::with(names, tys);
    if rel == CURRENT_FILE.0 {
        // redo-binding c32 (merged by redo-int): the current file's row
        t.push_row(&[u64::from(file_no)]);
        return Some(Ok(t));
    }
    if rel == "filesystem.executable" {
        if let Some((id, permission)) = run.supplied_permission.filter(|&(id, _)| id == file_no) {
            use ryo_values::domain::files::types::ExecutablePermission::*;
            match permission {
                PosixExecutable | PosixNotExecutable => t.push_row(&[u64::from(id), u64::from(permission == PosixExecutable)]),
                // Unknown metadata is also reported as partial by the input edge.
                Unsupported | Unknown => {}
            }
            return Some(Ok(t));
        }
        let path = format!("{}/{}", run.root_dir, run.paths[file_no as usize]);
        match host_columns::executable(std::path::Path::new(&path)) {
            Ok(value) => t.push_row(&[u64::from(file_no), u64::from(value)]),
            Err(e) => return Some(Err(format!("filesystem.executable: {path}: {e}"))),
        }
    }
    Some(Ok(t))
}
