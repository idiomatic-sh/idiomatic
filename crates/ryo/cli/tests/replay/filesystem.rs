//! `filesystem.file` has a row for each file the run inventories, whatever kind of file it is: a rule
//! that asks for it is answered for a Markdown file no linked language reads.

use crate::base;
use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::types::Rule;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::runs::types::RuleId;

/// The pool every run of these cases reads files on, built once for each run as a caller builds it.
fn pool() -> ryo_sdk::linter::WorkerPool {
    ryo_sdk::linter::WorkerPool::from(ryo_sdk::linter::Workers(
        std::num::NonZeroUsize::MIN.saturating_add(1),
    ))
}

#[test]
fn every_inventoried_file_is_a_row_whatever_it_holds() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    std::fs::write(root.path().join("lib.rs"), "fn run() {}\n").expect("a file");
    std::fs::write(root.path().join("notes.md"), "# notes\n").expect("a file");
    let rule = Rule {
        observations: Vec::new(),
        id: RuleId("all-files".into()),
        query: ryo_values::domain::program::types::QueryBundle::from(QueryText(
            "let files = select * from \"filesystem.file\"; emit files subject path key(path);"
                .into(),
        )),
        applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
        parameters: Vec::new(),
        include: vec![Glob("**".into())],
        exclude: Vec::new(),
    };
    let (rows, _) = base::lint_with(
        root.path(),
        vec![rule],
        Arc::new(ryo_core::outbound::facts::factory::memory()),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    )
    .expect("the rule runs");
    let mut paths: Vec<&str> = rows
        .iter()
        .filter_map(|row| row.get(1).map(String::as_str))
        .collect();
    paths.sort_unstable();
    assert_eq!(paths, ["lib.rs", "notes.md"]);
}

#[cfg(feature = "rust")]
#[test]
fn public_sdk_cache_tracks_given_source_edits_deletion_and_filters() {
    use ryo_sdk::linter::{
        Cache, CaseSource, Corpus, Glob, Languages, MemoryBudget, RelativePath, SourceText,
    };
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("consumer project");
    let cache = tempfile::tempdir().expect("consumer cache");
    let languages = Languages(vec![ryo_rust::linked()]);
    let source = |name: &str, text: &str| CaseSource {
        executable: Default::default(),
        path: RelativePath(name.into()),
        text: SourceText(text.into()),
    };
    let functions = "let f = select * from \"function\"; emit f subject file key(span);";
    let mut request = super::sdk::sdk_request(
        root.path(),
        functions,
        vec![source("a.rs", "fn old() {}\n")],
    );
    assert_eq!(
        ryo_sdk::linter::lint(
            languages.clone(),
            &pool(),
            MemoryBudget::Machine,
            Cache::At(cache.path().to_path_buf()),
            request.clone(),
        )
        .expect("cold")
        .findings
        .len(),
        1
    );
    request.corpus = Corpus::Given(vec![source("a.rs", "fn new() {}\nfn second() {}\n")]);
    assert_eq!(
        ryo_sdk::linter::lint(
            languages.clone(),
            &pool(),
            MemoryBudget::Machine,
            Cache::At(cache.path().to_path_buf()),
            request.clone(),
        )
        .expect("edit")
        .findings
        .len(),
        2
    );
    request.corpus = Corpus::Given(Vec::new());
    assert!(
        ryo_sdk::linter::lint(
            languages.clone(),
            &pool(),
            MemoryBudget::Machine,
            Cache::At(cache.path().to_path_buf()),
            request.clone(),
        )
        .expect("deletion")
        .findings
        .is_empty()
    );
    request.corpus = Corpus::Given(vec![
        source("a.rs", "fn a() {}"),
        source("b.rs", "fn b() {}"),
    ]);
    request.rules.values_mut().next().expect("rule").exclude =
        vec![Glob::try_from("b.rs").expect("glob")];
    let filtered = ryo_sdk::linter::lint(
        languages.clone(),
        &pool(),
        MemoryBudget::Machine,
        Cache::At(cache.path().to_path_buf()),
        request.clone(),
    )
    .expect("exclude");
    assert_eq!(filtered.findings.len(), 1);
    assert_eq!(
        filtered.findings[0].subject,
        ryo_sdk::linter::Slot::Is(ryo_sdk::linter::Cell::File(RelativePath("a.rs".into())))
    );
    request.files = vec![Glob::try_from("b.rs").expect("glob")];
    assert!(
        ryo_sdk::linter::lint(
            languages,
            &pool(),
            MemoryBudget::Machine,
            Cache::At(cache.path().to_path_buf()),
            request,
        )
        .expect("file selection intersects rule exclusion")
        .findings
        .is_empty()
    );
}

/// A run that keeps a store finds what a run that keeps nothing finds after a file is added below a directory, a file's bytes
/// change, a directory is added and an ignore file ignores a directory.
#[test]
fn a_kept_walk_finds_what_a_whole_walk_finds_after_each_edit() {
    use ryo_sdk::linter::{Cache, Corpus, Languages, MemoryBudget};
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a project");
    let cache = tempfile::tempdir().expect("a cache");
    let write = |path: &str, text: &str| {
        let at = root.path().join(path);
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("a directory");
        std::fs::write(at, text).expect("a file");
    };
    write("src/a.rs", "fn a() {}");
    write("src/deep/b.rs", "fn b() {}");
    write("docs/c.md", "c");
    write(".gitignore", "target/\n");
    write("target/d.rs", "x");
    let files = "let f = select * from \"filesystem.file\"; emit f subject path key(path);";
    let mut request = super::sdk::sdk_request(root.path(), files, Vec::new());
    request.corpus = Corpus::Walked;
    let languages = Languages(vec![ryo_rust::linked()]);
    let paths = |kept_at: Cache| {
        let mut found: Vec<String> = ryo_sdk::linter::lint(
            languages.clone(),
            &pool(),
            MemoryBudget::Machine,
            kept_at,
            request.clone(),
        )
        .expect("a run")
        .findings
        .into_iter()
        .map(|finding| {
            let ryo_sdk::linter::Slot::Is(ryo_sdk::linter::Cell::File(path)) = finding.subject
            else {
                panic!("a finding about a file");
            };
            path.0.to_string()
        })
        .collect();
        found.sort();
        found
    };
    let kept = || Cache::At(cache.path().to_path_buf());
    assert_eq!(paths(kept()), paths(Cache::None));
    let edits: [&dyn Fn(); 4] = [
        &|| write("src/deep/e.rs", "fn e() {}"),
        &|| write("src/a.rs", "fn a() { 1 }"),
        &|| write("new/f.rs", "fn f() {}"),
        &|| write(".gitignore", "target/\ndocs/\n"),
    ];
    for edit in edits {
        // The system's dates move between the runs.
        std::thread::sleep(std::time::Duration::from_millis(20));
        edit();
        assert_eq!(paths(kept()), paths(Cache::None));
    }
    assert!(!paths(kept()).iter().any(|path| path.starts_with("docs")));
}
