//! The trees the store tests write: files written under a root, and rows in one order.

use std::path::Path;

pub(crate) fn write(root: &Path, files: &[(&str, String)]) {
    for (path, text) in files {
        let place = root.join(path);
        std::fs::create_dir_all(place.parent().expect("a directory")).expect("a directory");
        std::fs::write(place, text).expect("a file");
    }
}

pub(crate) fn sorted(mut rows: Vec<Vec<String>>) -> Vec<Vec<String>> {
    rows.sort();
    rows
}
