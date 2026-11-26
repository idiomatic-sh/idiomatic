//! File facts supplied by the host during the walk.
use crate::languages::{CellWord, FilledColumn, FilledColumns, Relation};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Relations produced from source admission or the filesystem. Catalog ownership
/// alone does not identify the row producer: parser configuration also has an engine schema.
pub(super) fn supplies(rel: Relation) -> bool {
    matches!(rel, Relation::FilesystemFile | Relation::FilesystemExecutable |
        Relation::FilesystemReadFailure | Relation::FilesystemLineCount)
}

pub(super) const FILE_SETS: &[&str] = &["filesystem.executable"];

pub(super) fn executable(path: &Path) -> std::io::Result<bool> {
    Ok(std::fs::metadata(path)?.permissions().mode() & 0o111 != 0)
}

pub(super) fn fill(
    rel: Relation,
    run: Option<&super::finish::Run>,
    file: usize,
    out: &mut FilledColumns,
) -> Option<bool> {
    if rel != Relation::FilesystemExecutable {
        return None;
    }
    let Some(run) = run else { return Some(false) };
    let path = format!("{}/{}", run.root_dir, run.paths[file]);
    let Ok(value) = executable(Path::new(&path)) else {
        return Some(false);
    };
    out.0 = vec![
        FilledColumn(vec![CellWord(file as u64)]),
        FilledColumn(vec![CellWord(u64::from(value))]),
    ];
    Some(true)
}
