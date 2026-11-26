//! Proves the skips of the languages this build links are what the walks read: the languages' own declarations, not a list a test hands in.

/// Dependency, environment and build directories at the root and nested, and two files in the source directory; no language reads a `.txt`
/// file, so each one a run walks is listed unread.
const FILES: [&str; 7] = [
    "node_modules/x.txt",
    "pkg/node_modules/v.txt",
    ".venv/lib/y.txt",
    "target/z.txt",
    "pkg/target/w.txt",
    "src/a.txt",
    "src/b.txt",
];

/// `query run` does not go into the directories the linked languages declare.
#[test]
fn the_walk_does_not_enter_the_directories_the_linked_languages_declare() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    for file in FILES {
        let path = directory.path().join(file);
        std::fs::create_dir_all(path.parent().expect("a directory")).expect("a directory");
        std::fs::write(path, "x").expect("a file");
    }
    let request = ryo_sdk::query::RunRequest {
        root: directory.path().to_path_buf(),
        queries: vec![
            ryo_sdk::query::RyoQlQuery::try_from(
                "let functions = select * from \"function\"; emit functions subject \"file\" key(\"span\");",
            )
            .expect("a query"),
        ],
        limit: ryo_sdk::query::RowLimit(20),
    };
    let report = ryo_sdk::query::run(
        crate::languages::linked_sdk(),
        &ryo_sdk::linter::WorkerPool::from(ryo_sdk::linter::Workers(std::num::NonZeroUsize::MIN)),
        ryo_sdk::linter::MemoryBudget::Machine,
        request,
    )
    .expect("a run");
    let mut found = report.unread.iter().map(|file| file.path.0.to_string()).collect::<Vec<_>>();
    found.sort();
    assert_eq!(found, ["src/a.txt", "src/b.txt"]);
}

/// The linked set declares the directories of the languages it links.
#[test]
fn the_linked_languages_declare_their_directories() {
    let declared = crate::languages::skipped().0;
    for name in ["node_modules", ".venv", "target"] {
        assert!(declared.iter().any(|skip| &**skip == name), "{name}");
    }
}
