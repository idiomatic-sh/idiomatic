//! The steps of the `validate` feature: each a cucumber expression over the world, asserting the use case's own results.

use cucumber::{given, then, when};

use crate::Behavior;

/// Setup failures reported by the scenario runner.
type Failure = Box<dyn std::error::Error + Send + Sync>;
use std::sync::Arc;

use tastemaker_core::domain::files::types::MachinePath;
use tastemaker_core::domain::proofs::types::Proof;
use tastemaker_core::inbound::use_cases::check::fixtures::RULE;
use tastemaker_core::inbound::use_cases::validate::interface::ValidateUseCase as _;
use tastemaker_core::inbound::use_cases::validate::types::ValidateRequest;
use tastemaker_core::inbound::use_cases::validate::use_case::{ValidatePorts, ValidateUseCaseImpl};
use tastemaker_core::outbound::analysis::fixtures::fake::AnalysisFake;
use tastemaker_core::outbound::digest::fixtures::fake::DigestFake;
use tastemaker_core::outbound::packages::fixtures::fake::PackagesFake;
use tastemaker_core::outbound::repository::fixtures::fake::RepositoryFake;
use tastemaker_core::outbound::templates::fixtures::fake::TemplatesFake;

/// Sets what the `validate` scenario hands the use case.
#[given(expr = "the validate input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `validate` on the scenario's input and records its answer.
#[when(expr = "validate runs")]
fn runs(world: &mut Behavior) -> Result<(), Failure> {
    if world.input.starts_with("disk-") {
        world.answer = on_disk(&world.input)?;
        return Ok(());
    }
    let package = world.input.clone();
    let mut engine = AnalysisFake::default();
    drop(engine.rows.insert(
        "MARKER".into(),
        vec![("src/a/errors.txt".into(), "0..6".into())],
    ));
    let repository = Arc::new(RepositoryFake::of(&[(
        "tastemaker.toml",
        &format!(
            "workspace = {{ name = \"w\", dependencies = [{{ package = \"{package}\", path = \"{package}\" }}] }}\n"
        ),
    )]));
    let scheduler = Arc::new(tastemaker_core::outbound::scheduler::fixtures::fake::SchedulerFake);
    let digest = Arc::new(DigestFake);
    let matching: Arc<dyn tastemaker_core::outbound::matching::interface::MatchingPort> =
        Arc::new(tastemaker_core::outbound::matching::factory::globset());
    let reporting = Arc::new(
        tastemaker_core::application::reporting::service::ReportingService::new(
            Arc::<tastemaker_core::outbound::scheduler::fixtures::fake::SchedulerFake>::clone(
                &scheduler,
            ),
            Arc::clone(&matching),
            Arc::<DigestFake>::clone(&digest),
        ),
    );
    let validate = ValidateUseCaseImpl::new(ValidatePorts {
        scheduler,
        reporting,
        fixtures: Arc::<RepositoryFake>::clone(&repository),
        repository,
        settings: tastemaker_core::domain::settings::types::Store {
            directory: MachinePath("/fake/.tastemaker_cache".into()),
            build: tastemaker_core::domain::settings::types::Build("bdd".into()),
            writing: tastemaker_core::domain::writing::types::Writing::Write,
        },
        manifest: Arc::new(tastemaker_core::outbound::manifest::factory::toml()),
        packages: Arc::new(PackagesFake::of(
            "p",
            &[
                ("package.toml", "name = \"p\"\n"),
                ("rules/marker/0.1.0/rule.toml", RULE),
            ],
        )),
        analysis: Arc::new(engine),
        store: Arc::new(
            tastemaker_core::outbound::validate_store::fixtures::fake::ValidateStoreFake::default(),
        ),
        digest: Arc::new(DigestFake),
        matching: Arc::new(tastemaker_core::outbound::matching::factory::globset()),
        templates: Arc::new(TemplatesFake),
    });
    world.answer = match validate.validate(ValidateRequest {
        start: MachinePath("/fake".into()),
        only: Vec::new(),
    }) {
        Ok(report) => format!(
            "{} proven",
            report
                .proved
                .iter()
                .filter(|proved| proved.proof == Proof::Proven)
                .count()
        ),
        Err(error) => error.to_string(),
    };
    Ok(())
}

/// Compares the answer of `validate` with the one the scenario expects.
#[then(expr = "validate answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    // The step's text, kept as the answer's type: what a cucumber argument arrives as is an owned `String`.
    let expected: Box<str> = expected.into();
    assert_eq!(
        &*world.answer, &*expected,
        "validate answers what the scenario expects"
    );
}

/// Writes the scene `on_disk` validates at `root`: a git repository whose workspace uses one package, with a template of
/// two scenarios.
fn scene(root: &std::path::Path) -> Result<(), Failure> {
    assert!(
        std::process::Command::new("git")
            .args(["init", "-q"])
            .arg(root)
            .status()?
            .success(),
        "git initializes the scene's repository"
    );
    let template = root.join("p/templates/body/0.1.0");
    std::fs::create_dir_all(&template)?;
    std::fs::write(
        root.join("tastemaker.toml"),
        "workspace={name=\"scene\",dependencies=[{package=\"p\",path=\"p\"}]}\nclosed=false\n",
    )?;
    std::fs::write(root.join("p/package.toml"), "name=\"p\"\n")?;
    std::fs::write(
        template.join("template.toml"),
        "description=\"Text\"\nwhy=\"Text\"\ncontent=\"body\"\n\n[[scenarios]]\nid=\"body-file-present\"\nscenario=\"The file is there.\"\ngiven.file=\"body.txt\"\ngiven.files.\"body.txt\"=\"body\\n\"\n\n[[scenarios]]\nid=\"body-file-missing\"\nscenario=\"The file is missing.\"\ngiven.file=\"body.txt\"\nthen.findings=[{rule=\"body#count\",path=\"tastemaker.toml\"}]\n",
    )?;
    Ok(())
}

/// The real package reader, template engine and persisted proof store validate an on-disk package.
fn on_disk(input: &str) -> Result<String, Failure> {
    use tastemaker_core::outbound::{
        analysis, digest, manifest, matching, packages, repository, templates,
    };
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    scene(root)?;
    let repository: Arc<dyn repository::interface::RepositoryPort> =
        Arc::new(repository::factory::disk(
            Vec::new(),
            tastemaker_core::domain::files::types::DirectoryName(".tastemaker_cache".into()),
        ));
    // Each scenario's project is written in memory, and the report of its check reads it there.
    let fixtures: Arc<dyn repository::interface::RepositoryPort> =
        Arc::new(repository::factory::memory(
            Vec::new(),
            tastemaker_core::domain::files::types::DirectoryName(".tastemaker_cache".into()),
        ));
    let scheduler = Arc::new(tastemaker_core::outbound::scheduler::fixtures::fake::SchedulerFake);
    let digest: Arc<dyn digest::interface::DigestPort> = Arc::new(digest::factory::blake3());
    let matching: Arc<dyn matching::interface::MatchingPort> =
        Arc::new(matching::factory::globset());
    let reporting = Arc::new(
        tastemaker_core::application::reporting::service::ReportingService::new(
            Arc::<tastemaker_core::outbound::scheduler::fixtures::fake::SchedulerFake>::clone(
                &scheduler,
            ),
            Arc::clone(&matching),
            Arc::clone(&digest),
        ),
    );
    let store_at = tastemaker_core::domain::settings::types::Store {
        directory: MachinePath(root.join(".tastemaker_cache").display().to_string().into()),
        build: tastemaker_core::domain::settings::types::Build("bdd".into()),
        writing: tastemaker_core::domain::writing::types::Writing::Write,
    };
    let validate = ValidateUseCaseImpl::new(ValidatePorts {
        scheduler,
        reporting,
        repository,
        settings: store_at.clone(),
        fixtures,
        manifest: Arc::new(manifest::factory::toml()),
        packages: Arc::new(packages::factory::path()),
        analysis: Arc::new(analysis::factory::ryo(
            ryo::linter::Languages(Vec::new()),
            analysis::types::AnalysisSettings {
                pool: tastemaker_core::outbound::scheduler::factory::pool(
                    tastemaker_core::outbound::scheduler::types::WorkerCount(1),
                ),
                memory: analysis::types::Memory::Machine,
            },
        )),
        store: Arc::new(tastemaker_core::outbound::validate_store::factory::postcard(&store_at)),
        digest: Arc::new(digest::factory::blake3()),
        matching: Arc::new(matching::factory::globset()),
        templates: Arc::new(templates::factory::minijinja()),
    });
    let filter = if input == "disk-unmatched" {
        "no-such-definition"
    } else {
        "/template/body@"
    };
    Ok(
        match validate.validate(ValidateRequest {
            start: MachinePath(root.display().to_string().into()),
            only: vec![
                tastemaker_core::inbound::use_cases::validate::types::VersionFilter(filter.into()),
            ],
        }) {
            Ok(report) => format!(
                "{} proven",
                report
                    .proved
                    .iter()
                    .filter(|proof| proof.proof == Proof::Proven)
                    .count()
            ),
            Err(error) => error.to_string(),
        },
    )
}
