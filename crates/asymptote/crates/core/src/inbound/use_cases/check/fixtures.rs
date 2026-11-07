// ignore: idiomatic.arch/file-length - the test data of the check, up and down use cases, which share one scenario over fakes and one measured product on disk; the use-case spec keeps a non-contract use case's fixtures in this one file (a `fixtures/` directory is refused by contract-only-files), so it cannot be split one fixture per file.
//! The test data of the use cases: a product over the fakes of every port, its one tool `lint` printing a SARIF log
//! of two violations in `src/a.rs`, its lock empty.

use std::sync::Arc;

use super::types::{Against, CheckPorts, CheckRequest, Fixing};
use super::use_case::CheckUseCaseImpl;
use crate::application::accepted_state::service::AcceptedState;
use crate::application::cached_report::service::CachedReport;
use crate::domain::manifest::types::{Argument, Argv, Fix, Group, Groups, ProductRoot};
use crate::domain::operations::types::PerfRecord;
use crate::domain::violations::types::{RepoPath, ToolId};
use crate::outbound::bmf::fixtures::fake::BmfFake;
use crate::outbound::check_store::fixtures::fake::CheckStoreFake;
use crate::outbound::clock::fixtures::fake::ClockFake;
use crate::outbound::digest::fixtures::fake::DigestFake;
use crate::outbound::lock::fixtures::fake::LockFake;
use crate::outbound::manifest::fixtures::fake::ManifestFake;
use crate::outbound::matching::fixtures::fake::MatchingFake;
use crate::outbound::repository::fixtures::fake::RepositoryFake;
use crate::outbound::sarif::fixtures::fake::{SarifFake, ignored, reading, result};
use crate::outbound::sources::fixtures::fake::SourcesFake;
use crate::outbound::tools::fixtures::fake::ToolsFake;
use crate::outbound::tools::types::{Captured, Completion, Ending};

/// The product's root.
pub const ROOT: &str = "/repo/p";
/// What the tool prints, which the SARIF fake reads as the two violations.
pub const LOG: &str = "lint-log";

/// A product over fakes, each shared so a test reads what the use case did to it.
#[derive(Debug)]
pub struct Scenario {
    /// Its `asymptote.toml`.
    pub manifest: Arc<ManifestFake>,
    /// Its lock.
    pub lock: Arc<LockFake>,
    /// Its tools.
    pub tools: Arc<ToolsFake>,
    /// Their SARIF.
    pub sarif: Arc<SarifFake>,
    /// Their Bencher Metric Format.
    pub bmf: Arc<BmfFake>,
    /// Its files.
    pub sources: Arc<SourcesFake>,
    /// Fingerprints and digests.
    pub digest: Arc<DigestFake>,
    /// Its cache.
    pub store: Arc<CheckStoreFake>,
    /// Its history.
    pub repository: Arc<RepositoryFake>,
    /// Today.
    pub clock: Arc<ClockFake>,
    /// Globs.
    pub matching: Arc<MatchingFake>,
}

/// The product: the tool `lint`, exiting 1 with the log of two violations, on lines 1 and 2 of `src/a.rs`.
#[must_use]
pub fn scenario() -> Scenario {
    Scenario {
        manifest: Arc::new(ManifestFake::of(ROOT, &["lint"])),
        lock: Arc::new(LockFake::default()),
        tools: Arc::new(ToolsFake::default().answering("lint", 1, LOG)),
        sarif: Arc::new(reading(
            LOG,
            vec![result("src/a.rs", 1), result("src/a.rs", 2)],
        )),
        bmf: Arc::new(BmfFake::default()),
        sources: Arc::new(SourcesFake::holding(&[(
            "src/a.rs",
            "fn a() {}\nfn b() {}\n",
        )])),
        digest: Arc::new(DigestFake),
        store: Arc::new(CheckStoreFake::default()),
        repository: Arc::new(RepositoryFake {
            files: vec![RepoPath("src/a.rs".into())],
            ..RepositoryFake::default()
        }),
        clock: Arc::new(ClockFake),
        matching: Arc::new(MatchingFake),
    }
}

/// The product of [`scenario`] whose tool has a fix command, `lint-fix`, that ends as `ending`, saying `said` on
/// stderr.
#[must_use]
pub fn fixing(ending: Ending, said: &str) -> Scenario {
    let mut manifest = ManifestFake::of(ROOT, &["lint"]);
    for tool in &mut manifest.manifest.tools {
        tool.fix = Fix::Command(Argv(vec![Argument("lint-fix".into())]));
    }
    let mut tools = ToolsFake::default().answering("lint", 1, LOG);
    tools.answers.extend([(
        Argument("lint-fix".into()),
        Completion {
            ending,
            stdout: Captured::default(),
            stderr: Captured(said.into()),
        },
    )]);
    Scenario {
        manifest: Arc::new(manifest),
        tools: Arc::new(tools),
        ..scenario()
    }
}

/// The product of [`scenario`], each of its two violations suppressed by an in-source ignore.
#[must_use]
pub fn ignoring() -> Scenario {
    with_sarif(
        scenario(),
        reading(LOG, vec![ignored("src/a.rs", 1), ignored("src/a.rs", 2)]),
    )
}

/// `check` over the scenario's fakes.
#[must_use]
pub fn use_case(scenario: &Scenario) -> CheckUseCaseImpl {
    CheckUseCaseImpl::new(CheckPorts {
        manifest: Arc::<ManifestFake>::clone(&scenario.manifest),
        accepted: AcceptedState::new(
            Arc::<LockFake>::clone(&scenario.lock),
            Arc::<RepositoryFake>::clone(&scenario.repository),
        ),
        tools: Arc::<ToolsFake>::clone(&scenario.tools),
        sarif: Arc::<SarifFake>::clone(&scenario.sarif),
        bmf: Arc::<BmfFake>::clone(&scenario.bmf),
        lock: Arc::<LockFake>::clone(&scenario.lock),
        sources: Arc::<SourcesFake>::clone(&scenario.sources),
        digest: Arc::<DigestFake>::clone(&scenario.digest),
        store: Arc::<CheckStoreFake>::clone(&scenario.store),
        repository: Arc::<RepositoryFake>::clone(&scenario.repository),
    })
}

/// The request of a plain check of the scenario's product, against its lock.
#[must_use]
pub fn request() -> CheckRequest {
    CheckRequest {
        start: ProductRoot(ROOT.into()),
        against: Against::Lock,
        fixing: Fixing::Report,
        groups: Groups::All,
    }
}

/// The scenario with its SARIF fake replaced.
#[must_use]
pub fn with_sarif(scenario: Scenario, sarif: SarifFake) -> Scenario {
    Scenario {
        sarif: Arc::new(sarif),
        ..scenario
    }
}

/// `up` over the scenario's fakes.
#[must_use]
pub fn up(scenario: &Scenario) -> crate::inbound::use_cases::up::use_case::UpUseCaseImpl {
    crate::inbound::use_cases::up::use_case::UpUseCaseImpl::new(
        crate::inbound::use_cases::up::use_case::UpPorts {
            accepted: AcceptedState::new(
                Arc::<LockFake>::clone(&scenario.lock),
                Arc::<RepositoryFake>::clone(&scenario.repository),
            ),
            report: CachedReport::new(Arc::<SourcesFake>::clone(&scenario.sources)),
            manifest: Arc::<ManifestFake>::clone(&scenario.manifest),
            lock: Arc::<LockFake>::clone(&scenario.lock),
            store: Arc::<CheckStoreFake>::clone(&scenario.store),
            repository: Arc::<RepositoryFake>::clone(&scenario.repository),
            clock: Arc::<ClockFake>::clone(&scenario.clock),
            matching: Arc::<MatchingFake>::clone(&scenario.matching),
        },
    )
}

/// `down` over the scenario's fakes.
#[must_use]
pub fn down(scenario: &Scenario) -> crate::inbound::use_cases::down::use_case::DownUseCaseImpl {
    crate::inbound::use_cases::down::use_case::DownUseCaseImpl::new(
        crate::inbound::use_cases::down::use_case::DownPorts {
            accepted: AcceptedState::new(
                Arc::<LockFake>::clone(&scenario.lock),
                Arc::<RepositoryFake>::clone(&scenario.repository),
            ),
            report: CachedReport::new(Arc::<SourcesFake>::clone(&scenario.sources)),
            manifest: Arc::<ManifestFake>::clone(&scenario.manifest),
            lock: Arc::<LockFake>::clone(&scenario.lock),
            store: Arc::<CheckStoreFake>::clone(&scenario.store),
            repository: Arc::<RepositoryFake>::clone(&scenario.repository),
            clock: Arc::<ClockFake>::clone(&scenario.clock),
        },
    )
}

/// The request of `up` with `selectors` and the reason `why`.
#[must_use]
pub fn up_request(
    selectors: crate::domain::selection::types::Selectors,
) -> crate::inbound::use_cases::up::types::UpRequest {
    crate::inbound::use_cases::up::types::UpRequest {
        start: ProductRoot(ROOT.into()),
        reason: crate::domain::operations::types::Reason("why".into()),
        selectors,
        writing: crate::domain::ratchet::types::Writing::Write,
    }
}

/// The request of `down`.
#[must_use]
pub fn down_request() -> crate::inbound::use_cases::down::types::DownRequest {
    crate::inbound::use_cases::down::types::DownRequest {
        start: ProductRoot(ROOT.into()),
        writing: crate::domain::ratchet::types::Writing::Write,
    }
}

/// The product the group and perf scenarios check, on disk: `lint` (SARIF, nothing found), `bench` (Bencher Metric
/// Format, group `perf`) and `unit` (group `test`, exiting `unit_exit`), in a git repository, so its inputs are listed.
///
/// # Errors
///
/// When the directory cannot be written or git refuses to make it a repository.
pub fn measured_product(bench: &str, unit_exit: i32) -> std::io::Result<tempfile::TempDir> {
    let product = tempfile::tempdir()?;
    let root = product.path();
    std::fs::write(
        root.join("lint.sarif"),
        r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"lint","rules":[]}},"results":[]}]}"#,
    )?;
    std::fs::write(root.join("bench.json"), bench)?;
    std::fs::write(
        root.join("asymptote.toml"),
        format!(
            "[[tools]]\nid=\"lint\"\nrun=[\"sh\",\"-c\",\"cat lint.sarif\"]\ngroup=\"lint\"\noutput=\"sarif\"\n\n[[tools]]\nid=\"bench\"\nrun=[\"sh\",\"-c\",\"cat bench.json\"]\ngroup=\"perf\"\noutput=\"bmf\"\n\n[[tools]]\nid=\"unit\"\nrun=[\"sh\",\"-c\",\"exit {unit_exit}\"]\ngroup=\"test\"\n"
        ),
    )?;
    for arguments in [["init", "-q"], ["add", "-A"]] {
        let git = std::process::Command::new("git")
            .args(arguments)
            .current_dir(root)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()?;
        if !git.status.success() {
            return Err(std::io::Error::other(format!(
                "git {} failed: {}",
                arguments.join(" "),
                String::from_utf8_lossy(&git.stderr)
            )));
        }
    }
    Ok(product)
}

/// A perf record of tool `bench`, benchmark `c`, at `measure`: best `best`.
#[must_use]
pub fn perf_record(measure: &str, best: f64) -> PerfRecord {
    use crate::domain::operations::types::{BenchmarkName, MeasureName, Number, PerfKey};
    PerfRecord {
        key: PerfKey {
            tool: ToolId("bench".into()),
            benchmark: BenchmarkName("c".into()),
            measure: MeasureName(measure.into()),
        },
        best: Number(best),
        reasons: vec![crate::domain::operations::types::Reason("seeded".into())],
    }
}

/// Writes `records` to the lock of the product at `root` in one operation.
///
/// # Errors
///
/// When the lock cannot be written.
pub fn seed_lock(
    root: &std::path::Path,
    records: Vec<PerfRecord>,
) -> Result<(), crate::outbound::lock::errors::LockError> {
    use crate::domain::operations::types::Operation;
    use crate::outbound::lock::interface::LockPort as _;
    // The new operation's id is not needed: a scenario reads the lock back from disk.
    crate::outbound::lock::factory::toml(ProductRoot(root.display().to_string().into())).append(
        &Operation {
            parents: Vec::new(),
            covers: Vec::new(),
            accepted: Vec::new(),
            removed: Vec::new(),
            perf: records,
            perf_removed: Vec::new(),
        },
    )?;
    Ok(())
}

/// The best the lock holds for `wall-ms` of tool `bench`, and how many operations it holds.
///
/// # Errors
///
/// When the lock cannot be read or folded.
pub fn lock_state(
    root: &std::path::Path,
) -> Result<String, crate::inbound::use_cases::check::errors::CheckError> {
    use crate::outbound::lock::interface::LockPort as _;
    let operations =
        crate::outbound::lock::factory::toml(ProductRoot(root.display().to_string().into()))
            .read()?
            .0;
    let folded = crate::domain::operations::service::fold(&operations)?;
    let best = folded
        .perf
        .iter()
        .find(|record| &*record.key.measure.0 == "wall-ms")
        .map_or_else(|| "-".to_owned(), |record| record.best.0.to_string());
    Ok(format!("best {best}; operations {}", operations.len()))
}

/// What a measured scenario runs, and what `up` is asked after it.
#[derive(Debug)]
pub struct Scene {
    /// What `bench` prints.
    pub bench: String,
    /// The exit of `unit`.
    pub unit: i32,
    /// The lock's perf records.
    pub records: Vec<PerfRecord>,
    /// The groups it selects.
    pub groups: Groups,
    /// Whether each tool's fix runs first.
    pub fixing: Fixing,
    /// The tools `up` is asked for.
    pub tools: Vec<ToolId>,
    /// The groups of the checks made first, each after the one before it.
    pub before: Vec<Groups>,
}

/// A scene whose `bench` prints `bench`: with every group, an empty lock and nothing asked of `up`.
fn base(bench: String) -> Scene {
    Scene {
        bench,
        unit: 0,
        records: Vec::new(),
        groups: Groups::All,
        fixing: Fixing::Report,
        tools: Vec::new(),
        before: Vec::new(),
    }
}

/// The scene a scenario's input names, or the input when none is.
///
/// # Errors
///
/// The input.
pub fn scene(input: &str) -> Result<Scene, String> {
    let wall = |value: &str| format!(r#"{{"c": {{"wall-ms": {{"value": {value}}}}}}}"#);
    let held = || vec![perf_record("wall-ms", 100.0)];
    let against_held = |value: &str| Scene {
        records: held(),
        ..base(wall(value))
    };
    let only = |group: &str| Groups::Only(vec![Group(group.into())]);
    let many = "\"m0\": {\"value\": 0}, \"m1\": {\"value\": 1}, \"m2\": {\"value\": 2}, \"m3\": {\"value\": 3}, \"m4\": {\"value\": 4}, \"m5\": {\"value\": 5}, \"m6\": {\"value\": 6}, \"m7\": {\"value\": 7}, \"m8\": {\"value\": 8}, \"m9\": {\"value\": 9}, \"m10\": {\"value\": 10}, \"m11\": {\"value\": 11}, \"m12\": {\"value\": 12}";
    Ok(match input {
        "unseeded" | "seed" => base(wall("100")),
        "regressed" | "raise" => against_held("120"),
        "improved" => against_held("70"),
        "held" => against_held("100"),
        "resolved" => Scene {
            records: vec![perf_record("wall-ms", 100.0), perf_record("cpu-ms", 100.0)],
            ..base(r#"{"c": {"cpu-ms": {"value": 100}}}"#.to_owned())
        },
        "lint-group" => Scene {
            unit: 1,
            records: held(),
            groups: only("lint"),
            ..base(wall("500"))
        },
        "perf-group" => Scene {
            unit: 1,
            records: held(),
            groups: only("perf"),
            ..base(wall("120"))
        },
        "two-groups" => Scene {
            unit: 1,
            groups: Groups::Only(vec![Group("lint".into()), Group("perf".into())]),
            ..base(wall("100"))
        },
        "failing-test" => Scene {
            unit: 1,
            records: held(),
            ..base(wall("100"))
        },
        "all-then-lint" => Scene {
            before: vec![Groups::All, only("lint")],
            ..base(wall("100"))
        },
        "test-group" => Scene {
            unit: 1,
            groups: only("test"),
            ..base(wall("500"))
        },
        "improved-fixing" => Scene {
            fixing: Fixing::Fix,
            ..against_held("70")
        },
        "seed-tool" => Scene {
            tools: vec![ToolId("bench".into())],
            ..base(wall("100"))
        },
        "many" => base(format!("{{\"c\": {{{many}}}}}")),
        "other-tool" => Scene {
            tools: vec![ToolId("lint".into())],
            ..base(wall("100"))
        },
        other => return Err(format!("unknown measured case: {other}")),
    })
}

/// The scenario after a commit: the same product, files, cache and lock, under another head.
#[must_use]
pub fn after_a_commit(scenario: Scenario) -> Scenario {
    Scenario {
        repository: Arc::new(RepositoryFake {
            head: crate::domain::renames::types::Head::Commit(
                crate::domain::operations::types::Revision("another-head".into()),
            ),
            files: vec![RepoPath("src/a.rs".into())],
            ..RepositoryFake::default()
        }),
        ..scenario
    }
}

/// The product whose one tool reads the git range `origin/main..HEAD`.
#[must_use]
pub fn reading_a_range() -> Scenario {
    let mut fake = ManifestFake::of(ROOT, &["lint"]);
    for tool in &mut fake.manifest.tools {
        tool.run = Argv(vec![
            Argument("lint".into()),
            Argument("origin/main..HEAD".into()),
        ]);
    }
    Scenario {
        manifest: Arc::new(fake),
        ..scenario()
    }
}

/// The product with a second tool, `docs`, beside `lint`: `lint` reads `src/` and `docs` reads `docs/`, each
/// printing the log of [`scenario`].
#[must_use]
pub fn two_tools() -> Scenario {
    use crate::domain::manifest::types::Pathspec;
    let mut fake = ManifestFake::of(ROOT, &["lint", "docs"]);
    for tool in &mut fake.manifest.tools {
        tool.inputs = vec![Pathspec(
            if *tool.id.0 == *"lint" {
                "src/"
            } else {
                "docs/"
            }
            .into(),
        )];
    }
    Scenario {
        manifest: Arc::new(fake),
        tools: Arc::new(
            ToolsFake::default()
                .answering("lint", 1, LOG)
                .answering("docs", 1, LOG),
        ),
        sources: Arc::new(SourcesFake::holding(&[
            ("src/a.rs", "fn a() {}\nfn b() {}\n"),
            ("docs/guide.md", "# Guide\n"),
        ])),
        repository: Arc::new(RepositoryFake {
            files: vec![
                RepoPath("docs/guide.md".into()),
                RepoPath("src/a.rs".into()),
            ],
            ..RepositoryFake::default()
        }),
        ..scenario()
    }
}

/// The product with a perf tool `bench` beside `lint`: its median of `wall-ms` is 70.
#[must_use]
pub fn improving() -> Scenario {
    use crate::domain::manifest::types::{Output, Pathspec, Seconds, Tool};
    let mut fake = ManifestFake::of(ROOT, &["lint"]);
    fake.manifest.tools.push(Tool {
        id: ToolId("bench".into()),
        run: Argv(vec![Argument("bench".into())]),
        fix: Fix::Absent,
        output: Output::Bmf,
        group: Group("perf".into()),
        timeout: Seconds(60),
        inputs: vec![Pathspec(".".into())],
    });
    Scenario {
        manifest: Arc::new(fake),
        tools: Arc::new(ToolsFake::default().answering("lint", 1, LOG).answering(
            "bench",
            0,
            "bench-log",
        )),
        bmf: Arc::new(crate::outbound::bmf::fixtures::fake::reading(
            "bench-log",
            vec![crate::outbound::bmf::fixtures::fake::measurement(
                "c", "wall-ms", 70.0,
            )],
        )),
        ..scenario()
    }
}

/// A check of the `improving` product, whose lock holds a best of 100 for the measure: what it judged, whether it
/// rewrote the lock, and how many operations the lock holds after.
///
/// # Errors
///
/// When the lock cannot be written or the check cannot be made.
pub fn improved(
    fixing: Fixing,
    against: Against,
    scenario: &Scenario,
) -> Result<String, crate::inbound::use_cases::check::errors::CheckError> {
    use crate::domain::operations::types::Operation;
    use crate::inbound::use_cases::check::interface::CheckUseCase as _;
    use crate::outbound::lock::interface::LockPort as _;
    scenario.lock.append(&Operation {
        parents: Vec::new(),
        covers: Vec::new(),
        accepted: Vec::new(),
        removed: Vec::new(),
        perf: vec![perf_record("wall-ms", 100.0)],
        perf_removed: Vec::new(),
    })?;
    let report = use_case(scenario).check(CheckRequest {
        against,
        fixing,
        ..request()
    })?;
    Ok(format!(
        "improved {} {:?}; operations {}",
        report.perf.improved.len(),
        report.perf.rewriting,
        scenario.lock.held().len()
    ))
}

/// The captured product's change that makes its manifest refused: its `run` writes a capture no input does.
const UNCAPTURED: &str = "a capture no input names";

/// A product on disk, in a git repository, whose one tool `unit` captures each directory of `areas` as `{area}` and
/// runs once for each, over `areas/a` and `areas/b`; for `UNCAPTURED`, its `run` writes the capture and its inputs
/// do not.
///
/// # Errors
///
/// When the directory cannot be written or git refuses to make it a repository.
pub fn captured_product(change: &str) -> std::io::Result<tempfile::TempDir> {
    let product = tempfile::tempdir()?;
    let root = product.path();
    for area in ["a", "b"] {
        let directory = root.join("areas").join(area);
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join("source.txt"), area)?;
    }
    let inputs = if change == UNCAPTURED {
        "areas"
    } else {
        "areas/{area}"
    };
    std::fs::write(
        root.join("asymptote.toml"),
        format!(
            "[[tools]]\nid=\"unit\"\nrun=[\"sh\",\"-c\",\"test -f areas/{{area}}/source.txt\"]\ngroup=\"check\"\ninputs=[\"{inputs}\"]\n"
        ),
    )?;
    let git = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()?;
    if !git.status.success() {
        return Err(std::io::Error::other(format!(
            "git init failed: {}",
            String::from_utf8_lossy(&git.stderr)
        )));
    }
    Ok(product)
}

/// Applies `change` to the captured product at `root` between its two checks: `nothing`, `an edit in a` (to
/// `areas/a`), or `a new directory c` (`areas/c`, which no manifest names).
///
/// # Errors
///
/// When a file cannot be written, or the change is not one of these.
pub fn change_captured(root: &std::path::Path, change: &str) -> std::io::Result<()> {
    match change {
        "nothing" => Ok(()),
        "an edit in a" => std::fs::write(root.join("areas/a/source.txt"), "edited\n"),
        "a new directory c" => {
            std::fs::create_dir_all(root.join("areas/c"))?;
            std::fs::write(root.join("areas/c/source.txt"), "c")
        }
        other => Err(std::io::Error::other(format!(
            "unknown captured change: {other}"
        ))),
    }
}

/// The tools a check of the captured product knew and those of them it reused: `tools a+b; reused a`, or `none`.
#[must_use]
pub fn captured_runs(report: &super::types::CheckReport) -> String {
    let named = |reused: bool| {
        let ids: Vec<&str> = report
            .runs
            .iter()
            .filter(|run| !reused || run.reuse == super::types::Reuse::Reused)
            .map(|run| &*run.run.tool.0)
            .collect();
        if ids.is_empty() {
            "none".to_owned()
        } else {
            ids.join("+")
        }
    };
    format!("tools {}; reused {}", named(false), named(true))
}
