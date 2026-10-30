//! The steps of the `down` feature: each a cucumber expression over the world, asserting the use case's own results.

use cucumber::{given, then, when};

use crate::Behavior;

use asymptote_core::domain::selection::types::Selectors;
use asymptote_core::domain::violations::types::Standing;
use asymptote_core::inbound::use_cases::check::fixtures::{
    LOG, down, down_request, lock_state, measured_product, perf_record, request, scenario,
    seed_lock, up, up_request, use_case, with_sarif,
};
use asymptote_core::inbound::use_cases::check::interface::CheckUseCase as _;
use asymptote_core::inbound::use_cases::down::interface::DownUseCase as _;
use asymptote_core::inbound::use_cases::up::interface::UpUseCase as _;
use asymptote_core::outbound::sarif::fixtures::fake::{ignored, reading, result};

/// Names the scene the next `down` step runs.
#[given(expr = "the down input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Checks and accepts the fakes' product, changes it as the input asks, checks again and runs `down`, keeping what
/// it dropped.
#[when(expr = "down runs")]
fn runs(world: &mut Behavior) {
    let product = scenario();
    let product = if world.input == "unchecked" {
        product
    } else {
        // The setup's check and up leave a report and a lock behind; one that fails is the answer.
        if let Err(error) = use_case(&product).check(request()) {
            world.answer = format!("the setup check failed: {error}");
            return;
        }
        if let Err(error) = up(&product).up(up_request(Selectors::default())) {
            world.answer = format!("the setup up failed: {error}");
            return;
        }
        if world.input == "fixed" {
            let fixed = with_sarif(product, reading(LOG, vec![result("src/a.rs", 1)]));
            fixed.sources.write("src/a.rs", "fn a() {}\n// fixed\n");
            fixed
        } else if world.input == "ignored" {
            // One violation is fixed, and a new one in another file is suppressed by an in-source ignore.
            let ignoring = with_sarif(
                product,
                reading(LOG, vec![result("src/a.rs", 1), ignored("src/b.rs", 1)]),
            );
            ignoring.sources.write("src/a.rs", "fn a() {}\n// fixed\n");
            ignoring.sources.write("src/b.rs", "fn c() {}\n");
            ignoring
        } else {
            product
        }
    };
    if world.input != "unchecked" {
        // This check leaves the report `down` reads; one that fails is the answer.
        if let Err(error) = use_case(&product).check(request()) {
            world.answer = format!("the check before down failed: {error}");
            return;
        }
    }
    world.answer = match down(&product).down(down_request()) {
        Ok(report) => std::iter::once(format!("{} dropped", report.dropped.len()))
            .chain(report.recorded.iter().map(|record| {
                format!(
                    "recorded {} {}",
                    record.key.path.0,
                    match record.key.standing {
                        Standing::Suppressed => "suppressed",
                        Standing::Reported => "reported",
                    }
                )
            }))
            .collect::<Vec<_>>()
            .join(", "),
        Err(error) => error.to_string(),
    };
}

/// Asserts the last `down` step's answer.
#[then(expr = "down answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    // The step's text, kept as the answer's type: what a cucumber argument arrives as is an owned `String`.
    let expected: Box<str> = expected.into();
    assert_eq!(&*world.answer, &*expected, "down answered otherwise");
}

/// `down` over the real adapters, bound to the product at `root`.
fn real_down(
    root: &std::path::Path,
) -> asymptote_core::inbound::use_cases::down::use_case::DownUseCaseImpl {
    use asymptote_core::application::accepted_state::service::AcceptedState;
    use asymptote_core::application::cached_report::service::CachedReport;
    use asymptote_core::inbound::use_cases::down::use_case::{DownPorts, DownUseCaseImpl};
    use asymptote_core::outbound::{check_store, clock, lock, manifest, repository, sources};
    use std::sync::Arc;
    let worktree = {
        use asymptote_core::outbound::repository::interface::RepositoryPort as _;
        repository::factory::git(
            asymptote_core::domain::manifest::types::ProductRoot(root.display().to_string().into()),
            Ok(repository::types::Worktree::default()),
        )
        .worktree()
    };
    DownUseCaseImpl::new(DownPorts {
        manifest: Arc::new(manifest::factory::toml()),
        lock: Arc::new(lock::factory::toml(
            asymptote_core::domain::manifest::types::ProductRoot(root.display().to_string().into()),
        )),
        accepted: AcceptedState::new(
            Arc::new(lock::factory::toml(
                asymptote_core::domain::manifest::types::ProductRoot(
                    root.display().to_string().into(),
                ),
            )),
            Arc::new(repository::factory::git(
                asymptote_core::domain::manifest::types::ProductRoot(
                    root.display().to_string().into(),
                ),
                worktree.clone(),
            )),
        ),
        report: CachedReport::new(Arc::new(sources::factory::disk(
            asymptote_core::domain::manifest::types::ProductRoot(root.display().to_string().into()),
        ))),
        store: Arc::new(check_store::factory::postcard(check_store::types::Store {
            directory: root.join(asymptote_core::domain::operations::constants::CACHE),
            build: check_store::types::Build("acceptance".into()),
        })),
        repository: Arc::new(repository::factory::git(
            asymptote_core::domain::manifest::types::ProductRoot(root.display().to_string().into()),
            worktree.clone(),
        )),
        clock: Arc::new(clock::factory::system()),
    })
}

/// Seeds the measured product's lock with two perf records, checks it and runs `down`, keeping how many perf
/// records it dropped and what the lock holds after.
#[when(expr = "down drains the measured product {string}")]
fn drains(world: &mut Behavior, input: String) -> Result<(), Box<dyn std::error::Error>> {
    use asymptote_core::domain::manifest::types::{Group, Groups, ProductRoot};
    // The step's text, kept as the answer's type: what a cucumber argument arrives as is an owned `String`.
    let input: Box<str> = input.into();
    let (bench, groups) = match &*input {
        "resolved" => (r#"{"c": {"cpu-ms": {"value": 100}}}"#, Groups::All),
        "held" => (
            r#"{"c": {"cpu-ms": {"value": 100}, "wall-ms": {"value": 100}}}"#,
            Groups::All,
        ),
        "other-group" => (
            r#"{"c": {"cpu-ms": {"value": 100}}}"#,
            Groups::Only(vec![Group("lint".into())]),
        ),
        other => return Err(format!("unknown draining case: {other}").into()),
    };
    let product = measured_product(bench, 0)?;
    seed_lock(
        product.path(),
        vec![perf_record("wall-ms", 100.0), perf_record("cpu-ms", 100.0)],
    )?;
    let start = ProductRoot(product.path().display().to_string().into());
    // This check leaves the report `down` reads, so it must have run a tool.
    let checked = crate::check::real_check(product.path()).check(
        asymptote_core::inbound::use_cases::check::types::CheckRequest {
            start: start.clone(),
            groups,
            ..request()
        },
    )?;
    if checked.runs.is_empty() {
        return Err("the check before down ran no tool".into());
    }
    let report = real_down(product.path()).down(
        asymptote_core::inbound::use_cases::down::types::DownRequest {
            start,
            ..down_request()
        },
    )?;
    world.answer = format!(
        "{} perf dropped; {}",
        report.perf.len(),
        lock_state(product.path())?
    );
    Ok(())
}
