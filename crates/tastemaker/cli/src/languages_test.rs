//! Proves the skips of the languages this build links are what the walks read: the languages' own declarations, not a list a test hands in.

use tastemaker_core::domain::files::types::MachinePath;
use tastemaker_core::domain::paths::types::RepoPath;
use tastemaker_core::outbound::repository::factory::disk;
use tastemaker_core::outbound::repository::interface::RepositoryPort as _;

/// Dependency, environment and build directories at the root and nested, and one source file.
const FILES: [&str; 7] = [
    "node_modules/x.js",
    "pkg/node_modules/v.js",
    ".venv/lib/y.py",
    "target/z.rs",
    "pkg/target/w.rs",
    "src/a.ts",
    "src/lib.py",
];

/// The line `explain` prints for the directory `name`, over a workspace on disk, composed as the command composes it.
fn listed(name: &str) -> Option<String> {
    use tastemaker_core::inbound::use_cases::explain::interface::ExplainUseCase as _;
    use tastemaker_core::inbound::use_cases::explain::types::{Asked, ExplainRequest};

    let directory = crate::resources::workspace::check::fixtures::workspace();
    let directory_edge = crate::resources::workspace::check::fixtures::edge(&directory);
    let report = crate::resources::workspace::explain::composition::build(&directory_edge.store)
        .explain(ExplainRequest {
            start: MachinePath(directory.path().display().to_string().into()),
            asked: Asked::Workspace,
        })
        .ok()?;
    let prefix = format!("- `{name}`: ");
    report
        .sections
        .into_iter()
        .flat_map(|section| section.lines)
        .map(|line| line.0.to_string())
        .find(|line| line.starts_with(&prefix))
}

/// `explain` lists a directory with the languages that declare it, from the linked set.
#[test]
fn the_linked_languages_are_listed_with_the_directories_they_declare() {
    let node = listed("node_modules").expect("node_modules is listed");
    assert!(
        node.contains("javascript") && node.contains("typescript"),
        "{node}"
    );
    assert!(listed(".venv").is_some_and(|line| line.contains("python")));
    assert!(listed("target").is_some_and(|line| line.contains("rust")));
}

/// A walk of the repository, built as the commands build it, does not go into the directories the linked languages declare.
#[test]
fn the_walk_does_not_enter_the_directories_the_linked_languages_declare() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    for file in FILES {
        let path = directory.path().join(file);
        std::fs::create_dir_all(path.parent().expect("a directory")).expect("a directory");
        std::fs::write(path, "x").expect("a file");
    }
    let root = MachinePath(directory.path().display().to_string().into());
    let found = disk(
        crate::languages::skipped(),
        crate::resources::workspace::check::fixtures::edge(&directory).store,
    )
    .open(&root)
    .list(&RepoPath::default())
    .expect("a listing")
    .0
    .into_iter()
    .map(|path| path.0.to_string())
    .collect::<Vec<_>>();
    assert_eq!(found, ["src/a.ts", "src/lib.py"]);
}
