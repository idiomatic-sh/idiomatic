//! What the rules cost, held to the ratchet (`tests/fixtures/ratchet.toml`): every live rule together over ruff, cold, warm, after an
//! edit and at its peak, and the runbook workloads over Django. A run may not cost more than the best a run has done, within its
//! tolerance, and a run that beats it must lower it. The budget (the `[budget]` table of the same file) is the target and is
//! never changed: each run reports its gap to it. These rows time the process: they run on a quiet lock, alone, and the Django rows run
//! only where the pinned run and the sealed worker are on disk.

use crate::ratchet::{budget_number, hold, judge};
use crate::workload::{self, Workload};
use crate::{base, counting, ruff};
use std::sync::Arc;
use std::time::Instant;

use ryo_core::inbound::use_cases::lint::types::Rule;

/// The rules every row of ruff runs.
fn every_rule() -> Vec<Rule> {
    counting::runnable()
        .into_iter()
        .map(|(_, rule)| rule)
        .collect()
}

/// The workers the budget for ruff runs on.
fn workers() -> usize {
    usize::try_from(budget_number("workers")).expect("the budget's workers fit a count")
}

/// The budget of a metric of ruff.
fn budget(metric: &str) -> Option<u128> {
    Some(budget_number(metric))
}

/// A cold run over ruff with no store of ryo's own: wall and process CPU (user and kernel), in milliseconds.
fn cold_cost(directory: &std::path::Path) -> (u128, u128) {
    let (wall, cpu) = (Instant::now(), cpu_time::ProcessTime::now());
    let counted = counting::lint_with_workers(
        directory,
        every_rule(),
        Arc::new(ryo_core::outbound::lint_store::factory::discard()),
        workers(),
    )
    .expect("the rules run over ruff");
    let cost = (wall.elapsed().as_millis(), cpu.elapsed().as_millis());
    assert!(counted.reads > 0, "a cold run reads ruff's files");
    cost
}

#[test]
fn every_live_rule_together_holds_the_cold_ratchet_of_ruff() {
    let _alone = crate::alone();
    let (wall, cpu) = cold_cost(&base::snapshot::ruff());
    hold(&[
        judge(
            "ruff",
            "cold-milliseconds",
            wall,
            budget("cold-milliseconds"),
        ),
        judge(
            "ruff",
            "cold-cpu-milliseconds",
            cpu,
            budget("cold-cpu-milliseconds"),
        ),
    ]);
}

#[test]
fn a_warm_run_and_an_edit_run_over_ruff_hold_their_ratchets() {
    let _alone = crate::alone();
    let copy = ruff::ruff_copy();
    let store = base::store_at(copy.path());
    let _cold = counting::lint_counting(copy.path(), every_rule(), Arc::clone(&store))
        .expect("the cold run keeps its store");
    let started = Instant::now();
    let warm = counting::lint_counting(copy.path(), every_rule(), Arc::clone(&store))
        .expect("the warm run");
    let warm_wall = started.elapsed().as_millis();
    assert_eq!(warm.reads, 0, "a warm run reads no file");

    let edited = copy.path().join("crates/ruff_python_ast/src/lib.rs");
    let mut text = std::fs::read_to_string(&edited).expect("a file of ruff to edit");
    text.push_str("\npub fn edited_for_the_cost_row() {}\n");
    std::fs::write(&edited, text).expect("the edit");
    let started = Instant::now();
    let after = counting::lint_counting(copy.path(), every_rule(), store).expect("the edit run");
    let edit_wall = started.elapsed().as_millis();
    assert!(after.reads > 0, "an edit run reads the file edited");
    hold(&[
        judge(
            "ruff",
            "warm-milliseconds",
            warm_wall,
            budget("warm-milliseconds"),
        ),
        judge(
            "ruff",
            "edit-milliseconds",
            edit_wall,
            budget("edit-milliseconds"),
        ),
    ]);
}

/// The argument that names the directory a child process runs the cold run over, and so marks the process as the child.
#[cfg(target_os = "linux")]
const COLD_DIRECTORY: &str = "cold-peak-directory=";

/// What the child prints before the peak it measured, in megabytes.
#[cfg(target_os = "linux")]
const PEAK_LINE: &str = "peak-megabytes ";

/// The peak memory of this process so far, in megabytes.
#[cfg(target_os = "linux")]
fn peak_megabytes() -> u128 {
    let status = std::fs::read_to_string("/proc/self/status").expect("the process status");
    status
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmHWM:")?
                .trim()
                .strip_suffix("kB")?
                .trim()
                .parse::<u128>()
                .ok()
        })
        .expect("the status holds the peak in kilobytes")
        / 1024
}

#[cfg(target_os = "linux")]
#[test]
fn a_cold_run_over_ruff_holds_the_peak_ratchet() {
    let named = std::env::args().find_map(|argument| {
        argument
            .strip_prefix(COLD_DIRECTORY)
            .map(std::path::PathBuf::from)
    });
    if let Some(directory) = named {
        // The child: the cold run alone, so the peak is its own.
        let _cost = cold_cost(&directory);
        eprintln!("{PEAK_LINE}{}", peak_megabytes());
    } else {
        let _alone = crate::alone();
        let child = std::process::Command::new(std::env::current_exe().expect("this test program"))
            .args([
                "--exact",
                "cost::a_cold_run_over_ruff_holds_the_peak_ratchet",
                "--nocapture",
                "--test-threads=1",
            ])
            .arg(format!("{COLD_DIRECTORY}{}", base::snapshot::ruff().display()))
            .output()
            .expect("the child runs");
        let said = String::from_utf8_lossy(&child.stderr);
        assert!(child.status.success(), "the child failed: {said}");
        let peak: u128 = said
            .lines()
            .find_map(|line| line.strip_prefix(PEAK_LINE)?.trim().parse().ok())
            .expect("the child printed its peak");
        hold(&[judge(
            "ruff",
            "peak-megabytes",
            peak,
            budget("peak-megabytes"),
        )]);
    }
}

/// The inputs of the Django rows, or the reason they are not there, told to the person running the rows.
fn django() -> Option<workload::Inputs> {
    match workload::inputs() {
        Ok(inputs) => Some(inputs),
        Err(reason) => {
            eprintln!("not available: {reason}");
            None
        }
    }
}

/// Runs `workload` over Django, once, in a cache of its own that is kept for the second run when `again` asks for one: its cost, and the
/// cost of the run after it.
fn django_cost(
    inputs: &workload::Inputs,
    workload: Workload,
    again: bool,
) -> (workload::Cost, Option<workload::Cost>) {
    let directory = tempfile::tempdir().expect("a directory for the run");
    let request = workload::request(
        inputs,
        workload,
        &directory.path().join("cache"),
        directory.path(),
    );
    let first = workload::run(inputs, &request);
    let second = again.then(|| workload::run(inputs, &request));
    (first, second)
}

/// The verdicts of a Django cost against the table of `workload`, for the metrics it names.
fn django_verdicts(
    table: &str,
    cost: &workload::Cost,
    metrics: &[&str],
) -> Vec<Result<String, String>> {
    metrics
        .iter()
        .map(|metric| {
            let measured = match *metric {
                "wall-milliseconds" => cost.wall,
                "cpu-milliseconds" => cost.cpu,
                "peak-megabytes" => cost.peak,
                other => panic!("`{other}` is not a metric of a Django row"),
            };
            judge(table, metric, measured, None)
        })
        .collect()
}

#[test]
fn n999_over_django_holds_its_ratchet() {
    let _alone = crate::alone();
    let Some(inputs) = django() else { return };
    let (cost, _) = django_cost(&inputs, Workload::OneRule("N999"), false);
    hold(&django_verdicts(
        "django-n999",
        &cost,
        &["wall-milliseconds", "cpu-milliseconds", "peak-megabytes"],
    ));
}

#[test]
fn one_file_with_every_rule_holds_its_ratchet() {
    let _alone = crate::alone();
    let Some(inputs) = django() else { return };
    let (cost, _) = django_cost(&inputs, Workload::OneFile, false);
    hold(&django_verdicts(
        "django-one-file-413",
        &cost,
        &["wall-milliseconds", "peak-megabytes"],
    ));
}

#[test]
fn every_rule_over_all_of_django_holds_its_cold_and_warm_ratchets() {
    let _alone = crate::alone();
    let Some(inputs) = django() else { return };
    let (cold, warm) = django_cost(&inputs, Workload::Everything, true);
    let warm = warm.expect("the warm run was asked for");
    let metrics = ["wall-milliseconds", "cpu-milliseconds", "peak-megabytes"];
    let mut verdicts = django_verdicts("django-413-cold", &cold, &metrics);
    verdicts.extend(django_verdicts("django-413-warm", &warm, &metrics));
    hold(&verdicts);
}

#[test]
fn an_absent_input_is_a_typed_reason_not_a_pass() {
    let absent = std::path::Path::new("/nonexistent/pinned-django-run");
    assert_eq!(
        workload::inputs_at(absent, None).expect_err("the pinned run is absent"),
        workload::NotAvailable::CorpusAbsent(absent.to_path_buf())
    );
    let pinned = tempfile::tempdir().expect("a directory");
    std::fs::write(pinned.path().join("request.json"), "{}").expect("a request");
    let receipt = pinned.path().join("staged.json");
    assert_eq!(
        workload::inputs_at(pinned.path(), None).expect_err("the request is not staged"),
        workload::NotAvailable::RequestNotStaged(receipt.clone())
    );
    std::fs::write(&receipt, r#"{"staged_digest": ""}"#).expect("a receipt with no digest");
    assert_eq!(
        workload::inputs_at(pinned.path(), None).expect_err("an empty digest is not staged"),
        workload::NotAvailable::RequestNotStaged(receipt.clone())
    );
    std::fs::write(&receipt, r#"{"staged_digest": "04f6c9d8"}"#).expect("a staged receipt");
    assert_eq!(
        workload::inputs_at(pinned.path(), None).expect_err("no worker is named"),
        workload::NotAvailable::WorkerNotNamed
    );
    let missing = pinned.path().join("worker");
    assert_eq!(
        workload::inputs_at(pinned.path(), Some(missing.clone().into_os_string()))
            .expect_err("the named worker is absent"),
        workload::NotAvailable::WorkerAbsent(missing)
    );
}
