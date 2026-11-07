// ignore: idiomatic.arch/file-length - these step bindings build and inspect one check acceptance world across package placement, reporting and injected source inventory; the shared world and step vocabulary remain together so feature scenarios exercise the same workflow.
//! The steps of the `check` feature: each a cucumber expression over the world, asserting the use case's own results.

use crate::Behavior;
use cucumber::{given, then, when};
use std::sync::Arc;
/// What an acceptance step fails with: any error the use cases or the filesystem raise.
type Failure = Box<dyn std::error::Error + Send + Sync>;

use tastemaker_core::domain::files::types::{FileText, MachinePath};
use tastemaker_core::domain::paths::types::RepoPath;
use tastemaker_core::inbound::use_cases::check::errors::CheckError;
use tastemaker_core::inbound::use_cases::check::fixtures::{analysis, repository, use_case};
use tastemaker_core::inbound::use_cases::check::interface::CheckUseCase as _;
use tastemaker_core::inbound::use_cases::check::types::{CheckRequest, Fixing};

/// A TOML file whose second line's finding is ignored, and whose third line's ignore covers nothing.
const IGNORED: &str = "kept = 1\nother = 2 # ignore: p/marker - accepted here\nstale = 3 # ignore: p/marker - nothing here\n";

/// The `ignored` input: three markers in a TOML file, one on a line no ignore covers, one under an ignore at the end of its
/// line, and an ignore at the end of the last line that covers no finding.
fn ignored(
    files: &tastemaker_core::outbound::repository::fixtures::fake::RepositoryFake,
    found: &mut tastemaker_core::outbound::analysis::fixtures::fake::AnalysisFake,
) {
    drop(
        files
            .files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(RepoPath("notes.toml".into()), FileText(IGNORED.into())),
    );
    if let Some(rows) = found.rows.get_mut("MARKER") {
        rows.extend([
            ("notes.toml".into(), "0..8".into()),
            ("notes.toml".into(), "9..18".into()),
        ]);
    }
    // The comments of the file, as the shared `comment` relation gives them: each its span, whether it is documentation, and its content.
    drop(found.rows.insert(
        "key(\"span\", \"doc\", \"text\")".into(),
        vec![
            (
                "notes.toml".into(),
                "19..53,false, ignore: p/marker - accepted here".into(),
            ),
            (
                "notes.toml".into(),
                "64..97,false, ignore: p/marker - nothing here".into(),
            ),
        ],
    ));
}

/// Sets what the `check` scenario hands the use case.
#[given(expr = "the check input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `check` on the scenario's input and records its answer.
#[when(expr = "check runs")]
fn runs(world: &mut Behavior) -> Result<(), Failure> {
    if let Some(project) = &world.project {
        world.answer = real_check(project.path());
        return Ok(());
    }
    let mut files = repository();
    let mut found = analysis();
    if world.input == "nowhere" {
        files.workspace =
            tastemaker_core::outbound::repository::fixtures::fake::FakeWorkspace::Missing;
    }
    if world.input == "ignored" {
        ignored(&files, &mut found);
    }
    let store =
        Arc::new(tastemaker_core::inbound::use_cases::check::fixtures::StoreFake::default());
    let held = files.clone();
    let check = use_case(files, found, &store)?;
    if world.input == "unsaved" {
        // A store whose saves fail from here on: the check answers as it would, and only says the save failed.
        *store
            .answers
            .unwritable
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
    }
    if world.input == "unproven" {
        for rows in std::iter::once(
            &mut *store
                .proofs
                .tables
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ) {
            rows.clear();
        }
    }
    let dry = world.input == "dry-run";
    world.answer = match check.check(CheckRequest {
        start: MachinePath("/fake".into()),
        within: Vec::new(),
        fixing: Fixing::Fix,
        writing: if dry {
            tastemaker_core::domain::writing::types::Writing::DryRun
        } else {
            tastemaker_core::domain::writing::types::Writing::Write
        },
    }) {
        // Under `-n/--dry-run` the answer is whether the lock `--fix` would rewrite was written; the store's own adapter
        // writes nothing under it.
        Ok(report) if dry => {
            let locked = held
                .files
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .contains_key(&RepoPath("tastemaker.lock".into()));
            format!("{:?}, lock written {locked}", report.lock)
        }
        Ok(report) => {
            let mut rules = report
                .report
                .findings
                .iter()
                .map(|finding| tastemaker_core::domain::findings::service::printed(&finding.rule).0)
                .collect::<Vec<_>>();
            rules.sort();
            let suppressed = report.report.suppressed.len();
            if suppressed == 0 {
                rules.join(" ")
            } else {
                format!("{}, suppressed {suppressed}", rules.join(" "))
            }
        }
        Err(error) => {
            if world.input == "unproven" {
                assert!(
                    matches!(error, CheckError::Proof { .. }),
                    "a check without a current proof is refused for it"
                );
            }
            error.to_string()
        }
    };
    Ok(())
}

/// Checks the real consumer at `project` and answers what the check found or why it refused.
fn real_check(project: &std::path::Path) -> String {
    let (check, _) = real_services(project, "proof-behavior");
    match check.check(CheckRequest {
        start: MachinePath(project.display().to_string().into()),
        within: Vec::new(),
        fixing: Fixing::Fix,
        writing: tastemaker_core::domain::writing::types::Writing::Write,
    }) {
        Ok(report) if report.report.findings.is_empty() => "clean".to_owned(),
        Ok(report) => format!("unexpected findings: {:?}", report.report.findings),
        Err(CheckError::Proof { .. }) => "proof refused".to_owned(),
        Err(CheckError::Resolution { .. }) => "unresolved dependency".to_owned(),
        Err(error) => error.to_string(),
    }
}

/// The real core boundaries, with a filesystem-only rule so no syntax language is needed, keeping their store for `build`
/// in the cache directory of the workspace at `start`.
pub(super) fn real_services(
    start: &std::path::Path,
    build: &str,
) -> (
    tastemaker_core::inbound::use_cases::check::use_case::CheckUseCaseImpl,
    tastemaker_core::inbound::use_cases::validate::use_case::ValidateUseCaseImpl,
) {
    use tastemaker_core::inbound::use_cases::check::use_case::{CheckPorts, CheckUseCaseImpl};
    use tastemaker_core::inbound::use_cases::validate::use_case::{
        ValidatePorts, ValidateUseCaseImpl,
    };
    use tastemaker_core::outbound::{
        analysis, check_store, digest, manifest, matching, packages, repository, scheduler,
        templates, validate_store,
    };
    let repository: Arc<dyn repository::interface::RepositoryPort> =
        Arc::new(repository::factory::disk(
            Vec::new(),
            tastemaker_core::domain::files::types::DirectoryName(".tastemaker_cache".into()),
        ));
    let manifest: Arc<dyn manifest::interface::ManifestPort> = Arc::new(manifest::factory::toml());
    let packages: Arc<dyn packages::interface::PackagesPort> = Arc::new(packages::factory::path());
    let analysis: Arc<dyn analysis::interface::AnalysisPort> = Arc::new(analysis::factory::ryo(
        ryo::linter::Languages(Vec::new()),
        analysis::types::AnalysisSettings {
            pool: scheduler::factory::pool(scheduler::types::WorkerCount(1)),
            memory: analysis::types::Memory::Machine,
        },
    ));
    // Where the store is, and the build that writes it: values only, each store opens its own file.
    let store_at = tastemaker_core::domain::settings::types::Store {
        directory: MachinePath(start.join(".tastemaker_cache").display().to_string().into()),
        build: tastemaker_core::domain::settings::types::Build(build.into()),
        writing: tastemaker_core::domain::writing::types::Writing::Write,
    };
    let digest: Arc<dyn digest::interface::DigestPort> = Arc::new(digest::factory::blake3());
    let matching: Arc<dyn matching::interface::MatchingPort> =
        Arc::new(matching::factory::globset());
    let templates: Arc<dyn templates::interface::TemplatesPort> =
        Arc::new(templates::factory::minijinja());
    let scheduler: Arc<dyn scheduler::interface::SchedulerPort> = Arc::new(
        scheduler::factory::rayon(scheduler::factory::pool(scheduler::types::WorkerCount(2))),
    );
    let reporting = Arc::new(
        tastemaker_core::application::reporting::service::ReportingService::new(
            Arc::clone(&scheduler),
            Arc::clone(&matching),
            Arc::clone(&digest),
        ),
    );
    // Each scenario's project is written in memory, and the report of its check reads it there.
    let fixtures: Arc<dyn repository::interface::RepositoryPort> =
        Arc::new(repository::factory::memory(
            Vec::new(),
            tastemaker_core::domain::files::types::DirectoryName(".tastemaker_cache".into()),
        ));
    let fixture_reporting = Arc::new(
        tastemaker_core::application::reporting::service::ReportingService::new(
            Arc::clone(&scheduler),
            Arc::clone(&matching),
            Arc::clone(&digest),
        ),
    );
    (
        CheckUseCaseImpl::new(CheckPorts {
            reporting: Arc::clone(&reporting),
            repository: Arc::clone(&repository),
            manifest: Arc::clone(&manifest),
            packages: Arc::clone(&packages),
            analysis: Arc::clone(&analysis),
            counting: Arc::clone(&analysis),
            store: Arc::new(check_store::factory::postcard(&store_at)),
            proofs: Arc::new(validate_store::factory::postcard(&store_at)),
            digest: Arc::clone(&digest),
            matching: Arc::clone(&matching),
            templates: Arc::clone(&templates),
        }),
        ValidateUseCaseImpl::new(ValidatePorts {
            scheduler: Arc::clone(&scheduler),
            reporting: fixture_reporting,
            repository,
            fixtures,
            settings: store_at.clone(),
            manifest,
            packages,
            analysis,
            store: Arc::new(validate_store::factory::postcard(&store_at)),
            digest,
            matching,
            templates,
        }),
    )
}

/// A filesystem rule that reports `src/bad.txt`, with a scenario for the file it allows and the one it reports.
const REAL_RULE: &str = r#"description = "A prohibited file is reported."
why = "A consumer needs a real rule with a scenario."
query = '''
let found = select * from "filesystem.file" where path = "src/bad.txt";
emit found subject "path" key("path");
'''
[diagnostic]
primary = "path"
message = "a prohibited file"
help = "Remove it."
[[scenarios]]
id = "allowed-file"
scenario = "A file that is not the prohibited one is not reported."
given.include = ["**"]
given.files."src/good.txt" = "good\n"
[[scenarios]]
id = "prohibited-file"
scenario = "The prohibited file is reported."
given.include = ["**"]
given.files."src/bad.txt" = "bad\n"
"#;

/// Writes `text` to `path` under `root`, making the directories it needs.
fn write(root: &std::path::Path, path: &str, text: &str) -> Result<(), Failure> {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().ok_or("an input needs a parent directory")?)?;
    std::fs::write(file, text).map_err(Into::into)
}

/// Lays out a real consumer whose referenced rule has an incorrect case, in a repository of its own.
#[given("a real consumer whose referenced rule has an incorrect case")]
fn real_consumer(world: &mut Behavior) -> Result<(), Failure> {
    let project = tempfile::tempdir()?;
    let root = project.path();
    std::fs::create_dir_all(root.join(".git"))?;
    write(
        root,
        "tastemaker.toml",
        r#"workspace = { name = "consumer", dependencies = [{ package = "p", path = "p" }] }
closed = false
specs = [{ spec = "p/part", include = ["src"] }]
templates = [{ template = "p/module", file = "src/notes.txt" }]
"#,
    )?;
    write(
        root,
        "p/package.toml",
        "name = \"p\"\ndependencies = [{ package = \"a\", path = \"../a\" }]\n",
    )?;
    write(root, "a/package.toml", "name = \"a\"\n")?;
    write(
        root,
        "p/specs/part/0.1.0/spec.toml",
        r#"description = "A part checked for prohibited files."
why = "Parts must be checked."
closed = false
rules = [{ rule = "a/marker@0.1.0", include = ["**"], description = "Report prohibited files." }]

[[scenarios]]
id = "part-without-a-prohibited-file"
scenario = "A part that holds no prohibited file has no finding."
given.at = "src"
given.files."src/okay.txt" = "okay\n"

[[scenarios]]
id = "part-with-the-prohibited-file"
scenario = "A part that holds the prohibited file is reported."
given.at = "src"
given.files."src/bad.txt" = "bad\n"
then.findings = [{ rule = "marker", path = "src/bad.txt" }]
"#,
    )?;
    write(
        root,
        "p/templates/module/0.1.0/template.toml",
        &format!(
            r#"description = "A module."
why = "A module needs a body."
match = "text"
content = "okay\n"

[[scenarios]]
id = "module-placed-with-its-support"
scenario = "A module placed beside its support file: the module is its text, and the support file, which no linked language reads, holds its template's text."
given.files."module.txt" = "okay\n"
given.files."support.txt" = "support\n"
given.files."tastemaker.toml" = '''
workspace = {{ name = "scenario", dependencies = [{{ package = "p", path = {} }}] }}
closed = false
templates = [{{ template = "p/module", file = "module.txt" }}, {{ template = "p/support", file = "support.txt" }}]
'''

[[scenarios]]
id = "module-alone"
scenario = "A module that is its text has no finding."
given.file = "module.txt"
given.files."module.txt" = "okay\n"

[[scenarios]]
id = "module-missing"
scenario = "A module that is placed and missing is reported."
given.file = "module.txt"
then.findings = [{{ rule = "module#count", path = "tastemaker.toml" }}]
"#,
            toml::Value::String(root.join("p").display().to_string())
        ),
    )?;
    write(root, "src/notes.txt", "okay\n")?;
    write(root, "a/rules/marker/0.1.0/rule.toml", REAL_RULE)?;
    write(
        root,
        "p/templates/support/0.1.0/template.toml",
        "description = \"Support.\"\nwhy = \"A scenario needs support.\"\ncontent = \"support\\n\"\n\n[[scenarios]]\nid = \"support-file-present\"\nscenario = \"The support file is there and holds its template's text, which is all a file no linked language reads is held to.\"\ngiven.file = \"support.txt\"\ngiven.files.\"support.txt\" = \"support\\n\"\n\n[[scenarios]]\nid = \"support-file-departs\"\nscenario = \"The support file does not hold its template's text: no linked language reads it, so it is `#unknown` at error, never a silent pass.\"\ngiven.file = \"support.txt\"\ngiven.files.\"support.txt\" = \"other\\n\"\nthen.findings = [{ rule = \"support#unknown\", path = \"support.txt\" }]\n\n[[scenarios]]\nid = \"support-file-missing\"\nscenario = \"The support file is missing.\"\ngiven.file = \"support.txt\"\nthen.findings = [{ rule = \"support#count\", path = \"tastemaker.toml\" }]\n",
    )?;
    world.project = Some(project);
    Ok(())
}

/// The directory of the real consumer the scenario was given.
fn root(world: &Behavior) -> Result<&std::path::Path, Failure> {
    let project = world.project.as_ref().ok_or("a consumer must be given")?;
    Ok(project.path())
}

/// Drops the dependency the consumer's package names.
#[when("the referenced package dependency is removed")]
fn remove_dependency(world: &mut Behavior) -> Result<(), Failure> {
    write(root(world)?, "p/package.toml", "name = \"p\"\n")
}

/// Validates the real consumer and records whether every proof held.
#[when("the real consumer validates")]
fn real_validate(world: &mut Behavior) -> Result<(), Failure> {
    use tastemaker_core::inbound::use_cases::validate::interface::ValidateUseCase as _;
    let (_, validate) = real_services(root(world)?, "proof-behavior");
    let report = validate.validate(
        tastemaker_core::inbound::use_cases::validate::types::ValidateRequest {
            start: MachinePath(root(world)?.display().to_string().into()),
            only: Vec::new(),
        },
    )?;
    assert_eq!(
        report.proved.len(),
        4,
        "all used definitions and fixture support are validated"
    );
    let proven = report
        .proved
        .iter()
        .all(|proved| proved.proof == tastemaker_core::domain::proofs::types::Proof::Proven);
    if proven { "proven" } else { "failed" }.clone_into(&mut world.answer);
    Ok(())
}

/// Writes the corrected case into the referenced rule.
#[when("the referenced rule case is corrected")]
fn correct_case(world: &mut Behavior) -> Result<(), Failure> {
    write(
        root(world)?,
        "a/rules/marker/0.1.0/rule.toml",
        &REAL_RULE.replace(
            "bad\\n\"",
            "bad\\n\"\nthen.findings = [{ path = \"src/bad.txt\" }]",
        ),
    )
}

/// Adds a file the proofs read.
#[when(expr = "the proof input {string} is added")]
fn add_proof_input(world: &mut Behavior, path: String) -> Result<(), Failure> {
    let path = RepoPath(path.into());
    write(root(world)?, &path.0, "new proof input")
}

/// Deletes a file the proofs read.
#[when(expr = "the proof input {string} is deleted")]
fn delete_proof_input(world: &mut Behavior, path: String) -> Result<(), Failure> {
    std::fs::remove_file(root(world)?.join(path))?;
    Ok(())
}

/// Validates the consumer from two builds at once, then validates and checks it under each in turn.
#[when("two builds validate concurrently and each validates and checks the consumer")]
fn concurrent_builds(world: &mut Behavior) -> Result<(), Failure> {
    use tastemaker_core::domain::writing::types::Writing;
    use tastemaker_core::inbound::use_cases::validate::interface::ValidateUseCase as _;
    use tastemaker_core::inbound::use_cases::validate::types::ValidateRequest;
    let root = MachinePath(root(world)?.display().to_string().into());
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| -> Result<(), Failure> {
        let writers = ["build-a", "build-b"].map(|build| {
            let root = &root;
            let barrier = &barrier;
            scope.spawn(move || -> Result<(), Failure> {
                let (_, validate) = real_services(std::path::Path::new(&*root.0), build);
                let _released = barrier.wait();
                let report = validate.validate(ValidateRequest {
                    start: root.clone(),
                    only: Vec::new(),
                })?;
                assert!(
                    report.proved.iter().all(|proved| proved.proof
                        == tastemaker_core::domain::proofs::types::Proof::Proven),
                    "every proof holds under each build"
                );
                Ok(())
            })
        });
        for writer in writers {
            writer
                .join()
                .map_err(|payload| format!("validation thread panicked: {payload:?}"))??;
        }
        Ok(())
    })?;
    // The store is one build's: each build in its turn discards the other's and proves again before it checks.
    for build in ["build-a", "build-b"] {
        let (check, validate) = real_services(std::path::Path::new(&*root.0), build);
        let _proved = validate.validate(ValidateRequest {
            start: root.clone(),
            only: Vec::new(),
        })?;
        let report = check.check(CheckRequest {
            start: root.clone(),
            within: Vec::new(),
            fixing: Fixing::Fix,
            writing: Writing::Write,
        })?;
        assert!(
            report.report.findings.is_empty(),
            "the consumer is clean under each build"
        );
    }
    "both builds clean".clone_into(&mut world.answer);
    Ok(())
}

/// Compares the answer of `check` with the one the scenario expects.
#[then(expr = "check answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    // The step's text, kept as the answer's type: what a cucumber argument arrives as is an owned `String`.
    let expected: Box<str> = expected.into();
    assert_eq!(
        &*world.answer, &*expected,
        "check answers what the scenario expects"
    );
}

/// Checks the consumer through a store reopened and edited between runs.
#[when("the real consumer checks through a reopened and edited store")]
fn persistent_lifecycle(world: &mut Behavior) -> Result<(), Failure> {
    use tastemaker_core::domain::writing::types::Writing;
    use tastemaker_core::inbound::use_cases::check::types::LockOutcome;
    let project = root(world)?;
    let request = CheckRequest {
        start: MachinePath(project.display().to_string().into()),
        within: Vec::new(),
        fixing: Fixing::Fix,
        writing: Writing::Write,
    };
    let cold = real_services(project, "proof-behavior")
        .0
        .check(request.clone())?;
    assert!(
        cold.report.findings.is_empty(),
        "the consumer is clean on the first check"
    );
    assert!(
        project.join(".tastemaker_cache").is_dir(),
        "the check keeps its store on disk"
    );
    let mut expected = cold;
    expected.lock = LockOutcome::Current;
    let reopened = real_services(project, "proof-behavior")
        .0
        .check(request.clone())?;
    assert_eq!(
        reopened, expected,
        "a reopened store answers the same report from the kept answer"
    );
    write(project, "src/bad.txt", "bad\n")?;
    let edited = real_services(project, "proof-behavior")
        .0
        .check(request.clone())?;
    let [finding] = edited.report.findings.as_slice() else {
        return Err("the edit leaves exactly one finding".into());
    };
    assert_eq!(
        finding.path,
        RepoPath("src/bad.txt".into()),
        "the finding is in the file that was added"
    );
    assert_eq!(
        &*tastemaker_core::domain::findings::service::printed(&finding.rule).0,
        "a/marker",
        "the finding is the marker rule's"
    );
    let mut expected_edit = edited;
    expected_edit.lock = LockOutcome::Current;
    assert_eq!(
        real_services(project, "proof-behavior").0.check(request)?,
        expected_edit,
        "a reopened store answers the edited report from the kept answer"
    );
    world.answer = "persistent lifecycle holds".into();
    Ok(())
}
