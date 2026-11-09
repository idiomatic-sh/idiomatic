//! What a rule that governs no file, a rule's work as a count, and a file the manifests leave unexplained are, over small projects made for the cases.

use crate::base;
use std::fmt::Write as _;
use std::sync::Arc;

use super::manifests::workspace;

/// A rule whose include matches no file reads no rows of any relation of the files, and still gives its answer: none. The steps after
/// the files are given inputs with no rows and no columns, and a join of them must not ask for a column they lack.
#[test]
fn a_rule_whose_include_matches_no_file_gives_no_finding_and_no_error() {
    let _beside = crate::beside();
    let root = workspace();
    // Steps that read the tree of a file, joined across files by the file they are in.
    let query = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/recordings/workspace-completeness/query.ryoql"
    ))
    .expect("the recorded query");
    let across = "let listed = project(from=\"filesystem.file\", columns=[\"path\"]);
let parts = capture(from=listed, path=\"path\", pattern=\"{*dir}/x.rs\", captures=[\"dir\"]);
let dirs = project(from=parts, columns=[\"dir\"]);
let missing = antijoin(left=dirs, right=dirs, left-on=[\"dir\"], right-on=[\"dir\"]);
emit missing subject dir key(dir);";
    let rule_across = crate::rule_from_query("across", across, &["nothing-here/**".to_owned()]);
    let rule = crate::rule_from_query(
        "workspace-completeness",
        &query,
        &["nothing-here/**".to_owned()],
    );
    let (rows, _) = base::lint_with(
        root.path(),
        vec![rule, rule_across, crate::rule_from_query("every", "let a = select * from \"syntax-tree\" where kind = \"fn\"; emit a subject file key(id);", &["**".to_owned()])],
        Arc::new(ryo_execution_runtime::outbound::facts::factory::memory()),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    )
    .expect("a rule that governs no file runs");
    assert!(rows.iter().all(|row| row[0] == "every"), "{rows:?}");
}

/// The work of a rule is a count of rows its steps were fed, the same on any machine at any load: two runs agree, and a rule that
/// governs no file was fed none.
#[test]
fn the_work_of_a_rule_is_a_count_of_rows_per_step() {
    let _alone = crate::alone();
    base::COUNTING.store(true, std::sync::atomic::Ordering::Relaxed);
    let root = workspace();
    let rule = |id: &str, include: &str| {
        crate::rule_from_query(
            id,
            "let a = select * from \"syntax-tree\" where kind = \"fn\"; let b = project(from=a, columns=[\"file\"]); emit b subject file key(file);",
            &[include.to_owned()],
        )
    };
    let run = || {
        let (_, report) = base::lint_report(
            root.path(),
            vec![rule("all", "**"), rule("none", "nothing-here/**")],
            Arc::new(ryo_execution_runtime::outbound::facts::factory::memory()),
            Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
        )
        .expect("the rules run");
        report
            .work
            .iter()
            .map(|each| {
                (
                    each.rule.0.to_string(),
                    each.steps
                        .iter()
                        .map(|step| step.rows.0)
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    };
    let first = run();
    let second = run();
    base::COUNTING.store(false, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(first, second, "the same count on every run");
    assert_eq!(first.len(), 2);
    assert!(first[0].1.iter().sum::<u64>() > 0, "{first:?}");
    assert_eq!(first[1].1.iter().sum::<u64>(), 0, "{first:?}");
}

/// A join that looks the children of a few nodes up in the tree is fed the nodes and the children it finds, not every edge of the tree.
#[test]
fn a_probe_of_the_tree_is_fed_what_it_finds() {
    let _alone = crate::alone();
    base::COUNTING.store(true, std::sync::atomic::Ordering::Relaxed);
    let root = tempfile::tempdir().expect("a directory");
    let many = (0_i32..300_i32).fold(String::new(), |mut text, each| {
        writeln!(text, "const C{each}: u32 = {each};").expect("writing to a String cannot fail");
        text
    });
    std::fs::write(
        root.path().join("lib.rs"),
        format!("{many}pub fn run() {{}}\n"),
    )
    .expect("a file");
    let rule = |id: &str, query: &str| crate::rule_from_query(id, query, &["**/*.rs".to_owned()]);
    let (_, report) = base::lint_report(
        root.path(),
        vec![
            rule(
                "probe",
                "let functions = select l.\"id\" as \"id\" from \"syntax-tree\" as l where l.\"kind\" = \"fn\"; let children = select r.\"child\" as \"child\" from functions as l join \"syntax-child\" as r on l.\"id\" = r.\"parent\"; emit children subject child key(child);",
            ),
            rule(
                "scan",
                "let edges = select * from \"syntax-child\"; emit edges subject child key(child);",
            ),
        ],
        Arc::new(ryo_execution_runtime::outbound::facts::factory::memory()),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    )
    .expect("the rules run");
    base::COUNTING.store(false, std::sync::atomic::Ordering::Relaxed);
    let probe: u64 = report.work[0].steps.iter().map(|step| step.rows.0).sum();
    assert!(
        probe > 0 && probe < 50,
        "the probe was fed {probe} rows of a tree of thousands"
    );
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
    let rule = crate::rule_from_query("examined", query, &["**".to_owned()]);
    let (_, report) = base::lint_report(
        root.path(),
        vec![rule],
        Arc::new(ryo_execution_runtime::outbound::facts::factory::memory()),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
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

/// A word a rule writes for a kind that no linked language has fails the run before a file is read, naming the word and the languages
/// that were looked in: nothing runs until it is fixed.
#[test]
fn a_rule_that_names_a_kind_no_language_has_fails_the_run() {
    let _beside = crate::beside();
    let root = workspace();
    let rule = crate::rule_from_query(
        "misspelled",
        "let a = select * from \"syntax-tree\" where kind = \"function-declaration\"; emit a subject file key(id);",
        &["**".to_owned()],
    );
    let refused = base::lint_with(
        root.path(),
        vec![rule],
        Arc::new(ryo_execution_runtime::outbound::facts::factory::memory()),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    )
    .expect_err("a kind no language has fails the run");
    assert!(
        refused.contains("rule `misspelled`: kind `function-declaration` names no kind of rust"),
        "{refused}"
    );
}

/// A counted run that uses again what the store kept counts its own work only. A changed file it reads alone before the run, for
/// the rules that read across files, is not its work, as before the observer counted the work: the per-file rule's results for the
/// edited file come from that read, so the edit run counts none for it.
#[test]
fn a_counted_run_does_not_count_the_files_it_reads_alone() {
    let _alone = crate::alone();
    base::COUNTING.store(true, std::sync::atomic::Ordering::Relaxed);
    let every = "let a = select * from \"syntax-tree\" where kind = \"fn\"; let b = project(from=a, columns=[\"file\"]); emit b subject file key(file);";
    let across = "let listed = project(from=\"filesystem.file\", columns=[\"path\"]);
let parts = capture(from=listed, path=\"path\", pattern=\"{*dir}/x.rs\", captures=[\"dir\"]);
let dirs = project(from=parts, columns=[\"dir\"]);
let missing = antijoin(left=dirs, right=dirs, left-on=[\"dir\"], right-on=[\"dir\"]);
emit missing subject dir key(dir);";
    let store: Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort> =
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default());
    let work = |root: &std::path::Path| {
        let (_, report) = base::lint_report(
            root,
            vec![
                crate::rule_from_query("every", every, &["**".to_owned()]),
                crate::rule_from_query("across", across, &["**".to_owned()]),
            ],
            Arc::new(ryo_execution_runtime::outbound::facts::factory::memory()),
            Arc::clone(&store),
        )
        .expect("the rules run");
        report
            .work
            .iter()
            .filter(|each| each.rule.0.as_ref() == "every")
            .flat_map(|each| each.steps.iter().map(|step| step.rows.0))
            .sum::<u64>()
    };
    let root = tempfile::tempdir().expect("a directory");
    std::fs::write(root.path().join("a.rs"), "fn a() {}\n").expect("a file");
    std::fs::write(root.path().join("b.rs"), "fn b() {}\n").expect("a file");
    let cold = work(root.path());
    std::fs::write(root.path().join("b.rs"), "fn b() {}\nfn c() {}\n").expect("a file");
    let edit = work(root.path());
    base::COUNTING.store(false, std::sync::atomic::Ordering::Relaxed);
    assert!(cold > 0, "the cold run counts the files it reads");
    assert_eq!(
        edit, 0,
        "the edit run counts nothing of the file it read alone"
    );
}
