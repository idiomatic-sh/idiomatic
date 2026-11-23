//! File facts supplied by the host during the walk.
use ryo::languages::{CellWord, FilledColumn, FilledColumns, Relation};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

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
    fill_path(rel, Path::new(&path), file, out)
}

pub(super) fn fill_path(
    rel: Relation,
    path: &Path,
    file: usize,
    out: &mut FilledColumns,
) -> Option<bool> {
    if rel != Relation::FilesystemExecutable {
        return None;
    }
    let Ok(value) = executable(path) else {
        return Some(false);
    };
    out.0 = vec![
        FilledColumn(vec![CellWord(file as u64)]),
        FilledColumn(vec![CellWord(u64::from(value))]),
    ];
    Some(true)
}
