// ignore: idiomatic.arch/file-length - the check fixture builds one coherent workspace and its injected port set; its package, repository and analysis builders describe the same fixture and splitting them would obscure which ports a case receives.
//! The test data of the `check` use case: a workspace of one package, held by the fakes of its ports.

use std::sync::Arc;

use super::types::{CheckRequest, Fixing};
use super::use_case::{CheckPorts, CheckUseCaseImpl};
use crate::cases::Selection;
use crate::domain::files::types::MachinePath;
use crate::outbound::analysis::fixtures::fake::AnalysisFake;
use crate::outbound::check_store::fixtures::fake::CheckStoreFake;
use crate::outbound::digest::fixtures::fake::DigestFake;
use crate::outbound::packages::fixtures::fake::PackagesFake;
use crate::outbound::repository::fixtures::fake::RepositoryFake;
use crate::outbound::validate_store::fixtures::fake::ValidateStoreFake;

/// The package's manifest.
pub const PACKAGE: &str = "name = \"p\"\n";

/// A rule that finds the comment `MARKER`.
pub const RULE: &str = r#"description = "A marker comment is reported."
why = "A test needs a rule that finds something."
query = '''
let found = select * from "comment" where text = " MARKER";
emit found subject "file" key("span");
'''

[diagnostic]
primary = "span"
message = "a marker"
help = "Remove the marker."

[[scenarios]]
id = "marker-in-a-comment"
scenario = "A marker comment is reported."
given.include = ["**"]
given.files."src/a/errors.txt" = "MARKER\n"
then.findings = [{ path = "src/a/errors.txt", line = 1 }]

[[scenarios]]
id = "marker-outside-the-included-files"
scenario = "A marker in a file the rule does not read is not reported, and a file it reads without one is not either."
given.include = ["docs/**"]
given.files."src/a/errors.txt" = "MARKER\n"
given.files."docs/notes.txt" = "notes\n"
"#;

/// A spec that places one `errors.txt` in each of its instances, and closes them.
pub const SPEC: &str = r##"description = "A part: one errors.txt."
why = "A test needs a spec that places a file."
templates = [
  { template = "errors", file = "errors.txt", description = "The part's errors." },
]

[[scenarios]]
id = "part-with-its-errors"
scenario = "A part with its errors.txt has no finding."
given.at = "src/a"
given.files."src/a/errors.txt" = "errors\n"

[[scenarios]]
id = "part-with-a-stray-file"
scenario = "A part with a file no template places is reported."
given.at = "src/a"
given.files."src/a/errors.txt" = "errors\n"
given.files."src/a/stray.txt" = "stray\n"
then.findings = [{ rule = "#closed", path = "src/a/stray.txt" }]
"##;

/// The template of that file.
pub const TEMPLATE: &str = "description = \"The errors.\"\nwhy = \"A test needs a template.\"\ncontent = \"errors\\n\"\n\n[[scenarios]]\nid = \"errors-file-present\"\nscenario = \"The file is there.\"\ngiven.file = \"errors.txt\"\ngiven.files.\"errors.txt\" = \"errors\\n\"\n\n[[scenarios]]\nid = \"errors-file-missing\"\nscenario = \"The file is missing.\"\ngiven.file = \"errors.txt\"\nthen.findings = [{ rule = \"errors#count\", path = \".\" }]\n";

/// The workspace: the rule over everything, the spec at each `src/<domain>`.
pub const WORKSPACE: &str = r#"workspace = { name = "w", dependencies = [{ package = "p", path = "p" }] }
closed = false
rules = [{ rule = "p/marker", include = ["**"] }]
specs = [{ spec = "p/part", include = ["src/{domain}"] }]
"#;

/// The package `p`, as the packages fake holds it.
#[must_use]
pub fn package() -> PackagesFake {
    PackagesFake::of(
        "p",
        &[
            ("package.toml", PACKAGE),
            ("rules/marker/0.1.0/rule.toml", RULE),
            ("specs/part/0.1.0/spec.toml", SPEC),
            ("specs/part/0.1.0/templates/errors/template.toml", TEMPLATE),
        ],
    )
}

/// The workspace's files: `src/a` placed and holding a stray file and a marker, `src/b` missing its `errors.txt`.
#[must_use]
pub fn repository() -> RepositoryFake {
    RepositoryFake::of(&[
        ("tastemaker.toml", WORKSPACE),
        ("src/a/errors.txt", "MARKER\nerrors\n"),
        ("src/a/stray.txt", "stray\n"),
        ("src/b/notes.txt", "b\n"),
    ])
}

/// What the analysis fake finds: the marker in `src/a/errors.txt`.
#[must_use]
pub fn analysis() -> AnalysisFake {
    let mut fake = AnalysisFake::default();
    // The successful proof is seeded by validation below. If check validates again, this
    // deliberately contrary query verdict makes the expected marker finding disappear.
    fake.invalid.push("MARKER".into());
    drop(fake.rows.insert(
        "MARKER".into(),
        vec![("src/a/errors.txt".into(), "0..6".into())],
    ));
    fake
}

/// What the analysis fake finds while the package is proven: the marker in `src/a/errors.txt`, its query valid.
fn proving_analysis() -> AnalysisFake {
    let mut fake = AnalysisFake::default();
    drop(fake.rows.insert(
        "MARKER".into(),
        vec![("src/a/errors.txt".into(), "0..6".into())],
    ));
    fake
}

/// The marker rule at two versions, each over its own file.
pub const TWO_VERSIONS_APART: &str = r#"rules = [
  { rule = "p/marker@0.1.0", include = ["one.txt"], description = "First version." },
  { rule = "p/marker@0.2.0", include = ["two.txt"], description = "Second version." },
]
"#;

/// The marker rule at two versions, both over one file.
pub const TWO_VERSIONS_TOGETHER: &str = r#"rules = [
  { rule = "p/marker@0.1.0", include = ["one.txt"], description = "First version." },
  { rule = "p/marker@0.2.0", include = ["one.txt"], description = "Second version." },
]
"#;

/// A lock pinning both versions of the marker rule, and the part.
const TWO_VERSIONS_LOCK: &str = r#"version = 1
dependencies = ["p"]

[packages.p]
path = "p"

[packages.p.rules]
marker = ["0.1.0", "0.2.0"]

[packages.p.specs]
part = "0.1.0"
"#;

/// A workspace that places the part and no rule of its own: its rules come from the part.
const TWO_VERSIONS_WORKSPACE: &str = r#"workspace = { name = "w", dependencies = [{ package = "p", path = "p" }] }
closed = false
specs = [{ spec = "p/part", include = ["src/{domain}"] }]
"#;

/// The package `p` with the marker rule at two versions, its part applying them as `rules` says.
#[must_use]
pub fn two_versions_package(rules: &str) -> PackagesFake {
    let spec = SPEC.replacen("templates = [", &format!("{rules}templates = ["), 1);
    PackagesFake::of(
        "p",
        &[
            ("package.toml", PACKAGE),
            ("rules/marker/0.1.0/rule.toml", RULE),
            ("rules/marker/0.2.0/rule.toml", RULE),
            ("specs/part/0.1.0/spec.toml", &spec),
            ("specs/part/0.1.0/templates/errors/template.toml", TEMPLATE),
        ],
    )
}

/// The repository the two-versions package is proven over: its workspace and lock alone.
#[must_use]
pub fn two_versions_validation() -> RepositoryFake {
    RepositoryFake::of(&[
        ("tastemaker.toml", TWO_VERSIONS_WORKSPACE),
        ("tastemaker.lock", TWO_VERSIONS_LOCK),
    ])
}

/// The repository the two-versions package is checked over: two parts, each with its errors and a file of one version.
#[must_use]
pub fn two_versions_repository() -> RepositoryFake {
    RepositoryFake::of(&[
        ("tastemaker.toml", TWO_VERSIONS_WORKSPACE),
        ("tastemaker.lock", TWO_VERSIONS_LOCK),
        ("src/a/errors.txt", "MARKER\n"),
        ("src/a/one.txt", "MARKER\n"),
        ("src/b/errors.txt", "MARKER\n"),
        ("src/b/two.txt", "MARKER\n"),
    ])
}

/// What the analysis fake finds over the two-versions repository: the marker in each of `paths`, its query refused if
/// check validates again.
#[must_use]
pub fn two_versions_analysis(paths: &[&str]) -> AnalysisFake {
    let mut fake = AnalysisFake::default();
    fake.invalid.push("MARKER".into());
    drop(
        fake.rows.insert(
            "MARKER".into(),
            paths
                .iter()
                .map(|path| ((*path).into(), "0..6".into()))
                .collect(),
        ),
    );
    fake
}

/// The project's store as the fixtures stand it in: validation's proofs and the check's answer, each its own fake.
#[derive(Debug, Default)]
pub struct StoreFake {
    /// Validation's proofs.
    pub proofs: Arc<ValidateStoreFake>,
    /// The check's answer.
    pub answers: Arc<CheckStoreFake>,
}

/// `check` over the fakes and the store `store`, which holds validation's proofs and the check's answer.
/// Builds the checking fixture after proving its package.
///
/// # Errors
/// When the fixture package cannot be validated.
pub fn use_case(
    repository: RepositoryFake,
    analysis: AnalysisFake,
    store: &StoreFake,
) -> Result<CheckUseCaseImpl, super::errors::CheckError> {
    use_case_of(package, self::repository, repository, analysis, store)
}

/// `use_case` over the package `package` makes, proven over the repository `validation` makes: each port takes its own.
///
/// # Errors
/// When the package cannot be validated.
pub fn use_case_of(
    package: fn() -> PackagesFake,
    validation: fn() -> RepositoryFake,
    repository: RepositoryFake,
    analysis: AnalysisFake,
    store: &StoreFake,
) -> Result<CheckUseCaseImpl, super::errors::CheckError> {
    let scheduler: Arc<dyn crate::outbound::scheduler::interface::SchedulerPort> =
        Arc::new(crate::outbound::scheduler::fixtures::fake::SchedulerFake);
    scheduled_use_case(
        (package, validation),
        repository,
        analysis,
        store,
        &scheduler,
    )
}

/// `use_case_of` over the scheduler the caller picks: `proven` makes the package, then the repository it is proven over.
fn scheduled_use_case(
    proven: (fn() -> PackagesFake, fn() -> RepositoryFake),
    repository: RepositoryFake,
    analysis: AnalysisFake,
    store: &StoreFake,
    scheduler: &Arc<dyn crate::outbound::scheduler::interface::SchedulerPort>,
) -> Result<CheckUseCaseImpl, super::errors::CheckError> {
    use crate::inbound::use_cases::validate::interface::ValidateUseCase as _;
    use crate::inbound::use_cases::validate::types::ValidateRequest;
    use crate::inbound::use_cases::validate::use_case::{ValidatePorts, ValidateUseCaseImpl};
    let validation_repository: Arc<dyn crate::outbound::repository::interface::RepositoryPort> =
        Arc::new((proven.1)());
    let repository: Arc<dyn crate::outbound::repository::interface::RepositoryPort> =
        Arc::new(repository);
    let matching: Arc<dyn crate::outbound::matching::interface::MatchingPort> =
        Arc::new(crate::outbound::matching::factory::globset());
    let digest: Arc<dyn crate::outbound::digest::interface::DigestPort> = Arc::new(DigestFake);
    let validation_reporting = Arc::new(
        crate::application::reporting::service::ReportingService::new(
            Arc::clone(scheduler),
            Arc::clone(&matching),
            Arc::clone(&digest),
        ),
    );
    let reporting = Arc::new(
        crate::application::reporting::service::ReportingService::new(
            Arc::clone(scheduler),
            Arc::clone(&matching),
            Arc::clone(&digest),
        ),
    );
    // These checking fixtures start with the package validated, as a real consumer does.
    let _validated = ValidateUseCaseImpl::new(ValidatePorts {
        scheduler: Arc::clone(scheduler),
        reporting: validation_reporting,
        fixtures: Arc::clone(&validation_repository),
        settings: crate::domain::settings::types::Store {
            directory: MachinePath("/fake/.tastemaker_cache".into()),
            build: crate::domain::settings::types::Build("fixtures".into()),
            writing: crate::domain::writing::types::Writing::Write,
        },
        repository: validation_repository,
        manifest: Arc::new(crate::outbound::manifest::factory::toml()),
        packages: Arc::new((proven.0)()),
        analysis: Arc::new(proving_analysis()),
        store: Arc::<ValidateStoreFake>::clone(&store.proofs),
        digest: Arc::new(DigestFake),
        matching: Arc::new(crate::outbound::matching::factory::globset()),
        templates: Arc::new(crate::outbound::templates::factory::minijinja()),
    })
    .validate(ValidateRequest {
        start: MachinePath("/fake".into()),
        only: Vec::new(),
    })
    .map_err(|error| super::errors::CheckError::Resolution {
        message: format!("the check fixture package failed validation: {error}"),
    })?;
    let analysis: Arc<dyn crate::outbound::analysis::interface::AnalysisPort> = Arc::new(analysis);
    Ok(CheckUseCaseImpl::new(CheckPorts {
        reporting,
        repository,
        manifest: Arc::new(crate::outbound::manifest::factory::toml()),
        packages: Arc::new((proven.0)()),
        counting: Arc::clone(&analysis),
        analysis,
        store: Arc::<CheckStoreFake>::clone(&store.answers),
        proofs: Arc::<ValidateStoreFake>::clone(&store.proofs),
        digest: Arc::new(DigestFake),
        matching: Arc::new(crate::outbound::matching::factory::globset()),
        templates: Arc::new(crate::outbound::templates::factory::minijinja()),
    }))
}

/// A `tastemaker.toml` no reader of the format reads: a table opened and never closed.
pub const BROKEN_WORKSPACE: &str = "version = [\n";

/// Which workspace file a row's repository holds.
#[derive(Clone, Copy, Debug)]
pub enum Workspace {
    /// The workspace of `WORKSPACE`, with its files.
    Placed,
    /// The same files, and a `tastemaker.toml` the manifest port cannot read.
    Broken,
}

/// The workspace's files, with the `tastemaker.toml` `workspace` says.
#[must_use]
pub fn repository_of(workspace: Workspace) -> RepositoryFake {
    let files = repository();
    if let Workspace::Broken = workspace {
        drop(
            files
                .files
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert(
                    crate::domain::paths::types::RepoPath("tastemaker.toml".into()),
                    crate::domain::files::types::FileText(BROKEN_WORKSPACE.into()),
                ),
        );
    }
    files
}

/// What a refusal selector answers: whether the error is the variant a row names.
#[must_use]
pub const fn selection(matched: bool) -> Selection {
    if matched {
        Selection::Matches
    } else {
        Selection::Differs
    }
}

/// The ids the fixture's check finds.
pub const FOUND: &str = "p/marker p/part#closed p/part#closed p/part/errors#count";

#[must_use]
/// A check from `/fake`, of every scope, with `fixing`.
pub fn request(fixing: Fixing) -> CheckRequest {
    CheckRequest {
        start: MachinePath("/fake".into()),
        within: Vec::new(),
        fixing,
        writing: crate::domain::writing::types::Writing::Write,
    }
}

/// The root `tastemaker.toml` that names `src/a` a member, `src/a` holding one that declares a `workspace` too, which only a
/// root does.
#[must_use]
pub fn member_workspace(files: &RepositoryFake) -> String {
    drop(
        files
            .files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                crate::domain::paths::types::RepoPath("src/a/tastemaker.toml".into()),
                crate::domain::files::types::FileText(WORKSPACE.into()),
            ),
    );
    WORKSPACE.replace("name = \"w\",", "name = \"w\", members = [\"src/a\"],")
}

/// Both scheduling strategies preserve complete reports and every persisted proof and finding.
///
/// # Errors
/// When validating or checking the prepared fixture fails.
///
/// # Panics
/// When parallel execution changes a report or a kept table.
pub fn serial_and_parallel_agree() -> Result<(), super::errors::CheckError> {
    use super::interface::CheckUseCase as _;
    let serial_proofs = Arc::new(StoreFake::default());
    let parallel_proofs = Arc::new(StoreFake::default());
    let (serial_files, serial_analysis) = parallel_input();
    let (parallel_files, parallel_analysis) = parallel_input();
    let serial = use_case(serial_files, serial_analysis, &serial_proofs)?;
    let pool: Arc<dyn crate::outbound::scheduler::interface::SchedulerPort> = Arc::new(
        crate::outbound::scheduler::factory::rayon(crate::outbound::scheduler::factory::pool(
            crate::outbound::scheduler::types::WorkerCount(2),
        )),
    );
    let parallel = scheduled_use_case(
        (package, repository),
        parallel_files,
        parallel_analysis,
        &parallel_proofs,
        &pool,
    )?;
    for _ in 0_u8..2 {
        let serial_report = serial.check(request(Fixing::Fix))?;
        let parallel_report = parallel.check(request(Fixing::Fix))?;
        assert!(
            !serial_report.report.findings.is_empty(),
            "the fixture must exercise ordered findings"
        );
        assert_eq!(
            serial_report, parallel_report,
            "cold and warm report ordering is deterministic"
        );
        let serial_checks = serial_proofs
            .answers
            .checks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let parallel_checks = parallel_proofs
            .answers
            .checks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert!(
            !serial_checks.files.is_empty(),
            "the fixture must persist its proofs and findings"
        );
        assert!(
            serial_proofs.proofs.held() > 0,
            "the fixture must retain successful proofs"
        );
        assert_eq!(
            serial_checks, parallel_checks,
            "all cached findings preserve order"
        );
        assert_eq!(
            *serial_proofs
                .proofs
                .tables
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            *parallel_proofs
                .proofs
                .tables
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            "all proof keys preserve order"
        );
    }
    mixed_proofs_agree(&serial, &parallel, &serial_proofs, &parallel_proofs);
    for proofs in [&serial_proofs, &parallel_proofs] {
        for rows in std::iter::once(
            &mut *proofs
                .proofs
                .tables
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ) {
            assert!(
                !rows.is_empty(),
                "the fixture must still hold the proofs of the versions not taken away"
            );
            rows.clear();
        }
    }
    let serial_error = serial.check(request(Fixing::Report));
    let parallel_error = parallel.check(request(Fixing::Report));
    assert!(
        matches!(&serial_error, Err(super::errors::CheckError::Proof { subject, .. })
        if subject == "p/rule/marker@0.1.0"),
        "the first failing proof slot is the one reported"
    );
    assert_eq!(
        serial_error, parallel_error,
        "the first failing proof slot is deterministic"
    );
    Ok(())
}

/// More than one source batch makes ordered reports exercise actual worker fanout.
fn parallel_input() -> (RepositoryFake, AnalysisFake) {
    let files = repository();
    let mut engine = analysis();
    let mut sources = files
        .files
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rows = engine.rows.entry("MARKER".into()).or_default();
    for index in 0_u16..130 {
        let path = format!("src/generated/errors_{index}.rs");
        drop(sources.insert(
            crate::domain::paths::types::RepoPath(path.clone().into()),
            crate::domain::files::types::FileText("extra\r\n".into()),
        ));
        rows.push((path.into(), "0..6".into()));
    }
    drop(sources);
    (files, engine)
}

/// A proof no longer held remains paired with its own version, whatever order workers finish in.
fn mixed_proofs_agree(
    serial: &CheckUseCaseImpl,
    parallel: &CheckUseCaseImpl,
    serial_proofs: &StoreFake,
    parallel_proofs: &StoreFake,
) {
    use super::interface::CheckUseCase as _;
    // Every row the validation left: each subject in its turn is the one no longer held.
    let every = serial_proofs
        .proofs
        .tables
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let subjects = every
        .iter()
        .map(|row| {
            crate::domain::scenario_proving::service::subject_of(&row.release)
                .id
                .0
                .to_string()
        })
        .filter(|subject| {
            matches!(
                subject.as_str(),
                "p/rule/marker@0.1.0" | "p/spec/part@0.1.0"
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        subjects.len(),
        2,
        "both placed versions must have proof keys"
    );
    for subject in subjects {
        for proofs in [serial_proofs, parallel_proofs] {
            for rows in std::iter::once(
                &mut *proofs
                    .proofs
                    .tables
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
            ) {
                *rows = every
                    .iter()
                    .filter(|row| {
                        *crate::domain::scenario_proving::service::subject_of(&row.release)
                            .id
                            .0
                            != *subject
                    })
                    .cloned()
                    .collect();
            }
        }
        let serial_error = serial.check(request(Fixing::Report));
        let parallel_error = parallel.check(request(Fixing::Report));
        assert_eq!(
            serial_error,
            Err(super::errors::CheckError::Proof {
                subject: subject.clone(),
                reason: crate::domain::proofs::constants::UNPROVEN.into(),
            }),
            "a proof no longer held must identify its original version"
        );
        assert_eq!(
            serial_error, parallel_error,
            "mixed proof outcomes settle identically"
        );
    }
}
