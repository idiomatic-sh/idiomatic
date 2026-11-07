//! Source-derived inventories: the owner's consumer-visible surface, the production files outside
//! the owner, and the test-only module trees that are not production.
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

pub(super) const OWNER_TREE: &str = "application/file_dag/derived/owner";
pub(super) const OWNER_FILE: &str = "application/file_dag/derived/owner/mod.rs";

fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}
pub(super) fn files(src: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(src, &mut found);
    found.sort();
    found
}
pub(super) fn relative(src: &Path, path: &Path) -> String {
    path.strip_prefix(src)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}
fn is_test_gate(line: &str) -> bool {
    line.contains("cfg(test)") || line.contains("cfg(any(test")
}
/// Module files and directories declared behind a test-only `cfg`.
fn test_only(src: &Path, files: &[PathBuf]) -> BTreeSet<String> {
    let mut gated = BTreeSet::new();
    for path in files {
        let text = fs::read_to_string(path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let name = path.file_name().unwrap().to_string_lossy();
        let base = if name == "mod.rs" || name == "lib.rs" {
            path.parent().unwrap().to_path_buf()
        } else {
            path.with_extension("")
        };
        for (at, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let Some(declared) = trimmed
                .split("mod ")
                .nth(1)
                .filter(|_| trimmed.ends_with(';') && !trimmed.contains('{'))
                .filter(|_| trimmed.starts_with("mod ") || trimmed.starts_with("pub"))
            else {
                continue;
            };
            let name = declared.trim_end_matches(';').trim();
            let mut back = at;
            let mut gate = false;
            while back > 0 && lines[back - 1].trim().starts_with("#[") {
                back -= 1;
                gate |= is_test_gate(lines[back]);
            }
            if gate {
                gated.insert(relative(src, &base.join(format!("{name}.rs"))));
                gated.insert(relative(src, &base.join(name)));
            }
        }
    }
    gated
}
fn is_test_path(path: &str, gated: &BTreeSet<String>) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    gated
        .iter()
        .any(|gate| path == gate || path.starts_with(&format!("{gate}/")))
        || path.ends_with("test.rs")
        || path.ends_with("tests.rs")
        || segments.iter().any(|segment| {
            matches!(
                *segment,
                "fixtures" | "testing" | "family" | "test" | "tests"
            )
        })
}
/// Every non-test source file, relative to `src`.
pub(super) fn production(src: &Path) -> Vec<String> {
    let all = files(src);
    let gated = test_only(src, &all);
    all.iter()
        .map(|path| relative(src, path))
        .filter(|path| !is_test_path(path, &gated))
        .collect()
}
/// The `(file, name)` of every consumer-visible function in the owner tree: `pub(in
/// crate::application::file_dag)` or `pub(crate)`, outside `#[cfg(test)]`.
pub(super) fn owner_surface(src: &Path, production: &[String]) -> BTreeSet<(String, String)> {
    let mut found = BTreeSet::new();
    for file in production {
        if !(file == OWNER_FILE || file.starts_with(&format!("{OWNER_TREE}/"))) {
            continue;
        }
        let text = fs::read_to_string(src.join(file)).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            let rest = trimmed
                .strip_prefix("pub(in crate::application::file_dag) ")
                .or_else(|| trimmed.strip_prefix("pub(crate) "));
            let Some(rest) = rest else { continue };
            let rest = rest.strip_prefix("const ").unwrap_or(rest);
            let Some(rest) = rest.strip_prefix("fn ") else {
                continue;
            };
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            let mut back = at;
            let mut gate = false;
            while back > 0 {
                let above = lines[back - 1].trim();
                if above.starts_with("#[") || above.starts_with("///") || above.is_empty() {
                    gate |= above.contains("#[cfg(test)]");
                    back -= 1;
                } else {
                    break;
                }
            }
            if !gate {
                found.insert((file.clone(), name));
            }
        }
    }
    found
}
/// The pubs with an existing `fn <name>` in `file`.
pub(super) fn declares(src: &Path, file: &str, name: &str) -> bool {
    fs::read_to_string(src.join(file)).is_ok_and(|text| text.contains(&format!("fn {name}(")))
}
