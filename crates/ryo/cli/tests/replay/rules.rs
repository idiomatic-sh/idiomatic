//! Which rule a recording replays: its rule's live query, read where `base::rules` finds the rule (a spec rule, a replay rule, or
//! the highest version of a Tastemaker or benchmark rule), never a copy the recording holds, and a replay rule never named like
//! another rule.

use super::base::recording_at;
use super::base::recordings;
use super::base::rules::{self, Roots, latest_version, resolve};
use std::path::Path;

/// The message a panic of `run` carries, when it panics.
fn panic_message(run: impl FnOnce() + std::panic::UnwindSafe) -> Option<String> {
    let payload = std::panic::catch_unwind(run).err()?;
    payload.downcast_ref::<String>().cloned().or_else(|| {
        payload
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned())
    })
}

/// The directories under `root`, at any depth, named `name` and holding a `rule.toml` or a version directory holding one: every
/// definition of the rule there, found by walking the tree, not by `base::rules`.
fn definitions_named(root: &Path, name: &str) -> Vec<std::path::PathBuf> {
    let holds_rule = |directory: &Path| {
        directory.join("rule.toml").is_file()
            || std::fs::read_dir(directory)
                .expect("a directory of the tree")
                .map(|entry| entry.expect("an entry of the tree").path())
                .any(|child| child.join("rule.toml").is_file())
    };
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("a directory of the tree") {
            let path = entry.expect("an entry of the tree").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|each| each == name) && holds_rule(&path) {
                    found.push(path.clone());
                }
                pending.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The three numbers of a `major.minor.patch` directory name, when it is one.
fn numbered(name: &std::ffi::OsStr) -> Option<[u64; 3]> {
    let text = name.to_str()?;
    let parts: Vec<u64> = text
        .split('.')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    <[u64; 3]>::try_from(parts).ok()
}

#[test]
fn every_recording_reads_its_live_rule() {
    let _beside = crate::beside();
    let all = recordings();
    assert_eq!(all.len(), 52, "every recording loads");
    let package = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tastemaker");
    let replay = Roots::here().replay;
    for each in &all {
        let directory = rules::directory(&each.name);
        if directory.starts_with(&replay) {
            // A replay rule is one no Tastemaker package defines, in any spec, rule or benchmark, at any version.
            assert_eq!(
                definitions_named(&package, &each.name),
                Vec::<std::path::PathBuf>::new(),
                "the replay rule `{}` copies a Tastemaker definition",
                each.name
            );
            continue;
        }
        // A packaged rule is read at the highest version of what holds it: the rule's own versions, or its spec's.
        let version = directory
            .ancestors()
            .find(|each| each.file_name().and_then(numbered).is_some())
            .unwrap_or_else(|| panic!("{}: no version above {}", each.name, directory.display()));
        let siblings = std::fs::read_dir(version.parent().expect("a directory of versions"))
            .expect("the versions")
            .map(|entry| entry.expect("a version").file_name());
        let highest = siblings.filter_map(|name| numbered(&name)).max();
        assert_eq!(
            version.file_name().and_then(numbered),
            highest,
            "{} replays {}, not the highest version",
            each.name,
            directory.display()
        );
    }
    let versions = package.join("rules/repeated-string-literals");
    let live: toml::Table = std::fs::read_to_string(latest_version(&versions).join("rule.toml"))
        .expect("the live rule")
        .parse()
        .expect("a rule that reads");
    let replayed = all
        .iter()
        .find(|each| each.name == "repeated-string-literals")
        .expect("the recording of repeated-string-literals");
    assert_eq!(
        Some(replayed.query.as_str()),
        live.get("query").and_then(toml::Value::as_str),
        "repeated-string-literals replays the query of its highest version"
    );
}

#[test]
fn the_latest_version_is_the_highest_by_number() {
    let versions = tempfile::tempdir().expect("a directory of versions");
    for version in ["0.2.0", "0.9.0", "0.10.0"] {
        std::fs::create_dir(versions.path().join(version)).expect("a version");
    }
    assert_eq!(
        latest_version(versions.path()),
        versions.path().join("0.10.0")
    );
    std::fs::create_dir(versions.path().join("next")).expect("a directory that is no version");
    assert!(
        std::panic::catch_unwind(|| latest_version(versions.path())).is_err(),
        "a directory that is not three numbers is read as a version"
    );
}

#[test]
fn a_recording_holding_a_query_copy_fails() {
    let _beside = crate::beside();
    let above = tempfile::tempdir().expect("a directory for the recording");
    let at = above.path().join("todo-comments");
    std::fs::create_dir(&at).expect("the recording's directory");
    std::fs::write(at.join("scope.toml"), "include = [\"**\"]\n").expect("its scope");
    std::fs::write(at.join("query.ryoql"), "emit nothing;\n").expect("its copy of a query");
    let failure =
        panic_message(|| drop(recording_at(&at))).expect("a recording holding a copy fails");
    assert!(
        failure.contains("query.ryoql"),
        "the failure names the copy: {failure}"
    );
}

#[test]
fn a_replay_rule_named_like_another_rule_fails() {
    let roots = tempfile::tempdir().expect("a directory of rules");
    let at = Roots {
        package: roots.path().join("package"),
        replay: roots.path().join("replay"),
    };
    let make = |path: &Path| std::fs::create_dir_all(path).expect("a rule's directory");
    make(&at.replay.join("both-replay-and-tastemaker"));
    make(&at.package.join("rules/both-replay-and-tastemaker/0.1.0"));
    make(&at.replay.join("both-replay-and-benchmark"));
    make(
        &at.package
            .join("benchmarks/rules/both-replay-and-benchmark/0.1.0"),
    );
    // Spec rules no list names: a spec's rules are those of its highest version.
    make(
        &at.package
            .join("specs/a-spec/0.10.0/rules/both-replay-and-spec"),
    );
    make(&at.replay.join("both-replay-and-spec"));
    make(&at.package.join("specs/a-spec/0.10.0/rules/spec-only"));
    make(
        &at.package
            .join("specs/a-spec/0.9.0/rules/in-an-older-spec-only"),
    );
    make(&at.replay.join("in-an-older-spec-only"));
    make(&at.package.join("specs/a-spec/0.10.0/rules/in-two-specs"));
    make(&at.package.join("specs/b-spec/0.1.0/rules/in-two-specs"));
    make(
        &at.package
            .join("specs/b-spec/0.1.0/rules/both-spec-and-tastemaker"),
    );
    make(&at.package.join("rules/both-spec-and-tastemaker/0.1.0"));
    make(&at.package.join("rules/both-tastemaker-and-benchmark/0.1.0"));
    make(
        &at.package
            .join("benchmarks/rules/both-tastemaker-and-benchmark/0.1.0"),
    );
    make(&at.replay.join("replay-only"));
    for name in [
        "both-replay-and-tastemaker",
        "both-replay-and-benchmark",
        "both-replay-and-spec",
        "in-two-specs",
        "both-spec-and-tastemaker",
        "both-tastemaker-and-benchmark",
        "no-rule-at-all",
    ] {
        assert!(
            std::panic::catch_unwind(|| resolve(name, &at)).is_err(),
            "`{name}` resolves to one of the rules it names"
        );
    }
    assert_eq!(resolve("replay-only", &at), at.replay.join("replay-only"));
    assert_eq!(
        resolve("spec-only", &at),
        at.package.join("specs/a-spec/0.10.0/rules/spec-only")
    );
    assert_eq!(
        resolve("in-an-older-spec-only", &at),
        at.replay.join("in-an-older-spec-only"),
        "a rule only an older version of a spec holds is no live spec rule"
    );
}
