//! `asymptote check` ended by SIGINT or SIGTERM while a tool runs: the tool's whole process group is gone, and the
//! command ends as the signal ends it.
#![cfg(unix)]

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// A product whose one tool writes its process id and its background child's, then waits on the child.
const MANIFEST: &str = "[[tools]]\nid = \"slow\"\ngroup = \"check\"\nrun = [\"sh\", \"-c\", \"sleep 120 & echo $! > child.pid; echo $$ > tool.pid; wait\"]\ninputs = [\"asymptote.toml\"]\n";

/// Whether the process `pid` is still there, a zombie not yet reaped included.
fn alive(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// The text of `path` once it is written, waiting up to `limit`.
fn written(path: &std::path::Path, limit: Duration) -> String {
    let started = Instant::now();
    loop {
        if let Ok(text) = std::fs::read_to_string(path)
            && text.ends_with('\n')
        {
            return text.trim().to_owned();
        }
        assert!(
            started.elapsed() < limit,
            "{} was never written",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Runs `check` in a fresh repository, sends it `signal` once its tool runs, and checks how it ended.
#[expect(
    clippy::expect_used,
    reason = "test support: a scene that cannot be set up is a broken test, and the message names which"
)]
fn ended_by(signal: &str, number: i32) {
    let product = tempfile::tempdir().expect("a temporary directory");
    let path = product.path();
    let git = Command::new("git")
        .args(["init", "-q"])
        .current_dir(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .status()
        .expect("git runs");
    assert!(git.success(), "`git init` makes the repository");
    std::fs::write(path.join("asymptote.toml"), MANIFEST).expect("the manifest");
    // A shell starts asymptote and says how it ended: 128 and the signal's number when a signal ended it.
    let mut shell = Command::new("sh")
        .args([
            "-c",
            // In the foreground, never `&`: a script's background job starts with SIGINT ignored.
            "sh -c 'echo $$ > asymptote.pid; exec \"$0\" check' \"$0\" >/dev/null 2>&1; echo $? > ended",
            env!("CARGO_BIN_EXE_asymptote"),
        ])
        .current_dir(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .spawn()
        .expect("the shell starts");
    let asymptote = written(&path.join("asymptote.pid"), Duration::from_secs(30));
    let tool = written(&path.join("tool.pid"), Duration::from_secs(30));
    let child = written(&path.join("child.pid"), Duration::from_secs(30));
    let sent = Command::new("kill")
        .args([&format!("-{signal}"), &asymptote])
        .status()
        .expect("kill runs");
    assert!(sent.success(), "the signal is sent");
    let waited = shell.wait().expect("the shell ends");
    assert!(waited.success(), "the shell waits for asymptote");
    let ended = written(&path.join("ended"), Duration::from_secs(30));
    assert_eq!(
        ended,
        (128_i32 + number).to_string(),
        "asymptote ends as {signal} ends it"
    );
    let started = Instant::now();
    while (alive(&tool) || alive(&child)) && started.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!alive(&tool), "the tool {tool} is gone after {signal}");
    assert!(
        !alive(&child),
        "the tool's child {child} is gone after {signal}"
    );
}

#[test]
fn sigint_ends_check_and_its_running_tool() {
    ended_by("INT", 2);
}

#[test]
fn sigterm_ends_check_and_its_running_tool() {
    ended_by("TERM", 15);
}
