//! A target's checkout is fetched into the shared cache once, however many test processes ask for it at the same time: the
//! release check runs the default tests and the coverage run side by side, and both fetch ruff.

use crate::base::snapshot::fetched;
use std::path::Path;
use std::process::Command;

/// Runs git with `arguments` in `directory`, as a committer of its own, and what it prints.
fn git(directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=ryo",
            "-c",
            "user.email=ryo@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("git writes UTF-8")
}

#[test]
fn two_fetches_of_one_target_at_once_both_find_its_checkout() {
    let origin = tempfile::tempdir().expect("a directory for the repository");
    std::fs::write(
        origin.path().join("pyproject.toml"),
        "[project]\nname = \"target\"\n",
    )
    .expect("a file to commit");
    for arguments in [
        &["init", "--quiet"][..],
        &["add", "pyproject.toml"],
        &["commit", "--quiet", "--message", "the target"],
    ] {
        assert_eq!(
            git(origin.path(), arguments),
            "",
            "git {arguments:?} prints nothing"
        );
    }
    let revision = git(origin.path(), &["rev-parse", "HEAD"])
        .trim_end()
        .to_owned();
    let repository = origin.path().to_str().expect("a UTF-8 path").to_owned();
    let cache = tempfile::tempdir().expect("a cache directory");

    let checkouts: Vec<_> = std::thread::scope(|scope| {
        let fetches = [
            scope.spawn(|| fetched(cache.path(), "target", &repository, &revision)),
            scope.spawn(|| fetched(cache.path(), "target", &repository, &revision)),
        ];
        fetches
            .into_iter()
            .map(|fetch| fetch.join().expect("each fetch finishes"))
            .collect()
    });

    assert_eq!(checkouts[0], checkouts[1]);
    assert_eq!(
        git(&checkouts[0], &["rev-parse", "HEAD"]).trim_end(),
        revision
    );
    assert!(checkouts[0].join("pyproject.toml").is_file());
}
