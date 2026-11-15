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

#[test]
fn every_recording_reads_its_live_rule() {
    let _beside = crate::beside();
    let all = recordings();
    assert_eq!(all.len(), 52, "every recording loads");
    for each in &all {
        assert_eq!(
            each.query,
            rules::query(&each.name),
            "{} replays its rule's live query",
            each.name
        );
    }
    let versions = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tastemaker/rules/repeated-string-literals");
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
    make(&at.replay.join("bdd-runner"));
    make(&at.package.join("rules/both-tastemaker-and-benchmark/0.1.0"));
    make(
        &at.package
            .join("benchmarks/rules/both-tastemaker-and-benchmark/0.1.0"),
    );
    make(&at.replay.join("replay-only"));
    for name in [
        "both-replay-and-tastemaker",
        "both-replay-and-benchmark",
        "bdd-runner",
        "both-tastemaker-and-benchmark",
    ] {
        assert!(
            std::panic::catch_unwind(|| resolve(name, &at)).is_err(),
            "`{name}` resolves to one of the rules it names"
        );
    }
    assert_eq!(resolve("replay-only", &at), at.replay.join("replay-only"));
}
