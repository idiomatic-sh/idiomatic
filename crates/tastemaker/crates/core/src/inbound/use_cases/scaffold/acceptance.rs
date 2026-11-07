//! The steps of the `scaffold` feature: each a cucumber expression over the world, asserting the use case's own results.

use cucumber::{given, then, when};

use crate::Behavior;

/// Setup failures reported by the scenario runner.
type Failure = Box<dyn std::error::Error + Send + Sync>;
use std::collections::BTreeMap;
use std::sync::Arc;

use tastemaker_core::domain::files::types::MachinePath;
use tastemaker_core::domain::packages::types::DefinitionName;
use tastemaker_core::domain::paths::types::RepoPath;
use tastemaker_core::domain::writing::types::Writing;
use tastemaker_core::inbound::use_cases::check::fixtures::{package, repository};
use tastemaker_core::inbound::use_cases::scaffold::interface::ScaffoldUseCase as _;
use tastemaker_core::inbound::use_cases::scaffold::types::ScaffoldRequest;
use tastemaker_core::inbound::use_cases::scaffold::use_case::ScaffoldUseCaseImpl;

/// Sets what the `scaffold` scenario hands the use case.
#[given(expr = "the scaffold input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `scaffold` on the scenario's input and records its answer.
#[when(expr = "scaffold runs")]
fn runs(world: &mut Behavior) -> Result<(), Failure> {
    if world.input.starts_with("disk-") {
        world.answer = on_disk(&world.input)?;
        return Ok(());
    }
    let scaffold = ScaffoldUseCaseImpl::new(
        Arc::new(repository()),
        Arc::new(tastemaker_core::outbound::manifest::factory::toml()),
        Arc::new(package()),
        Arc::new(tastemaker_core::outbound::matching::factory::globset()),
        Arc::new(tastemaker_core::outbound::templates::factory::minijinja()),
        Arc::new(tastemaker_core::outbound::analysis::fixtures::fake::AnalysisFake::default()),
    );
    let (entry, at) = if world.input == "absent" {
        ("absent", "src/c")
    } else {
        ("part", world.input.as_str())
    };
    world.answer = match scaffold.scaffold(ScaffoldRequest {
        start: MachinePath("/fake".into()),
        entry: DefinitionName(entry.into()),
        at: RepoPath(at.into()),
        set: BTreeMap::new(),
        writing: Writing::DryRun,
    }) {
        Ok(report) => report
            .written
            .iter()
            .map(|path| format!("wrote {}", path.0))
            .collect::<Vec<_>>()
            .join(" "),
        Err(error) => error.to_string(),
    };
    Ok(())
}

/// Compares the answer of `scaffold` with the one the scenario expects.
#[then(expr = "scaffold answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    // The step's text, kept as the answer's type: what a cucumber argument arrives as is an owned `String`.
    let expected: Box<str> = expected.into();
    assert_eq!(
        &*world.answer, &*expected,
        "scaffold answers what the scenario expects"
    );
}

/// The production repository, package, template and Ryo adapters settle a workspace on disk.
fn on_disk(input: &str) -> Result<String, Failure> {
    use tastemaker_core::domain::entries::types::Setting;
    use tastemaker_core::inbound::use_cases::scaffold::errors::ScaffoldError;
    use tastemaker_core::outbound::{
        analysis, manifest, matching, packages, repository, templates,
    };
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    assert!(
        std::process::Command::new("git")
            .args(["init", "-q"])
            .arg(root)
            .status()?
            .success(),
        "git initializes the scene's repository"
    );
    lay_out(root, input)?;
    let sibling = input == "disk-sibling";
    let scaffold = ScaffoldUseCaseImpl::new(
        Arc::new(repository::factory::disk(
            Vec::new(),
            tastemaker_core::domain::files::types::DirectoryName(".tastemaker_cache".into()),
        )),
        Arc::new(manifest::factory::toml()),
        Arc::new(packages::factory::path()),
        Arc::new(matching::factory::globset()),
        Arc::new(templates::factory::minijinja()),
        Arc::new(analysis::factory::ryo(
            ryo::linter::Languages(Vec::new()),
            analysis::types::AnalysisSettings {
                pool: tastemaker_core::outbound::scheduler::factory::pool(
                    tastemaker_core::outbound::scheduler::types::WorkerCount(1),
                ),
                memory: analysis::types::Memory::Machine,
            },
        )),
    );
    let answer = scaffold.scaffold(ScaffoldRequest {
        start: MachinePath(root.display().to_string().into()),
        entry: DefinitionName("subject".into()),
        at: RepoPath("src".into()),
        // Only the unresolved scene's condition reads `bound`; scaffold refuses a name nothing reads.
        set: if input == "disk-unresolved" {
            BTreeMap::from([(Setting("bound".into()), Setting("0".into()))])
        } else {
            BTreeMap::new()
        },
        writing: Writing::Write,
    });
    Ok(match answer {
        Ok(_) if sibling => {
            assert_eq!(
                std::fs::read_to_string(root.join("src/dependent.txt"))?,
                "dependent",
                "the file that needs its sibling is written with it"
            );
            "dependent written".to_owned()
        }
        Err(ScaffoldError::Unsettled) => {
            assert!(
                !root.join("flag.txt").exists(),
                "an unsettled scaffold writes no file"
            );
            "unsettled, no files written".to_owned()
        }
        Err(ScaffoldError::Undecided { .. }) => {
            assert!(
                !root.join("flag.txt").exists(),
                "an undecided scaffold writes no file"
            );
            "undecided, no files written".to_owned()
        }
        other => return Err(format!("unexpected scaffold answer: {other:?}").into()),
    })
}

/// Writes the scene `input` names into `root`: the workspace file, its local package and the definitions they hold.
fn lay_out(root: &std::path::Path, input: &str) -> Result<(), Failure> {
    use std::fmt::Write as _;
    let package = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .ok_or("repository root is unavailable")?
        .join("tastemaker");
    // The package's path as a TOML literal string: a Windows path's backslashes are no escapes there.
    let mut workspace = format!(
        "workspace={{name=\"scene\",dependencies=[{{package=\"idiomatic.arch\",path='{}'}},{{package=\"p\",path=\"p\"}}]}}\nclosed=false\n",
        package.display()
    );
    let local = root.join("p");
    std::fs::create_dir_all(&local)?;
    std::fs::write(local.join("package.toml"), "name=\"p\"\n")?;
    let sibling = input == "disk-sibling";
    if sibling {
        workspace.push_str("specs=[{alias=\"subject\",spec=\"p/part\",include=[\"src\"]}]\n");
        let spec = local.join("specs/part/0.1.0");
        std::fs::create_dir_all(&spec)?;
        std::fs::write(
            spec.join("spec.toml"),
            "description=\"A pair\"\nwhy=\"One file needs the other\"\ntemplates=[{template=\"dependent\",file=\"dependent.txt\"},{template=\"marker\",file=\"marker.txt\"}]\n",
        )?;
        for (name, body) in [
            (
                "dependent",
                "{% if present.marker | default(false) %}dependent{% endif %}",
            ),
            ("marker", "marker"),
        ] {
            let template = spec.join("templates").join(name);
            std::fs::create_dir_all(&template)?;
            std::fs::write(
                template.join("template.toml"),
                format!("description=\"Text\"\nwhy=\"Text\"\ncontent=\"{body}\"\n"),
            )?;
        }
    } else {
        let bound = if input == "disk-unresolved" {
            "{ param = \"bound\" }"
        } else {
            "0"
        };
        let include = if input == "disk-unresolved" {
            "tastemaker.toml"
        } else {
            "flag.txt"
        };
        write!(
            workspace,
            r#"conditions = {{ flag = {{ description = "The flag exceeds its bound.", rule = {{ rule = "idiomatic.arch/file-length", include = ["{include}"], params = {{ max-lines = {bound} }} }} }} }}
templates = [{{ alias = "subject", template = "idiomatic.arch/readme", file = "flag.txt", count = {{ when = {{ condition = "flag" }}, then = 0, else = 1 }}, params = {{ name = "flag" }} }}]
"#
        )?;
    }
    std::fs::write(root.join("tastemaker.toml"), workspace)?;
    Ok(())
}
