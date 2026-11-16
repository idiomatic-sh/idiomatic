//! `validate`'s workload over the bench workspace.

use tastemaker_cli::resources::workspace::validate::command::{ValidateArguments, run};

/// The `validate` workload: the command run once per round through its own composition, its answer checked.
pub(crate) fn workload() -> crate::measure::Measured {
    let directory = crate::fixtures::workspace();
    let edge = crate::fixtures::edge(&directory);
    crate::measure::rounds(
        "workspace/validate",
        || crate::fixtures::arguments::<ValidateArguments>(&["validate"]),
        |arguments| {
            let outcome = run(arguments, &edge);
            assert_eq!(
                outcome,
                tastemaker_cli::outcome::Outcome::Success,
                "`validate` ends as it should"
            );
        },
    )
}

/// How many rounds the repository's own definitions are validated over: each a copy of its own, validated cold and then warm.
const REPOSITORY_ROUNDS: usize = 3;

/// `tm validate` over the repository's own definitions, as the command line runs it: the wall time of a cold run (nothing proved yet)
/// and of a warm one (everything proved), in milliseconds, and the largest resident set either reached, in kibibytes, which
/// `/usr/bin/time` reports. Taken on four processors, so its workers are the same on every machine.
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a benchmark fails by panicking: a copy that cannot be made or a validation that fails leaves nothing to measure, and the message names which"
)]
pub(crate) fn repository() -> Vec<crate::measure::Measured> {
    crate::measure::require_pinned();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("the repository root")
        .to_path_buf();
    let validated = |directory: &std::path::Path| -> (f64, f64) {
        let started = std::time::Instant::now();
        let output = std::process::Command::new("/usr/bin/time")
            .args(["-f", "peak-kib %M", env!("CARGO_BIN_EXE_tm"), "validate"])
            .current_dir(directory)
            .output()
            .expect("tm starts");
        let milliseconds = started.elapsed().as_secs_f64() * 1000.0_f64;
        let said = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(output.status.success(), "tm validate failed:\n{said}");
        let kibibytes = said
            .lines()
            .find_map(|line| line.strip_prefix("peak-kib ")?.trim().parse::<f64>().ok())
            .unwrap_or_else(|| panic!("`time` reported no peak:\n{said}"));
        (milliseconds, kibibytes)
    };
    let (mut colds, mut warms, mut peaks) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..REPOSITORY_ROUNDS {
        let scratch = tempfile::tempdir().expect("the scene directory");
        let git = std::process::Command::new("git")
            .args(["init", "-q"])
            .arg(scratch.path())
            .status()
            .expect("git starts");
        assert!(git.success(), "git init succeeds");
        let copied = std::process::Command::new("cp")
            .arg("-r")
            .arg(root.join("tastemaker"))
            .arg(scratch.path().join("tastemaker"))
            .status()
            .expect("cp starts");
        assert!(copied.success(), "the definitions are copied");
        std::fs::write(
            scratch.path().join("tastemaker.toml"),
            concat!(
                "tastemaker = \"",
                env!("CARGO_PKG_VERSION"),
                "\"\nworkspace = { name = \"bar\", dependencies = [{ package = \"idiomatic.arch\", path = \"tastemaker\" }] }\n"
            ),
        )
        .expect("the workspace is written");
        let (cold, cold_peak) = validated(scratch.path());
        let (warm, warm_peak) = validated(scratch.path());
        colds.push(cold);
        warms.push(warm);
        peaks.push(cold_peak.max(warm_peak));
    }
    let measured = |measure: &'static str, rounds: Vec<f64>| crate::measure::Measured {
        id: "workspace/validate-repository".to_owned(),
        measure,
        rounds,
    };
    vec![
        measured("cold-ms", colds),
        measured("warm-ms", warms),
        measured("peak-kib", peaks),
    ]
}
