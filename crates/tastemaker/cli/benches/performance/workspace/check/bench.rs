//! `check`'s workloads: over the bench workspace, and warm over a workspace of fifty instances, through the command line's
//! own composition, its lock written and its plan kept by the `check --fix` before.

use tastemaker_cli::resources::workspace::check::command::{CheckArguments, run};

#[path = "../../../../tests/fixtures/replay/pinned.rs"]
mod pinned;

/// The `check` workload: the command run once per round through its own composition, its answer checked.
pub(crate) fn workload() -> crate::measure::Measured {
    let directory = crate::fixtures::workspace();
    let edge = crate::fixtures::edge(&directory);
    crate::measure::rounds(
        "workspace/check",
        || crate::fixtures::arguments::<CheckArguments>(&["check"]),
        |arguments| {
            let outcome = run(arguments, &edge);
            assert_eq!(
                outcome,
                tastemaker_cli::outcome::Outcome::Failure,
                "`check` ends as it should"
            );
        },
    )
}

/// A workspace of fifty `src/<domain>` instances, its lock written and its plan kept.
#[expect(
    clippy::expect_used,
    reason = "a benchmark's setup fails by panicking: a scene that cannot be written or a line that cannot be parsed is a broken benchmark, and the message names which"
)]
fn instance_workspace() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let write = |path: &str, text: &str| {
        let place = directory.path().join(path);
        std::fs::create_dir_all(place.parent().expect("a parent")).expect("its directory");
        std::fs::write(place, text).expect("the file");
    };
    std::fs::create_dir_all(directory.path().join(".git")).expect("a .git directory");
    write(
        "tastemaker.toml",
        "workspace = { name = \"w\", dependencies = [{ package = \"p\", path = \"p\" }] }\nclosed = false\nspecs = [{ spec = \"p/part\", include = [\"src/{domain}\"] }]\n",
    );
    write("p/package.toml", "name = \"p\"\n");
    write("p/specs/part/0.1.0/spec.toml", crate::fixtures::SPEC);
    write(
        "p/specs/part/0.1.0/templates/errors/template.toml",
        crate::fixtures::TEMPLATE,
    );
    for index in 0_u32..50_u32 {
        write(&format!("src/d{index}/errors.rs"), "pub enum Error {}\n");
    }
    directory
}

/// The warm `check` workload over a workspace of fifty instances, taken after a first run wrote its lock and kept its plan.
#[expect(
    clippy::expect_used,
    reason = "a benchmark's setup fails by panicking: a scene that cannot be written or a line that cannot be parsed is a broken benchmark, and the message names which"
)]
pub(crate) fn instances() -> crate::measure::Measured {
    let directory = instance_workspace();
    let edge = tastemaker_cli::resources::edge_at(
        tastemaker_core::domain::files::types::MachinePath(
            directory.path().display().to_string().into(),
        ),
        tastemaker_core::domain::settings::types::Build("bench".into()),
        tastemaker_cli::output::Format::Quiet,
        tastemaker_core::domain::writing::types::Writing::Write,
    );
    let line = |words: &[&str]| {
        let command = <CheckArguments as clap::Args>::augment_args(clap::Command::new("check"));
        <CheckArguments as clap::FromArgMatches>::from_arg_matches(
            &command
                .try_get_matches_from(words)
                .expect("the line parses"),
        )
        .expect("the arguments")
    };
    let validation = <tastemaker_cli::resources::workspace::validate::command::ValidateArguments as clap::Args>::augment_args(clap::Command::new("validate"));
    let arguments = <tastemaker_cli::resources::workspace::validate::command::ValidateArguments as clap::FromArgMatches>::from_arg_matches(
        &validation.try_get_matches_from(["validate"]).expect("the line parses"),
    ).expect("the arguments");
    assert_eq!(
        tastemaker_cli::resources::workspace::validate::command::run(arguments, &edge),
        tastemaker_cli::outcome::Outcome::Success,
        "the fifty-instance workspace validates"
    );
    assert_eq!(
        run(line(&["check", "--fix"]), &edge),
        tastemaker_cli::outcome::Outcome::Success,
        "the fifty-instance workspace places its lock"
    );
    crate::measure::rounds(
        "workspace/check-fifty-instances",
        || (),
        |()| {
            assert_eq!(
                run(line(&["check"]), &edge),
                tastemaker_cli::outcome::Outcome::Success,
                "`check` over fifty instances ends as it should"
            );
        },
    )
}

/// The processors `tm check` is held to in the peak, so that it runs the same workers on every machine (four: the fewest a runner has).
const CPUS: &str = "0-3";

/// The kibibytes of the largest resident set `tm check` reached over a copy of the repository at its `HEAD`, run in
/// `crates/ryo`, cold: a fresh copy and no cache of Tastemaker's own, taken in the process the command line runs as.
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a benchmark fails by panicking: a copy that cannot be made, or a run that checked nothing, leaves nothing to measure, and the message names which"
)]
pub(crate) fn cold_peak() -> crate::measure::Measured {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("the repository root")
        .to_path_buf();
    crate::measure::peaks(
        "workspace/check-ryo-cold",
        3,
        || {
            let scratch = tempfile::tempdir().expect("the scene directory");
            let archive = std::process::Command::new("sh")
                .arg("-c")
                .arg("git -C \"$1\" archive HEAD | tar -x -C \"$2\"")
                .arg("sh")
                .arg(&root)
                .arg(scratch.path())
                .status()
                .expect("git archive starts");
            assert!(archive.success(), "the repository is archived");
            pinned::pinned(&scratch.path().join("crates/ryo/tastemaker.toml"))
                .expect("the workspace pinned to the built tm");
            let init = std::process::Command::new("git")
                .args(["init", "-q"])
                .arg(scratch.path())
                .status()
                .expect("git starts");
            assert!(init.success(), "git init succeeds");
            scratch
        },
        |scratch| {
            // The command line runs a worker for each processor it may use, and the lock holds no machine: the run is held to
            // the first `CPUS` processors, so its workers, and the tables each holds, are the same on every machine.
            let output = std::process::Command::new("taskset")
                .args(["-c", CPUS, "/usr/bin/time"])
                .args(["-f", "peak-kib %M", env!("CARGO_BIN_EXE_tm"), "check", "--format", "json"])
                .current_dir(scratch.path().join("crates/ryo"))
                .output()
                .expect("tm starts");
            let said = String::from_utf8_lossy(&output.stderr).into_owned();
            // The peak says something of memory only for a run that checked the tree: it ends clean (0) or with findings
            // (1) and its report holds a finding or a suppressed one (crates/ryo always has suppressed ones).
            assert!(
                matches!(output.status.code(), Some(0 | 1)),
                "tm check did not check the tree (exit {:?}):\n{said}",
                output.status.code()
            );
            let report: serde_json::Value = serde_json::from_slice(&output.stdout)
                .unwrap_or_else(|error| panic!("tm check answered no report ({error}):\n{said}"));
            let reported = |key: &str| report[key].as_array().map_or(0, Vec::len);
            assert!(
                reported("findings") + reported("suppressed") > 0,
                "tm check reported nothing over crates/ryo, so it checked nothing:\n{said}"
            );
            let kibibytes = said
                .lines()
                .find_map(|line| line.strip_prefix("peak-kib ")?.trim().parse::<f64>().ok())
                .unwrap_or_else(|| panic!("`time` reported no peak:\n{said}"));
            drop(scratch);
            kibibytes
        },
    )
}
