//! The steps of the `up` feature: each a cucumber expression over the world, asserting the use case's own results.

use cucumber::{given, then, when};

use crate::Behavior;

use asymptote_core::domain::manifest::types::ProductRoot;
use asymptote_core::domain::selection::types::{At, Selectors};
use asymptote_core::domain::violations::types::{LineNumber, RepoPath};
use asymptote_core::inbound::use_cases::check::fixtures::{
    ignoring, lock_state, measured_product, request, scenario, scene, seed_lock, up, up_request,
    use_case,
};
use asymptote_core::inbound::use_cases::check::interface::CheckUseCase as _;
use asymptote_core::inbound::use_cases::check::types::CheckRequest;
use asymptote_core::inbound::use_cases::up::interface::UpUseCase as _;
use asymptote_core::inbound::use_cases::up::types::UpRequest;

/// Names the scene the next `up` step runs.
#[given(expr = "the up input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Checks the fakes' product and runs `up` with the selectors the input names, keeping what it recorded and left.
#[when(expr = "up runs")]
fn runs(world: &mut Behavior) {
    // "ignored": both violations are suppressed by an in-source ignore, which `up` records too.
    let product = if world.input == "ignored" {
        ignoring()
    } else {
        scenario()
    };
    if world.input != "unchecked" {
        // This check leaves the report `up` reads; one that fails is the answer.
        if let Err(error) = use_case(&product).check(request()) {
            world.answer = format!("the check before up failed: {error}");
            return;
        }
    }
    let at = |line| At {
        path: RepoPath("src/a.rs".into()),
        line: LineNumber(line),
    };
    let selectors = match world.input.as_str() {
        "at" => Selectors {
            at: vec![at(2)],
            ..Selectors::default()
        },
        "nothing" => Selectors {
            at: vec![at(9)],
            ..Selectors::default()
        },
        _ => Selectors::default(),
    };
    world.answer = match up(&product).up(up_request(selectors)) {
        Ok(report) => format!(
            "{} recorded, {} left",
            report.recorded.len(),
            report.left.len()
        ),
        Err(error) => error.to_string(),
    };
}

/// Asserts the last `up` step's answer.
#[then(expr = "up answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    // The step's text, kept as the answer's type: what a cucumber argument arrives as is an owned `String`.
    let expected: Box<str> = expected.into();
    assert_eq!(&*world.answer, &*expected, "up answered otherwise");
}

/// `up` over the real adapters, bound to the product at `root`.
fn real_up(
    root: &std::path::Path,
) -> asymptote_core::inbound::use_cases::up::use_case::UpUseCaseImpl {
    use asymptote_core::application::accepted_state::service::AcceptedState;
    use asymptote_core::application::cached_report::service::CachedReport;
    use asymptote_core::inbound::use_cases::up::use_case::{UpPorts, UpUseCaseImpl};
    use asymptote_core::outbound::{
        check_store, clock, lock, manifest, matching, repository, sources,
    };
    use std::sync::Arc;
    let worktree = {
        use asymptote_core::outbound::repository::interface::RepositoryPort as _;
        repository::factory::git(
            ProductRoot(root.display().to_string().into()),
            Ok(repository::types::Worktree::default()),
        )
        .worktree()
    };
    UpUseCaseImpl::new(UpPorts {
        manifest: Arc::new(manifest::factory::toml()),
        lock: Arc::new(lock::factory::toml(ProductRoot(
            root.display().to_string().into(),
        ))),
        accepted: AcceptedState::new(
            Arc::new(lock::factory::toml(ProductRoot(
                root.display().to_string().into(),
            ))),
            Arc::new(repository::factory::git(
                ProductRoot(root.display().to_string().into()),
                worktree.clone(),
            )),
        ),
        report: CachedReport::new(Arc::new(sources::factory::disk(ProductRoot(
            root.display().to_string().into(),
        )))),
        store: Arc::new(check_store::factory::postcard(check_store::types::Store {
            directory: root.join(asymptote_core::domain::operations::constants::CACHE),
            build: check_store::types::Build("acceptance".into()),
        })),
        repository: Arc::new(repository::factory::git(
            ProductRoot(root.display().to_string().into()),
            worktree.clone(),
        )),
        clock: Arc::new(clock::factory::system()),
        matching: Arc::new(matching::factory::globset()),
    })
}

/// Checks the measured product on disk and runs `up` for the scene's tools, keeping the perf records it wrote and the
/// verdict of the check after.
#[when(expr = "up seeds the measured product {string}")]
fn seeds(world: &mut Behavior, input: String) -> Result<(), Box<dyn std::error::Error>> {
    // The step's text, kept as the answer's type: what a cucumber argument arrives as is an owned `String`.
    let input: Box<str> = input.into();
    let scene = scene(&input)?;
    let product = measured_product(&scene.bench, scene.unit)?;
    if !scene.records.is_empty() {
        seed_lock(product.path(), scene.records)?;
    }
    let start = ProductRoot(product.path().display().to_string().into());
    let asked = || CheckRequest {
        start: start.clone(),
        ..request()
    };
    // This check leaves the report `up` reads, so it must have run a tool.
    if crate::check::real_check(product.path())
        .check(asked())?
        .runs
        .is_empty()
    {
        return Err("the check before up ran no tool".into());
    }
    let made = real_up(product.path()).up(UpRequest {
        start: start.clone(),
        selectors: Selectors {
            tools: scene.tools,
            ..Selectors::default()
        },
        ..up_request(Selectors::default())
    });
    world.answer = match made {
        Ok(report) => {
            let after = crate::check::real_check(product.path()).check(asked())?;
            format!(
                "{} perf recorded; then {:?}; {}",
                report.perf.len(),
                after.verdict,
                lock_state(product.path())?
            )
        }
        Err(error) => error.to_string(),
    };
    Ok(())
}
