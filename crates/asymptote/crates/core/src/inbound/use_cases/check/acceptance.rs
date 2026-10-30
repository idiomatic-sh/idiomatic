//! The steps of the `check` feature: each a cucumber expression over the world, asserting the use case's own results.

use cucumber::{given, then, when};

use crate::Behavior;

use asymptote_core::domain::manifest::types::ProductRoot;
use asymptote_core::domain::operations::types::Revision;
use asymptote_core::inbound::use_cases::check::fixtures::{
    captured_product, captured_runs, change_captured, fixing, lock_state, measured_product,
    request, scenario, scene, seed_lock, use_case,
};
use asymptote_core::inbound::use_cases::check::interface::CheckUseCase as _;
use asymptote_core::inbound::use_cases::check::types::{Against, CheckRequest, Fixing, Reuse};
use asymptote_core::outbound::tools::types::{Ending, ExitCode};

/// Names the scene the next `check` step runs.
#[given(expr = "the check input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Checks the fakes' product as the input asks, and keeps what it found and whether it reused the run.
#[when(expr = "check runs")]
fn runs(world: &mut Behavior) {
    let product = match world.input.as_str() {
        "fix-fails" => fixing(Ending::Exited(ExitCode(2)), "cannot fix\n"),
        "fix-times-out" => fixing(Ending::TimedOut, ""),
        "fix-succeeds" => fixing(Ending::Exited(ExitCode(0)), ""),
        _ => scenario(),
    };
    let check = use_case(&product);
    let asked = match world.input.as_str() {
        "fix-fails" | "fix-times-out" | "fix-succeeds" => CheckRequest {
            fixing: Fixing::Fix,
            ..request()
        },
        "since" => CheckRequest {
            against: Against::Since(Revision("origin/main".into())),
            ..request()
        },
        "nowhere" => CheckRequest {
            start: ProductRoot("/elsewhere".into()),
            ..request()
        },
        _ => request(),
    };
    if world.input == "warm" {
        // The first check fills the cache the asked one reuses; one that fails is the answer.
        if let Err(error) = check.check(request()) {
            world.answer = format!("the warm-up check failed: {error}");
            return;
        }
    }
    world.answer = match check.check(asked.clone()) {
        Ok(report) => {
            let counted = if matches!(asked.against, Against::Since(_)) {
                "introduced"
            } else {
                "new"
            };
            let reuse = match report.runs.first().map(|run| run.reuse) {
                Some(Reuse::Reused) => "reused",
                Some(Reuse::Ran) | None => "ran",
            };
            let fixes: Vec<String> = report
                .failed_fixes
                .iter()
                .map(|failed| format!("; {} fix {}", failed.tool.0, failed_as(&failed.failure)))
                .collect();
            format!(
                "{} {counted}, {reuse}{}",
                report.outcome.new.len(),
                fixes.concat()
            )
        }
        Err(error) => error.to_string(),
    };
}

/// How a fix failed, in the feature's words.
fn failed_as(failure: &asymptote_core::domain::judgement::types::Failure) -> String {
    use asymptote_core::domain::judgement::types::Failure;
    match failure {
        Failure::Exited { code, stderr } => format!("exited {}: {}", code.0, stderr.0),
        Failure::Signalled { stderr } => format!("was ended by a signal: {}", stderr.0),
        Failure::TimedOut { after } => format!("timed out after {} s", after.0),
        Failure::Unstartable(problem) => format!("could not start: {}", problem.0),
        Failure::Unreadable { problem, .. } => format!("could not be read: {}", problem.0),
    }
}

/// Checks a product on disk whose tool prints the input's real SARIF log, over the production adapters.
#[when("the real SARIF tool is checked")]
fn real_sarif(world: &mut Behavior) -> Result<(), Box<dyn std::error::Error>> {
    let log = match world.input.as_str() {
        "unsuccessful" => {
            r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"lint"}},"invocations":[{"executionSuccessful":false}],"results":[]}]}"#
        }
        "indexed" => {
            r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"lint","rules":[{"id":"r"}]}},"results":[{"rule":{"id":"r","index":0},"message":{"text":"finding"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"src/a.rs"},"region":{"startLine":1}}}]}]}]}"#
        }
        "default-indices" => {
            r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"lint","rules":[{"id":"r"}]}},"results":[{"ruleIndex":-1,"rule":{"index":0,"toolComponent":{"name":"lint","index":-1}},"message":{"text":"finding"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"src/a.rs"},"region":{"startLine":1}}}]}]}]}"#
        }
        "pass" => {
            r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"lint"}},"results":[{"ruleId":"r","kind":"pass","level":"none","message":{"text":"ok"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"src/a.rs"}}}]}]}]}"#
        }
        other => return Err(format!("unknown real SARIF case: {other}").into()),
    };
    let product = tempfile::tempdir()?;
    std::fs::create_dir_all(product.path().join("src"))?;
    std::fs::write(product.path().join("src/a.rs"), "fn a() {}\n")?;
    std::fs::write(product.path().join("lint.sarif"), log)?;
    std::fs::write(
        product.path().join("asymptote.toml"),
        "[[tools]]\nid=\"lint\"\nrun=[\"sh\",\"-c\",\"cat lint.sarif\"]\ngroup=\"lint\"\noutput=\"sarif\"\n",
    )?;
    let check = real_check(product.path());
    let report = check.check(CheckRequest {
        start: ProductRoot(product.path().display().to_string().into()),
        ..request()
    })?;
    world.answer = format!("{:?}, {} new", report.verdict, report.outcome.new.len());
    Ok(())
}

/// Checks the measured product on disk as the scene names, after its earlier checks: its verdict, perf, lock, reuse.
#[when(expr = "the measured product {string} is checked")]
fn measured(world: &mut Behavior, input: String) -> Result<(), Box<dyn std::error::Error>> {
    // The step's text, kept as the answer's type: what a cucumber argument arrives as is an owned `String`.
    let input: Box<str> = input.into();
    let scene = scene(&input)?;
    let product = measured_product(&scene.bench, scene.unit)?;
    if !scene.records.is_empty() {
        seed_lock(product.path(), scene.records)?;
    }
    let asked = |groups| CheckRequest {
        start: ProductRoot(product.path().display().to_string().into()),
        groups,
        ..request()
    };
    let checked_before = !scene.before.is_empty();
    for groups in scene.before {
        // The earlier checks leave the runs the last one reuses, so each must have run a tool.
        if real_check(product.path())
            .check(asked(groups))?
            .runs
            .is_empty()
        {
            return Err("an earlier check ran no tool".into());
        }
    }
    let report = real_check(product.path()).check(CheckRequest {
        start: ProductRoot(product.path().display().to_string().into()),
        groups: scene.groups,
        fixing: scene.fixing,
        ..request()
    })?;
    let perf = &report.perf;
    let reused = if checked_before {
        let reused: Vec<&str> = report
            .runs
            .iter()
            .filter(|run| run.reuse == Reuse::Reused)
            .map(|run| &*run.run.tool.0)
            .collect();
        format!("; reused {}", reused.join("+"))
    } else {
        String::new()
    };
    let ran: Vec<&str> = report.runs.iter().map(|run| &*run.run.tool.0).collect();
    world.answer = format!(
        "{:?}; ran {}; regressed {} unseeded {} improved {} resolved {}; {}{}",
        report.verdict,
        ran.join("+"),
        perf.regressed.len(),
        perf.unseeded.len(),
        perf.improved.len(),
        perf.resolved.len(),
        lock_state(product.path())?,
        reused
    );
    Ok(())
}

/// Checks the captured product twice, the change applied between: the tools the second knew and reused, or the refusal.
#[when(expr = "the captured product is checked after {string}")]
fn captured(world: &mut Behavior, change: String) -> Result<(), Box<dyn std::error::Error>> {
    let product = captured_product(&change)?;
    let root = product.path();
    let asked = CheckRequest {
        start: ProductRoot(root.display().to_string().into()),
        ..request()
    };
    if let Err(error) = real_check(root).check(asked.clone()) {
        world.answer = error.to_string().replace(&*root.display().to_string(), ".");
        return Ok(());
    }
    change_captured(root, &change.into_boxed_str())?;
    world.answer = captured_runs(&real_check(root).check(asked)?);
    Ok(())
}

/// Asserts the last `check` step's answer.
#[then(expr = "check answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    // The step's text, kept as the answer's type: what a cucumber argument arrives as is an owned `String`.
    let expected: Box<str> = expected.into();
    assert_eq!(&*world.answer, &*expected, "check answered otherwise");
}

/// Checks a product on disk three times over independent production cache connections, the change applied after
/// the first, and keeps each report beside the one expected of it.
#[when(expr = "the production cache is reopened after {string}")]
fn reopened_cache(world: &mut Behavior, change: String) -> Result<(), Box<dyn std::error::Error>> {
    let change: Box<str> = change.into();
    let product = tempfile::tempdir()?;
    let root = product.path();
    std::fs::create_dir_all(root.join("src"))?;
    std::fs::write(root.join("src/a.rs"), "fn a() {}\n")?;
    std::fs::write(
        root.join("lint.sarif"),
        r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"lint","rules":[{"id":"r"}]}},"results":[{"ruleId":"r","level":"warning","partialFingerprints":{"primaryLocationLineHash":"2a"},"message":{"text":"finding"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"src/a.rs"},"region":{"startLine":1}}}]}]}]}"#,
    )?;
    std::fs::write(
        root.join("asymptote.toml"),
        "[[tools]]\nid=\"lint\"\nrun=[\"sh\",\"-c\",\"cat lint.sarif\"]\ngroup=\"lint\"\noutput=\"sarif\"\ninputs=[\"src/a.rs\",\"lint.sarif\"]\n",
    )?;
    for args in [
        vec!["init", "--quiet"],
        vec!["add", "src/a.rs", "lint.sarif", "asymptote.toml"],
    ] {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .output()?;
        if !output.status.success() {
            return Err(format!("git setup failed: {output:?}").into());
        }
    }
    let asked = CheckRequest {
        start: ProductRoot(root.display().to_string().into()),
        ..request()
    };
    // Each temporary use case and its connection are dropped before the next opens the store.
    world
        .check_reports
        .push(real_check(root).check(asked.clone())?);
    world
        .expected_check_reports
        .push(expected_report(asked.start.clone(), Reuse::Ran, true));
    match change.as_ref() {
        "no change" => {}
        "source and tool output edit" => {
            std::fs::write(root.join("src/a.rs"), "fn repaired_source() {}\n")?;
            std::fs::write(
                root.join("lint.sarif"),
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"lint","rules":[{"id":"r"}]}},"results":[]}]}"#,
            )?;
        }
        "corrupted store" => {
            let store = root.join(".asymptote/cache/check.store");
            if !store.is_file() {
                return Err("expected the persisted store".into());
            }
            std::fs::write(store, b"invalid store")?;
        }
        other => return Err(format!("unknown lifecycle change: {other}").into()),
    }
    let has_finding = change.as_ref() != "source and tool output edit";
    let reuse = if change.as_ref() == "no change" {
        Reuse::Reused
    } else {
        Reuse::Ran
    };
    world
        .check_reports
        .push(real_check(root).check(asked.clone())?);
    world
        .expected_check_reports
        .push(expected_report(asked.start.clone(), reuse, has_finding));
    // A rebuilt batch must itself survive another independent reopen.
    world
        .check_reports
        .push(real_check(root).check(asked.clone())?);
    world
        .expected_check_reports
        .push(expected_report(asked.start, Reuse::Reused, has_finding));
    Ok(())
}

/// Asserts each reopened check answered the complete report expected of it.
#[then("every complete check report matches its expected lifecycle outcome")]
fn complete_reports(world: &mut Behavior) {
    assert!(
        !world.expected_check_reports.is_empty(),
        "the lifecycle must run"
    );
    assert_eq!(
        world.check_reports, world.expected_check_reports,
        "a reopened check answered other than its lifecycle expects"
    );
}

/// The complete observable result of the declared SARIF fixture, independent of any cached answer.
fn expected_report(
    root: ProductRoot,
    reuse: Reuse,
    has_finding: bool,
) -> asymptote_core::inbound::use_cases::check::types::CheckReport {
    use asymptote_core::domain::judgement::types::{Inventory, Judgement, Run, Status, Verdict};
    use asymptote_core::domain::violations::types::{
        Fingerprint, Fingerprinted, Level, LineNumber, MessageText, Place, RepoPath, RuleId,
        Suppression, ToolId, Violation,
    };
    use asymptote_core::inbound::use_cases::check::types::{CheckReport, ToolRun};
    let new = if has_finding {
        vec![Violation {
            path: RepoPath("src/a.rs".into()),
            place: Place::Line(LineNumber(1)),
            tool: ToolId("lint".into()),
            rule: RuleId("r".into()),
            level: Level::Warning,
            suppression: Suppression::Unsuppressed,
            message: MessageText("finding".into()),
            // The production digest fingerprints the source line, replacing the tool-supplied hash.
            fingerprint: Fingerprinted::Known(Fingerprint(2_039_020_613_143_530_717)),
        }]
    } else {
        Vec::new()
    };
    CheckReport {
        root,
        runs: vec![ToolRun {
            run: Run {
                tool: ToolId("lint".into()),
                status: Status::Completed,
                inventory: Inventory::Declared([RuleId("r".into())].into()),
                reported: if has_finding {
                    [RuleId("r".into())].into()
                } else {
                    std::collections::BTreeSet::default()
                },
            },
            reuse,
        }],
        outcome: Judgement {
            new,
            ..Judgement::default()
        },
        verdict: if has_finding {
            Verdict::Changed
        } else {
            Verdict::Pass
        },
        against: Against::Lock,
        perf: asymptote_core::domain::perf::types::PerfOutcome::default(),
        failed_fixes: Vec::new(),
    }
}

/// Opens an independent production connection and injects the real adapters.
pub(crate) fn real_check(
    root: &std::path::Path,
) -> asymptote_core::inbound::use_cases::check::use_case::CheckUseCaseImpl {
    use asymptote_core::inbound::use_cases::check::types::CheckPorts;
    use asymptote_core::inbound::use_cases::check::use_case::CheckUseCaseImpl;
    use asymptote_core::outbound::repository::interface::RepositoryPort;
    use asymptote_core::outbound::{
        bmf, check_store, digest, lock, manifest, repository, sarif, sources, tools,
    };
    use std::sync::Arc;
    let product = ProductRoot(root.display().to_string().into());
    let lister =
        repository::factory::git(product.clone(), Ok(repository::types::Worktree::default()));
    let repository: Arc<dyn RepositoryPort> =
        Arc::new(repository::factory::git(product.clone(), lister.worktree()));
    CheckUseCaseImpl::new(CheckPorts {
        manifest: Arc::new(manifest::factory::toml()),
        accepted: asymptote_core::application::accepted_state::service::AcceptedState::new(
            Arc::new(lock::factory::toml(product.clone())),
            Arc::clone(&repository),
        ),
        tools: Arc::new(tools::factory::subprocess(
            tools::factory::ToolsConnection::default(),
        )),
        sarif: Arc::new(sarif::factory::serde_sarif()),
        bmf: Arc::new(bmf::factory::json()),
        lock: Arc::new(lock::factory::toml(product.clone())),
        sources: Arc::new(sources::factory::disk(product)),
        digest: Arc::new(digest::factory::blake3()),
        store: Arc::new(check_store::factory::postcard(check_store::types::Store {
            directory: root.join(asymptote_core::domain::operations::constants::CACHE),
            build: check_store::types::Build("acceptance".into()),
        })),
        repository,
    })
}
