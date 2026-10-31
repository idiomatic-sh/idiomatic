//! Real local git repositories prove immutable pins and non-destructive restoration.
use crate::domain::suite::types::Project;
use crate::outbound::workspace::interface::WorkspacePort;
use std::{fs, path::Path, process::Command};
fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git available");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf8")
        .trim()
        .into()
}
fn pinned_clone_patch_restore_and_intervening_edit_refusal() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let source = temporary.path().join("source");
    fs::create_dir(&source).expect("source");
    drop(git(&source, &["init", "--quiet"]));
    fs::write(source.join("input.txt"), "before\n").expect("input");
    drop(git(&source, &["add", "input.txt"]));
    drop(git(
        &source,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "initial",
        ],
    ));
    let project = Project {
        name: "fixture".into(),
        repository: source.to_string_lossy().into_owned().into(),
        revision: git(&source, &["rev-parse", "HEAD"]).into(),
        patch: "edit.patch".into(),
    };
    let cache = temporary.path().join("cache");
    let adapter = crate::outbound::workspace::factory::git(cache);
    let mut workspace = adapter.prepare(&project).expect("pinned clone");
    assert_eq!(
        adapter.tracked_files(&workspace).expect("files").0,
        ["input.txt"]
    );
    let patch = temporary.path().join("edit.patch");
    fs::write(&patch, "diff --git a/input.txt b/input.txt\n--- a/input.txt\n+++ b/input.txt\n@@ -1 +1 @@\n-before\n+after\n").expect("patch");
    adapter.apply_patch(&mut workspace, &patch).expect("apply");
    assert_eq!(
        fs::read_to_string(workspace.root().join("input.txt")).expect("read"),
        "after\n"
    );
    assert_eq!(
        adapter
            .prepare(&project)
            .expect_err("dirty clone")
            .to_string(),
        "corpus is dirty; refusing to overwrite changes"
    );
    fs::write(workspace.root().join("input.txt"), "user edit\n").expect("edit");
    assert_eq!(
        adapter
            .reset(&mut workspace)
            .expect_err("intervening edit")
            .to_string(),
        "corpus changed after patch; refusing restore"
    );
    assert_eq!(
        fs::read_to_string(workspace.root().join("input.txt")).expect("read"),
        "user edit\n"
    );
    fs::write(workspace.root().join("input.txt"), "after\n").expect("restore patch");
    adapter.reset(&mut workspace).expect("restore clean");
    assert_eq!(
        fs::read_to_string(workspace.root().join("input.txt")).expect("read"),
        "before\n"
    );
    drop(adapter.prepare(&project).expect("clean reuse"));
    let invalid = Project {
        revision: "main".into(),
        ..project
    };
    assert_eq!(
        adapter
            .prepare(&invalid)
            .expect_err("mutable pin")
            .to_string(),
        "project requires a safe name and full commit revision"
    );
}

fn contract() {
    let cache = tempfile::tempdir().expect("isolated cache");
    let adapter = crate::outbound::workspace::factory::git(cache.path().to_owned());
    crate::outbound::workspace::fixtures::contract::check(&adapter);
}

use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("pins and edits preserve user changes"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    contract();
    pinned_clone_patch_restore_and_intervening_edit_refusal();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
