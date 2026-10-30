//! Initialization acceptance with the production filesystem and manifest adapters.
use crate::Behavior;
use cucumber::{given, then, when};
use std::sync::Arc;
use tastemaker_core::domain::definitions::types::Role;
use tastemaker_core::domain::files::types::MachinePath;
use tastemaker_core::domain::writing::types::Writing;
use tastemaker_core::inbound::use_cases::init::{
    interface::InitUseCase as _, types::InitRequest, use_case::InitUseCaseImpl,
};
use tastemaker_core::outbound::manifest::interface::ManifestPort as _;
use tastemaker_core::outbound::manifest::types::DefinitionText;
use tastemaker_core::outbound::{manifest, repository};
/// What an acceptance step fails with: any error the use cases or the filesystem raise.
type Failure = Box<dyn std::error::Error + Send + Sync>;
/// Sets what the `init` scenario hands the use case.
#[given(expr = "the init input {string}")]
fn given_input(world: &mut Behavior, input: String) {
    world.input = input;
}
/// Runs `init` on the scenario's input and records its answer.
#[when(expr = "init runs")]
fn runs(world: &mut Behavior) -> Result<(), Failure> {
    let project = tempfile::tempdir()?;
    let root = project.path();
    // A project is a git repository: `init` refuses one outside any.
    assert!(
        std::process::Command::new("git")
            .args(["init", "-q"])
            .arg(root)
            .status()?
            .success(),
        "git initializes the project's repository"
    );
    let service = InitUseCaseImpl::new(
        Arc::new(repository::factory::disk(
            Vec::new(),
            tastemaker_core::domain::files::types::DirectoryName(".tastemaker_cache".into()),
        )),
        Arc::new(manifest::factory::toml()),
    );
    let request = InitRequest {
        target: MachinePath(root.display().to_string().into()),
        writing: if world.input == "preview" {
            Writing::DryRun
        } else {
            Writing::Write
        },
    };
    if world.input == "fresh" {
        // A project that already holds sources: init's closed workspace places them.
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::write(root.join("src/good.txt"), "valid source")?;
    }
    if world.input == "conflict" {
        std::fs::create_dir_all(root.join("tastemaker"))?;
        std::fs::write(root.join("tastemaker/package.toml"), "owner assets")?;
    }
    if world.input == "rerun" {
        assert!(
            !service.init(request.clone())?.written.is_empty(),
            "the first init writes the project's files"
        );
    }
    let before = std::fs::read(root.join("tastemaker/package.toml")).ok();
    let result = service.init(request);
    match world.input.as_str() {
        "preview" => {
            assert!(
                !result?.written.is_empty(),
                "a preview lists the files it plans"
            );
            assert!(
                !root.join("tastemaker").exists(),
                "a preview writes no package directory"
            );
            assert!(
                !root.join("tastemaker.toml").exists(),
                "a preview writes no workspace file"
            );
            "no files written"
        }
        "rerun" | "conflict" => {
            assert!(result.is_err(), "an init over existing files is refused");
            assert_eq!(
                std::fs::read(root.join("tastemaker/package.toml")).ok(),
                before,
                "a refused init leaves the existing package file as it was"
            );
            "existing files preserved"
        }
        "fresh" => {
            assert!(
                !result?.written.is_empty(),
                "a fresh init lists the files it wrote"
            );
            let text = std::fs::read_to_string(root.join("tastemaker.toml"))?;
            let workspace =
                manifest::factory::toml().workspace_file(&DefinitionText(text.into()))?;
            assert!(
                matches!(workspace.role, Role::Root(_)),
                "init writes a root workspace file"
            );
            assert!(
                root.join("tastemaker/package.toml").is_file(),
                "init writes the local package's file"
            );
            "local package initialized"
        }
        _ => return Err("unknown initialization scenario".into()),
    }
    .clone_into(&mut world.answer);
    world.project = Some(project);
    Ok(())
}
/// Compares the answer of `init` with the one the scenario expects.
#[then(expr = "init answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    let expected: Box<str> = expected.into();
    assert_eq!(
        world.answer, &*expected,
        "init answers what the scenario expects"
    );
}

/// A project authors its own asset and observes the complete enforcement loop.
#[then("the initialized project can author and enforce its own rule")]
fn author_and_enforce(world: &mut Behavior) -> Result<(), Failure> {
    let project = world
        .project
        .as_ref()
        .ok_or("initialization did not retain its project")?;
    let root = project.path();
    assert!(
        std::process::Command::new("git")
            .args(["init", "-q"])
            .arg(root)
            .status()?
            .success(),
        "git initializes the project's repository"
    );
    let start = MachinePath(root.display().to_string().into());
    author(root, &start)?;
    enforce(root, start)
}

/// Creates the project's own rule, writes its query and a valid source, and adds the rule over `src`.
fn author(root: &std::path::Path, start: &MachinePath) -> Result<(), Failure> {
    use tastemaker_core::domain::{
        packages::types::{DefinitionKind, DefinitionName},
        paths::types::RepoPath,
    };
    use tastemaker_core::inbound::use_cases::{
        add::{
            interface::AddUseCase as _,
            types::{AddRequest, DefinitionAsked},
            use_case::AddUseCaseImpl,
        },
        create::{
            interface::CreateUseCase as _,
            types::{CreateRequest, Destination, Extension, ScenarioExtension},
            use_case::CreateUseCaseImpl,
        },
    };
    use tastemaker_core::outbound::{matching, packages};
    let created = CreateUseCaseImpl::new(
        Arc::new(repository::factory::disk(
            Vec::new(),
            tastemaker_core::domain::files::types::DirectoryName(".tastemaker_cache".into()),
        )),
        Arc::new(manifest::factory::toml()),
        Arc::new(packages::factory::path()),
    )
    .create(CreateRequest {
        start: start.clone(),
        name: DefinitionName("no-prohibited-file".into()),
        kind: DefinitionKind::Rule,
        into: Destination::Workspace,
        extension: Extension::Named(ScenarioExtension("txt".into())),
        writing: Writing::Write,
    })?;
    assert_eq!(
        &*created.name.0, "no-prohibited-file",
        "create writes the rule it was asked for"
    );
    let rule = root.join("tastemaker/rules/no-prohibited-file/0.1.0/rule.toml");
    assert!(rule.is_file(), "create writes the rule's file");
    std::fs::write(rule, LOCAL_RULE)?;
    let added = AddUseCaseImpl::new(
        Arc::new(repository::factory::disk(
            Vec::new(),
            tastemaker_core::domain::files::types::DirectoryName(".tastemaker_cache".into()),
        )),
        Arc::new(manifest::factory::toml()),
        Arc::new(packages::factory::path()),
        Arc::new(matching::factory::globset()),
    )
    .add(AddRequest {
        start: start.clone(),
        definition: DefinitionAsked("local/no-prohibited-file".into()),
        at: RepoPath("src".into()),
        params: std::collections::BTreeMap::new(),
        writing: Writing::Write,
    })?;
    assert_eq!(
        &*added.reference.0, "no-prohibited-file",
        "add enters the rule it was asked for"
    );
    Ok(())
}

/// Validates the rule, then checks the project: the valid source and the files Asymptote adds have no finding, and a prohibited file is the one.
fn enforce(root: &std::path::Path, start: MachinePath) -> Result<(), Failure> {
    use tastemaker_core::domain::proofs::types::Proof;
    use tastemaker_core::inbound::use_cases::{
        check::{
            interface::CheckUseCase as _,
            types::{CheckRequest, Fixing},
        },
        validate::{interface::ValidateUseCase as _, types::ValidateRequest},
    };
    let (check, validate) = crate::check::real_services(root, "init-acceptance");
    let proofs = validate.validate(ValidateRequest {
        start: start.clone(),
        only: Vec::new(),
    })?;
    assert_eq!(
        proofs.proved.len(),
        2,
        "validate proves the project's rule and the unchecked template init copied"
    );
    assert!(
        proofs
            .proved
            .iter()
            .all(|proved| proved.proof == Proof::Proven),
        "the scenarios of both hold"
    );
    let request = CheckRequest {
        start,
        within: Vec::new(),
        fixing: Fixing::Fix,
        writing: Writing::Write,
    };
    assert!(
        check.check(request.clone())?.report.findings.is_empty(),
        "the good source has no finding"
    );
    // A project that adds Asymptote after `init`: its configuration and its committed lock are not findings of the closed root.
    std::fs::create_dir_all(root.join(".asymptote/operations"))?;
    std::fs::write(
        root.join("asymptote.toml"),
        "[hooks]\npre-commit = [\"lint\"]\n",
    )?;
    std::fs::write(
        root.join(".asymptote/operations/0a1b.lock"),
        "version = 1\n",
    )?;
    assert!(
        check.check(request.clone())?.report.findings.is_empty(),
        "asymptote's configuration and lock are no finding"
    );
    std::fs::write(root.join("src/bad.txt"), "prohibited source")?;
    assert_eq!(
        check.check(request)?.report.findings.len(),
        1,
        "the prohibited file is the one finding"
    );
    Ok(())
}

/// A filesystem rule needs no syntax provider and proves both allowed and forbidden files.
const LOCAL_RULE: &str = r#"description = "A prohibited file is reported."
why = "This project reserves the bad.txt filename."
query = '''
let found = select * from "filesystem.file" where path = "src/bad.txt";
emit found subject "path" key("path");
'''
[diagnostic]
primary = "path"
message = "a prohibited file"
help = "Rename the file."
[[scenarios]]
id = "allowed-file"
scenario = "A file that is not the reserved one is not reported."
given.include = ["**"]
given.files."src/good.txt" = "valid source"
[[scenarios]]
id = "prohibited-file"
scenario = "The reserved file name is reported."
given.include = ["**"]
given.files."src/bad.txt" = "prohibited source"
then.findings = [{ path = "src/bad.txt" }]
"#;
