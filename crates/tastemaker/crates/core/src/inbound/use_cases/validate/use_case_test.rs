// ignore: idiomatic.arch/file-length - the validation table and its nested-fixture reader harness prove one workflow, including exact diagnostics and finish-before-publication ordering; separating the event recorder from its cases would hide the lifecycle assertions.
//! What the `validate` use case promises, against the fakes of the ports it calls.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::domain::paths::types::RepoPath;

use super::errors::ValidateError;
use super::interface::ValidateUseCase;
use super::types::{Proved, Proving, ValidateRequest};
use super::use_case::{ValidatePorts, ValidateUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::{FileText, MachinePath};
use crate::domain::proofs::types::Proof;
use crate::inbound::use_cases::check::fixtures::selection;
use crate::outbound::analysis::fixtures::fake::AnalysisFake;
use crate::outbound::packages::fixtures::fake::PackagesFake;
use crate::outbound::repository::fixtures::fake::RepositoryFake;
use crate::outbound::validate_store::fixtures::fake::ValidateStoreFake as StoreFake;
use crate::outbound::validate_store::interface::ValidateStorePort;

/// A rule with a scenario that expects the marker at line 1, and one that expects nothing.
const RULE: &str = r#"description = "A marker comment is reported."
why = "A test needs a rule with a scenario."
query = '''
let found = select * from "comment" where text = " MARKER";
emit found subject "file" key("span");
'''

[diagnostic]
primary = "span"
message = "a marker"
help = "Remove the marker."

[[scenarios]]
id = "marker-in-a-line-comment"
scenario = "A marker comment is reported."
given.include = ["**"]
given.files."src/notes.txt" = "MARKER\n"
then.findings = [{ path = "src/notes.txt", line = 1 }]

[[scenarios]]
id = "marker-outside-the-included-files"
scenario = "A marker in a file the rule does not read is not reported, and a file it reads without one is not either."
given.include = ["docs/**"]
given.files."src/notes.txt" = "MARKER\n"
given.files."docs/notes.txt" = "notes\n"
"#;

/// The workspace's lock, pinning the marker rule's first version.
const PINS: &str = "version = 1\ndependencies = [\"p\"]\n\n[packages.p]\npath = \"p\"\n\n[packages.p.rules]\nmarker = \"0.1.0\"\n";

/// The workspace's lock, pinning no rule of its package.
const UNPINNED: &str = "version = 1\ndependencies = [\"p\"]\n\n[packages.p]\npath = \"p\"\n";

/// The workspace, depending on `package`.
fn workspace(package: &str) -> RepositoryFake {
    RepositoryFake::of(&[(
        "tastemaker.toml",
        &format!(
            "workspace = {{ name = \"w\", dependencies = [{{ package = \"{package}\", path = \"{package}\" }}] }}\n"
        ),
    )])
}

/// What a row changes.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// An explicit filter matches no available definition.
    NoMatch,
    /// The rule's case passes.
    Proven,
    /// Validated again with nothing changed: the proof is a lookup, no definition file is read, and the store is not
    /// written.
    Again,
    /// The fake refuses the rule's query.
    Refused,
    /// The workspace names a package no source holds.
    NoPackage,
    /// Validated again after only a template's content changed: the proof is made again.
    ContentChanged,
    /// Validated after the content changed many times: the store holds one proof per version, not one per change.
    ChangedOften,
    /// The workspace's `tastemaker.toml` is text the manifest port cannot read.
    Broken,
    /// The nearest `tastemaker.toml` is a local one: it declares no `workspace`.
    Local,
    /// Validated again after only the content of a template a spec owns changed: the spec is proven again.
    OwnedChanged,
    /// Validated again after a package that only a scenario's own `tastemaker.toml` names changed: the proof is made again.
    ForeignPackageChanged,
    /// Validated again after only a past version a spec's composition pins changed, a version nothing else proves: the
    /// spec is proven again, the pinned version's files folded into its digest.
    PinnedUseChanged,
    /// A package holds a file outside its manifest, its releases, its nested packages and its versions' directories.
    Stray,
    /// The package holds a second version of the marker rule and the workspace's lock pins neither.
    Unread,
    /// Every version validated, then one rule's scenario edited: that rule's version alone is proved again.
    OneEdited,
    /// A run that fails after a validation keeps the proofs that validation held: the next run holds them all.
    FailedRunKeeps,
    /// A run in which a version's proof fails writes nothing: the store holds what the last run that succeeded proved.
    FailedProofWritesNothing,
    /// A definition with a past version: only its latest is proved, and nothing pins the past one.
    PastVersion,
    /// The same, with the workspace's lock pinning the past version: both are proved.
    PinnedPastVersion,
    /// The versions proved one after the other and on a pool of workers: the same report, the same tree.
    SerialParallel,
    /// After a validation of many scenarios, no scratch directory is left, and none was made for the lint calls: each is
    /// handed its scenario's files.
    Bounded,
    /// A run lists the versions a package offers again, the next run of the same use case: what one run listed is not kept for it.
    ListsAgainNextRun,
    /// A run leaves none of its caches (`Parses`, `Manifests`, `Offers`, `Verdicts`, `Wrappers`) holding more than its bound, and none holding
    /// anything for the next run.
    CachesStayWithinTheirBounds,
}

/// What a row changes.
type Input = Scenario;

/// Each proof, in a word, and whether it was kept.
type Value = String;
type Failure = ValidateError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_explicit_filter_matching_no_version_is_refused"),
        word: Word::Refuses,
        input: || Scenario::NoMatch,
        expect: Expect::error(ValidateError::NoMatch),
    },
    Case {
        name: CaseName("a_rule_whose_case_passes_is_proven"),
        word: Word::Succeeds,
        input: || Scenario::Proven,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_unchanged_version_is_a_lookup"),
        word: Word::Succeeds,
        input: || Scenario::Again,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_template_whose_content_changed_is_proven_again"),
        word: Word::Succeeds,
        input: || Scenario::ContentChanged,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("many_runs_in_a_row_leave_a_bounded_store_one_proof_per_version"),
        word: Word::Succeeds,
        input: || Scenario::ChangedOften,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_spec_whose_owned_template_content_changed_is_proven_again"),
        word: Word::Succeeds,
        input: || Scenario::OwnedChanged,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "a_package_only_a_scenarios_own_tastemaker_toml_names_proves_again_when_it_changes",
        ),
        word: Word::Succeeds,
        input: || Scenario::ForeignPackageChanged,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_spec_proves_again_when_a_past_version_its_composition_pins_changes"),
        word: Word::Succeeds,
        input: || Scenario::PinnedUseChanged,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("one_edited_scenario_proves_its_version_again_and_keeps_every_other_proof"),
        word: Word::Succeeds,
        input: || Scenario::OneEdited,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_failed_run_keeps_the_proofs_the_last_validation_held"),
        word: Word::Succeeds,
        input: || Scenario::FailedRunKeeps,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_run_in_which_a_proof_fails_writes_nothing"),
        word: Word::Succeeds,
        input: || Scenario::FailedProofWritesNothing,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_past_version_nothing_pins_is_not_proved"),
        word: Word::Succeeds,
        input: || Scenario::PastVersion,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_past_version_the_lock_pins_is_proved_beside_the_latest"),
        word: Word::Succeeds,
        input: || Scenario::PinnedPastVersion,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("serial_and_parallel_validations_answer_and_keep_the_same"),
        word: Word::Succeeds,
        input: || Scenario::SerialParallel,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("no_scenario_sets_up_a_directory_of_its_own_and_none_is_left"),
        word: Word::Succeeds,
        input: || Scenario::Bounded,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "the_versions_a_package_offers_are_listed_again_by_the_next_run_of_the_same_use_case",
        ),
        word: Word::Succeeds,
        input: || Scenario::ListsAgainNextRun,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "a_run_leaves_no_cache_over_its_bound_and_none_holding_anything_for_the_next",
        ),
        word: Word::Succeeds,
        input: || Scenario::CachesStayWithinTheirBounds,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rule_whose_query_ryo_refuses_does_not_hold"),
        word: Word::Succeeds,
        input: || Scenario::Refused,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_package_no_source_holds_fails_the_validation"),
        word: Word::Fails,
        input: || Scenario::NoPackage,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ValidateError::Packages(
                    crate::outbound::packages::errors::PackagesError::Missing {
                        package: "q".into(),
                        path: "/q".into(),
                    },
                ))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_workspace_file_the_manifest_cannot_read_fails_the_validation"),
        word: Word::Fails,
        input: || Scenario::Broken,
        expect: Expect::Refusal(|error| selection(matches!(error, ValidateError::Manifest { .. }))),
    },
    Case {
        name: CaseName("a_local_tastemaker_toml_is_no_root_to_validate_from"),
        word: Word::Refuses,
        input: || Scenario::Local,
        expect: Expect::Refusal(|error| selection(matches!(error, ValidateError::NotARoot { .. }))),
    },
    Case {
        name: CaseName("a_version_no_lock_reads_and_not_the_latest_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Unread,
        expect: Expect::Refusal(|error| {
            selection(
                matches!(error, ValidateError::Unread { versions } if versions == &["p/rule/marker@0.1.0"]),
            )
        }),
    },
    Case {
        name: CaseName("a_file_a_package_holds_outside_its_versions_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Stray,
        expect: Expect::Refusal(|error| {
            selection(matches!(error, ValidateError::Stray { file, .. } if file == "stray.txt"))
        }),
    },
];

/// A template whose content is `body`, with a scenario that finds its file missing and one that finds it there.
fn template(body: &str, missing: &str) -> String {
    format!(
        "description = \"The errors.\"\nwhy = \"A test needs a template.\"\ncontent = \"\"\"\n{body}\"\"\"\n\n[[scenarios]]\nid = \"file-missing\"\nscenario = \"The file is missing.\"\ngiven.file = \"example.toml\"\nthen.findings = [{missing}]\n\n[[scenarios]]\nid = \"file-present\"\nscenario = \"The file is there.\"\ngiven.file = \"example.toml\"\ngiven.files.\"example.toml\" = \"value=1\\n\"\n"
    )
}

/// What a shared template's file missing is found as.
const SHARED_MISSING: &str = "{ rule = \"body#count\", path = \"tastemaker.toml\" }";

/// What an owned template's file missing is found as.
const OWNED_MISSING: &str = "{ rule = \"errors#count\", path = \".\" }";

/// A spec that owns one template, `errors`, and the scenarios that find the instance whole and its file missing.
const SPEC: &str = "description = \"A part.\"\nwhy = \"A test needs a spec that owns a template.\"\ntemplates = [\n  { template = \"errors\", file = \"errors.txt\", description = \"The part's errors.\" },\n]\n\n[[scenarios]]\nid = \"errors-file-present\"\nscenario = \"The part has its errors file.\"\ngiven.at = \"src/part\"\ngiven.files.\"src/part/errors.txt\" = \"errors\\n\"\n\n[[scenarios]]\nid = \"errors-file-missing\"\nscenario = \"A part without its errors file is reported.\"\ngiven.at = \"src/part\"\ngiven.files.\"src/part/other.txt\" = \"\\n\"\nthen.findings = [{ rule = \"p/part/errors#count\", path = \"src/part\" }]\n";

/// The package `p`: the marker rule, a template of the content `body`, and a spec that owns another.
fn holding(body: &str) -> PackagesFake {
    PackagesFake::of(
        "p",
        &[
            ("package.toml", "name = \"p\"\n"),
            ("rules/marker/0.1.0/rule.toml", RULE),
            // A file beside the rule: a proof's key stamps it, and nothing reads it.
            ("rules/marker/0.1.0/notes.txt", "a note\n"),
            (
                "templates/body/0.1.0/template.toml",
                &template(body, SHARED_MISSING),
            ),
            ("specs/part/0.1.0/spec.toml", SPEC),
            (
                "specs/part/0.1.0/templates/errors/template.toml",
                &template(body, OWNED_MISSING),
            ),
        ],
    )
}

/// The package `p`, whose marker rule has a scenario that writes its own `tastemaker.toml`, and the package `r` that file
/// alone depends on and names a rule of: the workspace depends on `p` only. `note` is `r`'s description of its rule.
fn with_scenario_package(note: &str) -> PackagesFake {
    let scenario = "\n[[scenarios]]\nid = \"another-package\"\nscenario = \"A scenario brings its own workspace.\"\ngiven.files.\"tastemaker.toml\" = '''\nworkspace = { name = \"s\", dependencies = [{ package = \"r\", path = \"../r\" }] }\nrules = [{ rule = \"r/other\", include = [\"**\"], description = \"The other rule.\" }]\n'''\ngiven.files.\"src/notes.txt\" = \"MARKER\\n\"\nthen.findings = [{ path = \"src/notes.txt\", line = 1 }]\n";
    let mut both = holding("errors\n");
    drop(both.files.insert(
        ("/p".into(), "rules/marker/0.1.0/rule.toml".into()),
        format!("{RULE}{scenario}").into(),
    ));
    both.files.extend([
        (
            ("/r".into(), "package.toml".into()),
            "name = \"r\"\n".into(),
        ),
        (
            ("/r".into(), "rules/other/0.1.0/rule.toml".into()),
            format!("{RULE}\n# {note}\n").into(),
        ),
    ]);
    both
}

/// The package `p` with a past version of its marker rule beside the latest, and a spec whose composition pins the past
/// one: nothing proves that version itself. `note` is a comment the past version's file ends with.
fn with_pinned_use(note: &str) -> PackagesFake {
    let spec = "description = \"A spec of a past rule.\"\nwhy = \"A test needs a spec that pins a past version.\"\nrules = [\n  { rule = \"p/marker@0.1.0\", include = [\"**\"], description = \"The marker, as it was.\" },\n]\n\n[[scenarios]]\nid = \"marker-found\"\nscenario = \"A marker comment is reported.\"\ngiven.files.\"src/notes.txt\" = \"MARKER\\n\"\nthen.findings = [{ rule = \"p/marker\", path = \"src/notes.txt\", line = 1 }]\n\n[[scenarios]]\nid = \"no-marker\"\nscenario = \"A file without a marker is not reported.\"\ngiven.files.\"src/plain.txt\" = \"plain\\n\"\n";
    let mut pins = holding("errors\n");
    pins.files.extend([
        (
            ("/p".into(), "rules/marker/0.1.0/rule.toml".into()),
            format!("{RULE}\n# {note}\n").into(),
        ),
        (
            ("/p".into(), "rules/marker/0.2.0/rule.toml".into()),
            RULE.into(),
        ),
        (
            ("/p".into(), "specs/pins/0.1.0/spec.toml".into()),
            spec.into(),
        ),
    ]);
    pins
}

/// How many proofs the store holds, over every build.
fn kept_proofs(store: &StoreFake) -> usize {
    store.held()
}

/// Validates every version, then validates again with nothing changed, and answers what the second run answered once it
/// holds that the run read no definition file, its stamps unmoved, and did not write the store.
fn unchanged(
    validate: &impl ValidateUseCase,
    store: &StoreFake,
    read: &std::sync::Mutex<Vec<Box<str>>>,
    request: ValidateRequest,
) -> Result<super::types::ValidateReport, Failure> {
    let saves = || {
        *store
            .saves
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    };
    let _every = validate.validate(ValidateRequest {
        only: Vec::new(),
        ..request.clone()
    })?;
    let before = saves();
    read.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
    let report = validate.validate(request)?;
    let paths = read
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    // The stamps say nothing moved: no file of a version is read.
    let versions = paths
        .iter()
        .filter(|path| {
            ["/rules/", "/specs/", "/templates/"]
                .iter()
                .any(|kind| path.contains(kind))
        })
        .collect::<Vec<_>>();
    let written = saves() - before;
    assert!(
        versions.is_empty() && written == 0,
        "an unchanged validation read {versions:?}; saved {written} times"
    );
    Ok(report)
}

/// The version filter a row's validation is limited to.
fn only_for(scenario: Scenario) -> &'static str {
    match scenario {
        Scenario::NoMatch => "no-such-definition",
        Scenario::ContentChanged | Scenario::ChangedOften => "template/body",
        Scenario::OwnedChanged => "spec/part",
        Scenario::PinnedUseChanged => "spec/pins",
        Scenario::Proven
        | Scenario::Again
        | Scenario::Refused
        | Scenario::NoPackage
        | Scenario::Broken
        | Scenario::Local
        | Scenario::ForeignPackageChanged
        | Scenario::Stray
        | Scenario::Unread
        | Scenario::OneEdited
        | Scenario::FailedRunKeeps
        | Scenario::FailedProofWritesNothing
        | Scenario::PastVersion
        | Scenario::PinnedPastVersion
        | Scenario::SerialParallel
        | Scenario::Bounded
        | Scenario::ListsAgainNextRun
        | Scenario::CachesStayWithinTheirBounds => "rule/marker",
    }
}

/// The repository a row's workspace is in: the workspace of `package`, or a root the row breaks.
fn repository_for(scenario: Scenario, package: &str) -> RepositoryFake {
    match scenario {
        Scenario::Broken => RepositoryFake::of(&[("tastemaker.toml", "version = [\n")]),
        Scenario::Local => RepositoryFake::of(&[("tastemaker.toml", "closed = false\n")]),
        Scenario::NoMatch
        | Scenario::Proven
        | Scenario::Again
        | Scenario::Refused
        | Scenario::NoPackage
        | Scenario::ContentChanged
        | Scenario::ChangedOften
        | Scenario::OwnedChanged
        | Scenario::ForeignPackageChanged
        | Scenario::PinnedUseChanged
        | Scenario::Stray
        | Scenario::OneEdited
        | Scenario::FailedRunKeeps
        | Scenario::FailedProofWritesNothing
        | Scenario::PastVersion
        | Scenario::PinnedPastVersion
        | Scenario::SerialParallel
        | Scenario::Bounded
        | Scenario::ListsAgainNextRun
        | Scenario::CachesStayWithinTheirBounds => workspace(package),
        Scenario::Unread => {
            let unpinned = workspace(package);
            drop(
                unpinned
                    .files
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(
                        RepoPath("tastemaker.lock".into()),
                        FileText(UNPINNED.into()),
                    ),
            );
            unpinned
        }
    }
}

/// The package the scenario validates: one rule, with a stray file or a second version where the scenario says.
fn held_for(scenario: Scenario) -> PackagesFake {
    let mut held = holding("errors\n");
    if matches!(scenario, Scenario::Stray) {
        drop(
            held.files
                .insert(("/p".into(), "stray.txt".into()), "a note\n".into()),
        );
    }
    if matches!(scenario, Scenario::Unread) {
        drop(held.files.insert(
            ("/p".into(), "rules/marker/0.2.0/rule.toml".into()),
            RULE.into(),
        ));
    }
    held
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    if let Some(answer) = lifecycle(scenario) {
        return answer;
    }
    let package = if matches!(scenario, Scenario::NoPackage) {
        "q"
    } else {
        "p"
    };
    let store = Arc::new(StoreFake::default());
    // One store for every run, kept by the case to count what it holds.
    let port: Arc<dyn ValidateStorePort> = Arc::<StoreFake>::clone(&store);
    let validator = |packages: PackagesFake| {
        let repository = Arc::new(repository_for(scenario, package));
        let scheduler = Arc::new(crate::outbound::scheduler::fixtures::fake::SchedulerFake);
        let digest = Arc::new(crate::outbound::digest::fixtures::fake::DigestFake);
        let matching: Arc<dyn crate::outbound::matching::interface::MatchingPort> =
            Arc::new(crate::outbound::matching::factory::globset());
        let reporting = Arc::new(
            crate::application::reporting::service::ReportingService::new(
                Arc::<crate::outbound::scheduler::fixtures::fake::SchedulerFake>::clone(&scheduler),
                Arc::clone(&matching),
                Arc::<crate::outbound::digest::fixtures::fake::DigestFake>::clone(&digest),
            ),
        );
        ValidateUseCaseImpl::new(ValidatePorts {
            scheduler,
            reporting,
            fixtures: Arc::<RepositoryFake>::clone(&repository),
            settings: crate::domain::settings::types::Store {
                directory: MachinePath("/fake/.tastemaker_cache".into()),
                build: crate::domain::settings::types::Build("test".into()),
                writing: crate::domain::writing::types::Writing::Write,
            },
            repository,
            manifest: Arc::new(crate::outbound::manifest::factory::toml()),
            packages: Arc::new(packages),
            analysis: Arc::new(analysis_for(scenario)),
            store: Arc::clone(&port),
            digest: Arc::new(crate::outbound::digest::fixtures::fake::DigestFake),
            matching: Arc::new(crate::outbound::matching::factory::globset()),
            templates: Arc::new(crate::outbound::templates::fixtures::fake::TemplatesFake),
        })
    };
    let only = only_for(scenario);
    let request = || ValidateRequest {
        start: MachinePath("/fake".into()),
        only: vec![super::types::VersionFilter(only.into())],
    };
    let held = held_for(scenario);
    let read = Arc::clone(&held.read);
    let validate = validator(held);
    let mut report = if matches!(scenario, Scenario::ForeignPackageChanged) {
        // The first run only fills the store the second run reads.
        let _primed = validator(with_scenario_package("")).validate(request())?;
        validator(with_scenario_package("changed")).validate(request())?
    } else if matches!(scenario, Scenario::PinnedUseChanged) {
        // The first run fills the store; the second, nothing changed, keeps the spec's proof; the third changes only the
        // past version the spec pins.
        let _primed = validator(with_pinned_use("")).validate(request())?;
        let again = validator(with_pinned_use("")).validate(request())?;
        if again
            .proved
            .iter()
            .any(|proved| proved.proving != Proving::Kept)
        {
            return Ok(format!(
                "an unchanged spec was proved again: {:?}",
                again.proved
            ));
        }
        validator(with_pinned_use("changed")).validate(request())?
    } else {
        validate.validate(request())?
    };
    if matches!(scenario, Scenario::Again) {
        report = unchanged(&validate, &store, &read, request())?;
    }
    if matches!(scenario, Scenario::ContentChanged | Scenario::OwnedChanged) {
        report = validator(holding("errors changed\n")).validate(request())?;
    }
    if matches!(scenario, Scenario::ChangedOften) {
        let first = kept_proofs(&store);
        for change in 0..5 {
            // The fake stamps a file by its length, so each body is longer than the last.
            let body = format!("errors changed{}\n", "s".repeat(change));
            report = validator(holding(&body)).validate(request())?;
        }
        // Six runs made six bodies; the store holds as many proofs as after the first.
        let kept = kept_proofs(&store);
        if kept != first {
            return Ok(format!("{kept} proofs kept, {first} after the first run"));
        }
    }

    let [proved] = report.proved.as_slice() else {
        return Ok(format!("{} versions proved", report.proved.len()));
    };
    Ok(judged(scenario, proved))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

/// What the row says of the one version proved: nothing when it is proved and made or kept as the row expects.
fn judged(scenario: Scenario, proved: &Proved) -> Value {
    // A template's or a spec's proof is made again or kept; whether the fakes let its scenarios hold is not this row's
    // question.
    if matches!(
        scenario,
        Scenario::ContentChanged
            | Scenario::ChangedOften
            | Scenario::OwnedChanged
            | Scenario::ForeignPackageChanged
            | Scenario::PinnedUseChanged
    ) {
        return if proved.proving == Proving::Made {
            String::new()
        } else {
            format!("{:?}", proved.proving)
        };
    }
    let expected = match scenario {
        Scenario::NoMatch
        | Scenario::Proven
        | Scenario::NoPackage
        | Scenario::ContentChanged
        | Scenario::ChangedOften
        | Scenario::Broken
        | Scenario::Local
        | Scenario::OwnedChanged
        | Scenario::ForeignPackageChanged
        | Scenario::PinnedUseChanged
        | Scenario::Stray
        | Scenario::Unread
        | Scenario::OneEdited
        | Scenario::FailedRunKeeps
        | Scenario::FailedProofWritesNothing
        | Scenario::PastVersion
        | Scenario::PinnedPastVersion
        | Scenario::SerialParallel
        | Scenario::Bounded
        | Scenario::ListsAgainNextRun
        | Scenario::CachesStayWithinTheirBounds => (true, Proving::Made),
        Scenario::Again => (true, Proving::Kept),
        Scenario::Refused => (false, Proving::Made),
    };
    let answered = (proved.proof == Proof::Proven, proved.proving);
    if answered == expected {
        String::new()
    } else {
        format!("{answered:?} {:?}", proved.proof)
    }
}

fn analysis_for(scenario: Scenario) -> AnalysisFake {
    let mut analysis = AnalysisFake::default();
    drop(analysis.rows.insert(
        "MARKER".into(),
        vec![("src/notes.txt".into(), "0..6".into())],
    ));
    if matches!(scenario, Scenario::Refused) {
        analysis.invalid.push("MARKER".into());
    }
    analysis
}

/// A validation of `packages` from the workspace `repository`, over the store `store` and the scheduler `scheduler`:
/// every scenario's project written to `repository`'s scratch directories, as the edge writes them to memory.
fn lifecycle_validator(
    repository: &Arc<RepositoryFake>,
    packages: PackagesFake,
    store: &Arc<StoreFake>,
    scheduler: Arc<dyn crate::outbound::scheduler::interface::SchedulerPort>,
) -> ValidateUseCaseImpl {
    lifecycle_validator_over(
        repository,
        packages,
        store,
        scheduler,
        analysis_for(Scenario::Proven),
    )
}

/// `lifecycle_validator` over the analysis `analysis`.
fn lifecycle_validator_over(
    repository: &Arc<RepositoryFake>,
    packages: PackagesFake,
    store: &Arc<StoreFake>,
    scheduler: Arc<dyn crate::outbound::scheduler::interface::SchedulerPort>,
    analysis: AnalysisFake,
) -> ValidateUseCaseImpl {
    let digest = Arc::new(crate::outbound::digest::fixtures::fake::DigestFake);
    let matching: Arc<dyn crate::outbound::matching::interface::MatchingPort> =
        Arc::new(crate::outbound::matching::factory::globset());
    let reporting = Arc::new(
        crate::application::reporting::service::ReportingService::new(
            Arc::clone(&scheduler),
            Arc::clone(&matching),
            Arc::<crate::outbound::digest::fixtures::fake::DigestFake>::clone(&digest),
        ),
    );
    ValidateUseCaseImpl::new(ValidatePorts {
        scheduler,
        reporting,
        fixtures: Arc::<RepositoryFake>::clone(repository),
        settings: crate::domain::settings::types::Store {
            directory: MachinePath("/fake/.tastemaker_cache".into()),
            build: crate::domain::settings::types::Build("test".into()),
            writing: crate::domain::writing::types::Writing::Write,
        },
        repository: Arc::<RepositoryFake>::clone(repository),
        manifest: Arc::new(crate::outbound::manifest::factory::toml()),
        packages: Arc::new(packages),
        analysis: Arc::new(analysis),
        store: Arc::<StoreFake>::clone(store),
        digest,
        matching,
        templates: Arc::new(crate::outbound::templates::fixtures::fake::TemplatesFake),
    })
}

/// Every version, validated, from the workspace of `p`.
fn everything() -> ValidateRequest {
    ValidateRequest {
        start: MachinePath("/fake".into()),
        only: Vec::new(),
    }
}

/// The marker rule and the template, which hold: the spec beside them does not, and a run in which a version fails writes nothing.
fn holding_versions() -> ValidateRequest {
    ValidateRequest {
        start: MachinePath("/fake".into()),
        only: vec![
            super::types::VersionFilter("rule/marker".into()),
            super::types::VersionFilter("template/body".into()),
        ],
    }
}

/// Each version a report names, whether it holds, and whether its proof was kept or made.
fn outcomes(report: &super::types::ValidateReport) -> Vec<(String, bool, Proving)> {
    report
        .proved
        .iter()
        .map(|proved| {
            (
                proved.subject.id.0.to_string(),
                proved.proof == Proof::Proven,
                proved.proving,
            )
        })
        .collect()
}

/// The package `p` with a second, past version of the marker rule beside its latest.
fn with_past_marker() -> PackagesFake {
    let mut held = holding("errors\n");
    let latest = held
        .files
        .remove(&("/p".into(), "rules/marker/0.1.0/rule.toml".into()))
        .unwrap_or_default();
    drop(held.files.insert(
        ("/p".into(), "rules/marker/0.1.0/rule.toml".into()),
        latest.clone(),
    ));
    drop(
        held.files
            .insert(("/p".into(), "rules/marker/0.2.0/rule.toml".into()), latest),
    );
    held
}

/// The lifecycle rows: what a row answers, or nothing for a row of the others.
fn lifecycle(scenario: Scenario) -> Option<Result<Value, Failure>> {
    let store = Arc::new(StoreFake::default());
    let repository = Arc::new(workspace("p"));
    Some(match scenario {
        Scenario::OneEdited => one_edited(&repository, &store),
        Scenario::FailedRunKeeps => failed_run_keeps(&repository, &store),
        Scenario::FailedProofWritesNothing => failed_proof_writes_nothing(&repository, &store),
        Scenario::PastVersion | Scenario::PinnedPastVersion => {
            past_version(scenario, &repository, &store)
        }
        Scenario::SerialParallel => serial_parallel(&repository, &store),
        Scenario::Bounded => bounded(&repository, &store),
        Scenario::ListsAgainNextRun => lists_again(&repository, &store),
        Scenario::CachesStayWithinTheirBounds => {
            caches_stay_within_their_bounds(&repository, &store)
        }
        Scenario::NoMatch
        | Scenario::Proven
        | Scenario::Again
        | Scenario::Refused
        | Scenario::NoPackage
        | Scenario::ContentChanged
        | Scenario::ChangedOften
        | Scenario::Broken
        | Scenario::Local
        | Scenario::OwnedChanged
        | Scenario::ForeignPackageChanged
        | Scenario::PinnedUseChanged
        | Scenario::Stray
        | Scenario::Unread => return None,
    })
}

/// The serial scheduler.
fn serial() -> Arc<dyn crate::outbound::scheduler::interface::SchedulerPort> {
    Arc::new(crate::outbound::scheduler::fixtures::fake::SchedulerFake)
}

/// Edited after a validation of every version: only the edited version is proved again, and no unchanged file is read.
fn one_edited(repository: &Arc<RepositoryFake>, store: &Arc<StoreFake>) -> Result<Value, Failure> {
    let first = lifecycle_validator(repository, holding("errors\n"), store, serial())
        .validate(holding_versions())?;
    let mut edited = holding("errors\n");
    let marker = ("/p".into(), "rules/marker/0.1.0/rule.toml".into());
    let text = edited.files.get(&marker).cloned().unwrap_or_default();
    drop(edited.files.insert(
        marker,
        format!("{text}\n# A scenario's comment edited.\n").into(),
    ));
    let read = Arc::clone(&edited.read);
    let second =
        lifecycle_validator(repository, edited, store, serial()).validate(holding_versions())?;
    let expected = outcomes(&first)
        .into_iter()
        .map(|(id, holds, _)| {
            let proving = if id == "p/rule/marker@0.1.0" || !holds {
                Proving::Made
            } else {
                Proving::Kept
            };
            (id, holds, proving)
        })
        .collect::<Vec<_>>();
    // A version whose proof is held and whose stamp did not move reads none of its files: the template's.
    let others = read
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .filter(|path| path.contains("templates/body/"))
        .count();
    let answered = outcomes(&second);
    Ok(if answered == expected && others == 0 {
        String::new()
    } else {
        format!("{answered:?}, unchanged definition files read {others} times")
    })
}

/// A run that fails after a validation keeps that validation's proofs.
fn failed_run_keeps(
    repository: &Arc<RepositoryFake>,
    store: &Arc<StoreFake>,
) -> Result<Value, Failure> {
    let first = lifecycle_validator(repository, holding("errors\n"), store, serial())
        .validate(holding_versions())?;
    let held = kept_proofs(store);
    let mut strayed = holding("errors\n");
    drop(
        strayed
            .files
            .insert(("/p".into(), "stray.txt".into()), "a note\n".into()),
    );
    let failed =
        lifecycle_validator(repository, strayed, store, serial()).validate(holding_versions());
    let again = lifecycle_validator(repository, holding("errors\n"), store, serial())
        .validate(holding_versions())?;
    let kept = outcomes(&again)
        .into_iter()
        .zip(outcomes(&first))
        .all(|((_, holds, proving), _)| !holds || proving == Proving::Kept);
    Ok(
        if matches!(failed, Err(ValidateError::Stray { .. }))
            && kept_proofs(store) == held
            && kept
            && held > 0
        {
            String::new()
        } else {
            format!(
                "{failed:?}, {} proofs after, {held} before",
                kept_proofs(store)
            )
        },
    )
}

/// A run in which a proof fails writes nothing: after a validation that held, an edited rule is proved again under an
/// engine that refuses its query, and the store is neither written nor changed.
fn failed_proof_writes_nothing(
    repository: &Arc<RepositoryFake>,
    store: &Arc<StoreFake>,
) -> Result<Value, Failure> {
    let writes_so_far = || {
        *store
            .saves
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    };
    let _first = lifecycle_validator(repository, holding("errors\n"), store, serial())
        .validate(holding_versions())?;
    let (held, saved) = (kept_proofs(store), writes_so_far());
    let mut edited = holding("errors\n");
    let marker = ("/p".into(), "rules/marker/0.1.0/rule.toml".into());
    let text = edited.files.get(&marker).cloned().unwrap_or_default();
    drop(edited.files.insert(
        marker,
        format!("{text}\n# A scenario's comment edited.\n").into(),
    ));
    let second = lifecycle_validator_over(
        repository,
        edited,
        store,
        serial(),
        analysis_for(Scenario::Refused),
    )
    .validate(holding_versions())?;
    let failed = second
        .proved
        .iter()
        .filter(|proved| proved.proof != Proof::Proven)
        .count();
    Ok(
        if failed > 0 && held > 0 && kept_proofs(store) == held && writes_so_far() == saved {
            String::new()
        } else {
            format!(
                "{failed} proofs failed, {} proofs after and {} writes, {held} and {saved} before",
                kept_proofs(store),
                writes_so_far()
            )
        },
    )
}

/// Only the latest version is proved, and a past one when the lock pins it.
fn past_version(
    scenario: Scenario,
    repository: &Arc<RepositoryFake>,
    store: &Arc<StoreFake>,
) -> Result<Value, Failure> {
    let placed = if matches!(scenario, Scenario::PinnedPastVersion) {
        let pinned = workspace("p");
        drop(
            pinned
                .files
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert(RepoPath("tastemaker.lock".into()), FileText(PINS.into())),
        );
        Arc::new(pinned)
    } else {
        Arc::clone(repository)
    };
    let report = lifecycle_validator(&placed, with_past_marker(), store, serial()).validate(
        ValidateRequest {
            only: vec![super::types::VersionFilter("rule/marker".into())],
            ..everything()
        },
    )?;
    let proved = report
        .proved
        .iter()
        .map(|proved| proved.subject.id.0.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let expected = if matches!(scenario, Scenario::PinnedPastVersion) {
        "p/rule/marker@0.1.0 p/rule/marker@0.2.0"
    } else {
        "p/rule/marker@0.2.0"
    };
    Ok(if proved == expected {
        String::new()
    } else {
        proved
    })
}

/// The versions proved one after the other and on a pool answer the same report and tree.
fn serial_parallel(
    repository: &Arc<RepositoryFake>,
    store: &Arc<StoreFake>,
) -> Result<Value, Failure> {
    let parallel_store = Arc::new(StoreFake::default());
    let one = lifecycle_validator(repository, holding("errors\n"), store, serial())
        .validate(everything())?;
    let many = lifecycle_validator(
        &Arc::new(workspace("p")),
        holding("errors\n"),
        &parallel_store,
        Arc::new(crate::outbound::scheduler::factory::rayon(
            crate::outbound::scheduler::factory::pool(
                crate::outbound::scheduler::types::WorkerCount(2),
            ),
        )),
    )
    .validate(everything())?;
    let tables = |held_by: &StoreFake| {
        held_by
            .tables
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    };
    Ok(if one == many && tables(store) == tables(&parallel_store) {
        String::new()
    } else {
        format!("{:?} against {:?}", outcomes(&one), outcomes(&many))
    })
}

/// One scratch directory is made for every lint call, and none is left.
fn bounded(repository: &Arc<RepositoryFake>, store: &Arc<StoreFake>) -> Result<Value, Failure> {
    let _report = lifecycle_validator(repository, holding("errors\n"), store, serial()).validate(
        ValidateRequest {
            only: vec![super::types::VersionFilter("rule/marker".into())],
            ..everything()
        },
    )?;
    // Each scenario's project is a scratch directory of its own, in memory, discarded as the scenario ends; the lint
    // calls are handed its files and make none.
    let made = repository.scratched.load(Ordering::Relaxed);
    let scenarios = 2;
    let left = repository
        .scratches
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .len();
    Ok(if made == scenarios && left == 0 {
        String::new()
    } else {
        format!("{} made for the lint calls, {left} left", made - scenarios)
    })
}

/// Runs one use case twice over the same workspace: answers what is wrong, or nothing, when the second run asked the package for the
/// versions it offers as many times as the first.
fn lists_again(repository: &Arc<RepositoryFake>, store: &Arc<StoreFake>) -> Result<Value, Failure> {
    let packages = holding("errors\n");
    let listed = Arc::clone(&packages.listed);
    let validate = lifecycle_validator(repository, packages, store, serial());
    let count = || {
        listed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    };
    let _first = validate.validate(everything())?;
    let first = count();
    let _second = validate.validate(everything())?;
    let second = count() - first;
    Ok(if first > 0 && second == first {
        String::new()
    } else {
        format!("listed {first} times, then {second}")
    })
}

/// A cache, locked.
fn lock<T>(cache: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The sizes of the five caches of `validate`, each against its bound: nothing when every cache is within it and empty.
fn caches_stay_within_their_bounds(
    repository: &Arc<RepositoryFake>,
    store: &Arc<StoreFake>,
) -> Result<Value, Failure> {
    use super::constants::{MANIFESTS, OFFERS, PARSES, VERDICTS, WRAPPERS};
    use super::types::{Manifests, Offers, Parses, Verdicts, Wrappers};
    let validate = lifecycle_validator(repository, holding("errors\n"), store, serial());
    let _report = validate.validate(everything())?;
    let held = |size: usize, bound: usize, cache: &str| {
        (size > bound || size > 0).then(|| format!("{cache} holds {size}, bound {bound}"))
    };
    let caches = validate.caches();
    let Parses(parses) = caches.parses;
    let Manifests(manifests) = caches.manifests;
    let Offers(offers) = caches.offers;
    let Verdicts(verdicts) = caches.verdicts;
    let Wrappers(wrappers) = caches.wrappers;
    Ok([
        held(lock(parses).len(), PARSES, "Parses"),
        held(lock(manifests).len(), MANIFESTS, "Manifests"),
        held(lock(offers).len(), OFFERS, "Offers"),
        held(lock(verdicts).len(), VERDICTS, "Verdicts"),
        held(lock(wrappers).len(), WRAPPERS, "Wrappers"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("; "))
}
