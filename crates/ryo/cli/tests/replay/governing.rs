//! What a rule that governs no file, and a file the manifests leave unexplained, are, over small projects made for the cases.

use crate::base;

use super::manifests::workspace;

/// A rule whose include matches no file reads no rows of any relation of the files, and still gives its answer: none. The steps after
/// the files are given inputs with no rows and no columns, and a join of them must not ask for a column they lack.
#[test]
fn a_rule_whose_include_matches_no_file_gives_no_finding_and_no_error() {
    let _beside = crate::beside();
    let root = workspace();
    // Steps that read the tree of a file, joined across files by the file they are in.
    let query = base::rules::query("workspace-completeness");
    let across = "let listed = project(from=\"filesystem.file\", columns=[\"path\"]);
let parts = capture(from=listed, path=\"path\", pattern=\"{*dir}/x.rs\", captures=[\"dir\"]);
let dirs = project(from=parts, columns=[\"dir\"]);
let missing = antijoin(left=dirs, right=dirs, left-on=[\"dir\"], right-on=[\"dir\"]);
emit missing subject dir key(dir);";
    let rule_across =
        crate::tools::rule_from_query("across", across, &["nothing-here/**".to_owned()]);
    let rule = crate::tools::rule_from_query(
        "workspace-completeness",
        &query,
        &["nothing-here/**".to_owned()],
    );
    let (rows, _) = base::lint_with(
        root.path(),
        vec![rule, rule_across, crate::tools::rule_from_query("every", "let a = select * from \"syntax-tree\" where kind = \"fn\"; emit a subject file key(id);", &["**".to_owned()])],
        ryo_sdk::linter::Cache::None,
    )
    .expect("a rule that governs no file runs");
    assert!(rows.iter().all(|row| row[0] == "every"), "{rows:?}");
}

/// A module declared without a file is found where it is: the relations that say which files are examined are read only in part
/// there, and a rule that trusts their absence of rows cannot decide for that file. A file no crate root reaches is no doubt: it
/// is known not to be examined, so a rule decides it (`orphan-rust-files` reports it).
#[test]
fn a_module_without_a_file_is_unknown_at_its_file_and_a_file_no_crate_root_reaches_is_not() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    let write = |path: &str, text: &str| {
        let at = root.path().join(path);
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("a directory");
        std::fs::write(at, text).expect("a file");
    };
    write(
        "Cargo.toml",
        "[package]\nname = \"p\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[[bench]]\nname = \"performance\"\npath = \"benches/performance.rs\"\nharness = false\n",
    );
    write("src/lib.rs", "mod tests;\n");
    write("benches/performance.rs", "fn main() {}\n");
    write("benches/performance/adapter.rs", "pub fn run() {}\n");
    let query = "let listed = project(from=\"filesystem.file\", columns=[\"path\"]);\nlet seen = project(from=\"source-dependency\", columns=[\"source\"]);\nlet missing = antijoin(left=listed, right=seen, left-on=[\"path\"], right-on=[\"source\"]);\nemit missing subject path key(path);";
    let rule = crate::tools::rule_from_query("examined", query, &["**".to_owned()]);
    let (_, report) = base::lint_report(
        root.path(),
        vec![rule],
        ryo_sdk::linter::Cache::None,
    )
    .expect("the rule runs");
    let mut places: Vec<String> = report
        .unknowns
        .iter()
        .map(|each| format!("{}: {}", each.path.0, each.message.0))
        .collect();
    places.sort();
    assert!(
        !places
            .iter()
            .any(|each| each.starts_with("benches/performance/adapter.rs:")),
        "a file no crate root reaches is no unknown: {places:#?}"
    );
    assert!(
        places
            .iter()
            .any(|each| each.starts_with("src/lib.rs:") && each.contains("declares module `tests`")),
        "{places:#?}"
    );
}

