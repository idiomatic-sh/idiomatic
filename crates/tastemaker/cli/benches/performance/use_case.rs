// ignore: idiomatic.arch/file-length - the cost workloads are one module of this file, which the cli spec places as it is: a `cost.rs` or a `use_case/` directory beside it reads as a resource and a command it would place; they share one tree, store and set of helpers.
//! The core's workloads: one `check` of the fixture workspace over the fakes of its ports, warm (its answer kept by the
//! runs before, so what is measured is what a run spends to find nothing changed) and cold (a use case built for each
//! round).
//!
//! What Tastemaker costs over ryo's own tree, against ryo's `lint` of the same rules (goal 2 of the design,
//! `docs/README.md`), each cost its own workload so the ratchet judges each and no ratio is asserted: cold `check`
//! (the proofs prepared, the kept answer and the source results cleared) and ryo's cold `lint` (its cache removed), warm `check`
//! and ryo's warm `lint` (each run again over what the run before kept), `validate` with no proof kept (every scenario
//! of every version in use proved: the validation fixtures' cost), and `validate` with nothing changed.
//!
//! The tree is a copy of ryo and Tastemaker in a repository of its own, so its lock can be written; cold costs take 5
//! rounds, the others 3, each the median of its rounds, every workload over the same files and page cache.

use tastemaker_core::inbound::use_cases::check::fixtures::{
    StoreFake, analysis, repository, request, use_case,
};
use tastemaker_core::inbound::use_cases::check::interface::CheckUseCase as _;
use tastemaker_core::inbound::use_cases::check::types::Fixing;

/// The core's warm and cold `check`.
pub(crate) fn measured() -> Vec<crate::measure::Measured> {
    let mut measured = vec![warm(), cold()];
    measured.extend(cost::measured());
    measured
}

/// `check` over the fixture workspace and one use case, built once and run twice first, so each round measures a run
/// that finds nothing changed and answers what was kept.
#[expect(
    clippy::expect_used,
    reason = "a benchmark's setup fails by panicking: a fixture that does not validate or check is a broken benchmark, and the message names which"
)]
fn warm() -> crate::measure::Measured {
    let check = use_case(repository(), analysis(), &StoreFake::default())
        .expect("the benchmark fixture validates");
    for _ in 0..2_u32 {
        let report = check
            .check(request(Fixing::Fix))
            .expect("the fixture checks");
        assert_eq!(
            report.report.findings.len(),
            4,
            "the fixture reports its four findings"
        );
    }
    crate::measure::rounds(
        "core/warm-check",
        || (),
        |()| {
            let report = check
                .check(request(Fixing::Report))
                .expect("the fixture checks");
            assert_eq!(
                report.report.findings.len(),
                4,
                "a warm check reports the fixture's four findings"
            );
        },
    )
}

/// `check` with its use case built for each round, so each round measures a first run.
#[expect(
    clippy::expect_used,
    reason = "a benchmark's setup fails by panicking: a fixture that does not validate or check is a broken benchmark, and the message names which"
)]
fn cold() -> crate::measure::Measured {
    crate::measure::rounds_of(
        "core/cold-check",
        crate::measure::COLD_ROUNDS,
        || {
            use_case(repository(), analysis(), &StoreFake::default())
                .expect("the benchmark fixture validates")
        },
        |check| {
            let report = check
                .check(request(Fixing::Fix))
                .expect("the fixture checks");
            assert_eq!(
                report.report.findings.len(),
                4,
                "a cold check reports the fixture's four findings"
            );
        },
    )
}

/// What Tastemaker costs over ryo's own tree.
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a benchmark's setup fails by panicking: a scene that cannot be written, a run that cannot start or a store that cannot be rewritten is a broken benchmark, and the message names which"
)]
#[expect(
    clippy::inline_modules,
    reason = "the cli spec places this bench's files: a `cost.rs` or a `use_case/` directory beside it reads as a resource and a command it would place, so the cost workloads stay a module of this file"
)]
mod cost {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use tastemaker_core::domain::files::types::{DirectoryName, MachinePath};
    use tastemaker_core::domain::settings::types::{Build, Store};
    use tastemaker_core::domain::writing::types::Writing;
    use tastemaker_core::outbound::analysis::fixtures::fake::AnalysisFake;
    use tastemaker_core::outbound::analysis::interface::AnalysisPort;
    use tastemaker_core::outbound::analysis::types::{AnalysisRequest, AnalysisSettings, Memory};
    use tastemaker_core::outbound::check_store::interface::CheckStorePort as _;
    use tastemaker_core::outbound::check_store::types::Checked;
    use tastemaker_core::outbound::validate_store::interface::ValidateStorePort as _;
    use tastemaker_core::outbound::{check_store, validate_store};

    use crate::measure::{Measured, rounds_of};

    /// The store's directory beneath a project's root.
    const STORE: &str = ".tastemaker_cache";
    /// ryo's own directory within a store.
    const RYO_STORE: &str = "ryo";

    /// The rounds of a cold cost.
    const COLD_ROUNDS: usize = 5;
    /// The rounds of every other cost.
    const WARM_ROUNDS: usize = 3;

    /// The tree measured, beside its package, copied into a repository of its own so its lock can be written.
    fn target() -> tempfile::TempDir {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let copy = tempfile::tempdir().expect("a temporary directory");
        copy_tree(&source.join("ryo"), &copy.path().join("crates/ryo"));
        copy_tree(
            &source.join("../tastemaker"),
            &copy.path().join("tastemaker"),
        );
        std::fs::create_dir_all(copy.path().join(".git")).expect("a .git directory");
        std::fs::write(
            copy.path().join(".gitignore"),
            "target/\n.tastemaker_cache/\n",
        )
        .expect("a .gitignore");
        copy
    }

    /// `from` copied to `to`, its build trees left out.
    fn copy_tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).expect("the directory");
        for entry in std::fs::read_dir(from)
            .expect("the directory reads")
            .flatten()
        {
            let name = entry.file_name();
            if name == "target" || name == ".tastemaker_cache" || name == ".ryo_cache" {
                continue;
            }
            let path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                copy_tree(&path, &to.join(&name));
            } else {
                let _: u64 = std::fs::copy(&path, to.join(&name)).expect("the file copies");
            }
        }
    }

    /// The edge of a command run at `root`.
    fn edge(root: &Path) -> tastemaker_cli::edge::Edge {
        tastemaker_cli::resources::edge_at(
            MachinePath(root.display().to_string().into()),
            Build("cost".into()),
            tastemaker_cli::output::Format::Quiet,
            Writing::Write,
        )
    }

    /// `tm check -q` at `root`.
    fn check(root: &Path, line: &[&str]) {
        let command = <tastemaker_cli::resources::workspace::check::command::CheckArguments as clap::Args>::augment_args(clap::Command::new("check"));
        let arguments = <tastemaker_cli::resources::workspace::check::command::CheckArguments as clap::FromArgMatches>::from_arg_matches(
            &command.try_get_matches_from(line).expect("the line parses"),
        )
        .expect("the arguments");
        let outcome =
            tastemaker_cli::resources::workspace::check::command::run(arguments, &edge(root));
        // The copy carries findings of its own, so the check may end in a failure: what it must not end in is an error.
        assert_ne!(
            outcome,
            tastemaker_cli::outcome::Outcome::Error,
            "the measured check `{line:?}` runs"
        );
    }

    /// The request `check` sends from `root`, as a cold check of the core sends it, recorded by the fake analysis it is
    /// built over and writing nothing, and the ryo cache the reference keeps in its place.
    fn request(root: &Path) -> (AnalysisRequest, PathBuf) {
        use tastemaker_core::inbound::use_cases::check::interface::CheckUseCase as _;
        use tastemaker_core::inbound::use_cases::check::types::{CheckRequest, Fixing};
        use tastemaker_core::inbound::use_cases::check::use_case::{CheckPorts, CheckUseCaseImpl};
        use tastemaker_core::outbound::{
            digest, manifest, matching, packages, repository, scheduler, templates,
        };
        forget_answer(root);
        let recording = Arc::new(AnalysisFake::default());
        let repository: Arc<dyn repository::interface::RepositoryPort> = Arc::new(
            repository::factory::disk(Vec::new(), DirectoryName(STORE.into())),
        );
        let store_at = Store {
            writing: Writing::DryRun,
            ..store_at(root)
        };
        let digest: Arc<dyn digest::interface::DigestPort> = Arc::new(digest::factory::blake3());
        let matching: Arc<dyn matching::interface::MatchingPort> =
            Arc::new(matching::factory::globset());
        let check = CheckUseCaseImpl::new(CheckPorts {
            reporting: Arc::new(
                tastemaker_core::application::reporting::service::ReportingService::new(
                    Arc::new(scheduler::factory::rayon(scheduler::factory::pool(
                        scheduler::types::WorkerCount(2),
                    ))),
                    Arc::clone(&matching),
                    Arc::clone(&digest),
                ),
            ),
            repository,
            manifest: Arc::new(manifest::factory::toml()),
            packages: Arc::new(packages::factory::path()),
            analysis: Arc::<AnalysisFake>::clone(&recording),
            counting: Arc::<AnalysisFake>::clone(&recording),
            store: Arc::new(check_store::factory::postcard(&store_at)),
            proofs: Arc::new(validate_store::factory::postcard(&store_at)),
            digest,
            matching,
            templates: Arc::new(templates::factory::minijinja()),
        });
        let _report = check
            .check(CheckRequest {
                start: MachinePath(root.display().to_string().into()),
                within: Vec::new(),
                fixing: Fixing::Fix,
                writing: Writing::DryRun,
            })
            .expect("the core checks the copy");
        let request = recording
            .sent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .drain(..)
            .next()
            .expect("check sent a lint call");
        (
            request,
            PathBuf::from(&*reference(root).directory.0).join(RYO_STORE),
        )
    }

    /// Removes `directory`, whether or not it is there.
    fn remove(directory: &Path) {
        match std::fs::remove_dir_all(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("{}: {error}", directory.display()),
        }
    }

    /// Where `root`'s store is, under the build the costs run as.
    fn store_at(root: &Path) -> Store {
        Store {
            directory: MachinePath(root.join(STORE).display().to_string().into()),
            build: Build("cost".into()),
            writing: Writing::Write,
        }
    }

    /// The store the reference lint of `root` keeps ryo's work in, beside the store `check` keeps.
    fn reference(root: &Path) -> Store {
        Store {
            directory: MachinePath(
                root.join(STORE)
                    .join("reference")
                    .display()
                    .to_string()
                    .into(),
            ),
            ..store_at(root)
        }
    }

    /// Removes the answer the last check of `root` kept, so the next check runs again.
    fn forget_answer(root: &Path) {
        let store = check_store::factory::postcard(&store_at(root));
        store
            .keep_checked(&Checked::default())
            .expect("the store is rewritten");
    }

    /// Forgets every proof `root`'s store keeps, so the next validation proves every version again.
    fn forget_proofs(root: &Path) {
        let store = validate_store::factory::postcard(&store_at(root));
        store.keep_definitions(&[]).expect("the store is rewritten");
    }

    /// Proves the copied package before a check is measured.
    fn validate(root: &Path) {
        let command = <tastemaker_cli::resources::workspace::validate::command::ValidateArguments as clap::Args>::augment_args(clap::Command::new("validate"));
        let arguments = <tastemaker_cli::resources::workspace::validate::command::ValidateArguments as clap::FromArgMatches>::from_arg_matches(
            &command.try_get_matches_from(["validate"]).expect("the line parses"),
        )
        .expect("the arguments");
        assert_eq!(
            tastemaker_cli::resources::workspace::validate::command::run(arguments, &edge(root)),
            tastemaker_cli::outcome::Outcome::Success,
            "the validation prerequisite succeeds"
        );
    }

    /// The six costs: `cost/validate-cold`, `cost/cold-check`, `cost/ryo-cold-lint`, `cost/warm-check`,
    /// `cost/ryo-warm-lint` and `cost/validate-unchanged`.
    pub(crate) fn measured() -> Vec<Measured> {
        let tree = target();
        let root = tree.path().join("crates/ryo");
        check(&root, &["check", "--fix"]);
        // The languages the edge links, in its order, so the store `check` kept is this build's.
        let analysis: Arc<dyn AnalysisPort> =
            Arc::new(tastemaker_core::outbound::analysis::factory::ryo_kept(
                ryo::linter::Languages(vec![
                    #[cfg(feature = "css")]
                    ryo_css::linked(),
                    #[cfg(feature = "go")]
                    ryo_go::linked(),
                    #[cfg(feature = "html")]
                    ryo_html::linked(),
                    #[cfg(feature = "javascript")]
                    ryo_javascript::linked(),
                    #[cfg(feature = "json")]
                    ryo_json::linked(),
                    #[cfg(feature = "markdown")]
                    ryo_markdown::linked(),
                    #[cfg(feature = "python")]
                    ryo_python::linked(),
                    #[cfg(feature = "rust")]
                    ryo_rust::linked(),
                    #[cfg(feature = "toml")]
                    ryo_toml::linked(),
                    #[cfg(feature = "typescript")]
                    ryo_typescript::linked(),
                ]),
                AnalysisSettings {
                    pool: tastemaker_core::outbound::scheduler::factory::pool(
                        tastemaker_core::outbound::scheduler::types::WorkerCount(
                            std::thread::available_parallelism().map_or(1, std::num::NonZero::get),
                        ),
                    ),
                    memory: Memory::Machine,
                },
                &reference(&root),
            ));
        // Every scenario of every version in use, proved with no proof kept: the validation fixtures' cost.
        let validate_cold = rounds_of(
            "cost/validate-cold",
            COLD_ROUNDS,
            || forget_proofs(&root),
            |()| validate(&root),
        );
        let (request, ryo_cache) = request(&root);
        // ryo's `lint` of the request `check` sends.
        let lint = || drop(analysis.lint(request.clone()).expect("the lint runs"));
        let cold_check = rounds_of(
            "cost/cold-check",
            COLD_ROUNDS,
            || cold(&root),
            |()| check(&root, &["check"]),
        );
        check(&root, &["check"]);
        let warm_check = rounds_of(
            "cost/warm-check",
            WARM_ROUNDS,
            || (),
            |()| check(&root, &["check"]),
        );
        let ryo_cold_lint = rounds_of(
            "cost/ryo-cold-lint",
            COLD_ROUNDS,
            || remove(&ryo_cache),
            |()| lint(),
        );
        lint();
        let ryo_warm_lint = rounds_of("cost/ryo-warm-lint", WARM_ROUNDS, || (), |()| lint());
        validate(&root);
        let validate_unchanged = rounds_of(
            "cost/validate-unchanged",
            WARM_ROUNDS,
            || (),
            |()| validate(&root),
        );
        vec![
            validate_cold,
            cold_check,
            ryo_cold_lint,
            warm_check,
            ryo_warm_lint,
            validate_unchanged,
        ]
    }

    /// Clears the kept answer and the source results, retaining the validation prerequisite outside the measurement.
    fn cold(root: &Path) {
        let proofs = validate_store::factory::postcard(&store_at(root)).definitions();
        assert!(
            !proofs.0.is_empty(),
            "the cold setup retains validation's proofs"
        );
        forget_answer(root);
        remove(&root.join(STORE).join(RYO_STORE));
    }
}
