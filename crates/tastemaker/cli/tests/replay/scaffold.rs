//! A language scaffolded from the real `tastemaker.toml` of ryo is whole: its crate compiles, and `tm check` finds no part
//! of it missing or misplaced. Findings about its content (the fixtures and rows the author writes) are the author's work and are not
//! asked about here.

mod contract;

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::languages::{
    TEMPLATE, created, given_content, initialized, over_samples, ran, reported,
};

/// The directory name of the language scaffolded, and so its crate's name.
const LANGUAGE: &str = "zz";

/// The parser library the scaffolded language names.
const LIBRARY: &str = "zz_parser";

/// The rules whose findings say a part is missing or misplaced, not that its content is unwritten: the suffixes of the placement checks.
const PLACEMENT: [&str; 6] = [
    "#count",
    "#closed",
    "#template",
    "#coverage",
    "#params",
    "#missing",
];

/// The rule that names a part a language lacks, and the one part a first scaffold cannot write: the sample files the allocation test reads
/// are the language's own text.
const PARTS: &str = "language-parts";
const SAMPLES: &str = "allocation";

/// The repository root: three directories above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

/// Copies a tree, leaving out every `target` directory, each workspace file pinned to the built `tm`.
fn copy(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == "target" {
            continue;
        }
        let (source, target) = (entry.path(), to.join(&name));
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            crate::fixes::link(&source, &target)?;
        } else if kind.is_dir() {
            copy(&source, &target)?;
        } else {
            let _: u64 = std::fs::copy(&source, &target)?;
            if name == crate::fixes::WORKSPACE_FILE {
                crate::fixes::pinned(&target)?;
            }
        }
    }
    Ok(())
}

/// Runs a command in `directory`, failing the test with its output when the command fails.
#[expect(
    clippy::expect_used,
    reason = "test support: a scene that cannot be written or a run that cannot start is a broken test, and the message names which"
)]
fn run(command: &mut Command, directory: &Path) {
    let output = command
        .current_dir(directory)
        .output()
        .expect("the command starts");
    assert!(
        output.status.success(),
        "{command:?} failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Runs `tm check` with `arguments` in `directory` and answers what it printed, failing the test when the check did not
/// run: it ends 0 when it finds nothing and 1 when it finds something.
#[expect(
    clippy::expect_used,
    reason = "test support: a check that cannot start is a broken test, and the message says so"
)]
fn checked(tastemaker: &str, arguments: &[&str], directory: &Path) -> Vec<u8> {
    let output = Command::new(tastemaker)
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect("tm check starts");
    let code = output.status.code();
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        code.is_some_and(|code| code <= 1_i32),
        "{arguments:?} failed: {said}"
    );
    output.stdout
}

/// The findings `tm check` gives the fresh language that say a part is missing or misplaced, after `tm validate` and
/// `tm check --fix` over `workspace`.
#[expect(
    clippy::expect_used,
    reason = "test support: a check that cannot start or answers no JSON is a broken test, and the message names which"
)]
fn misplaced_parts(tastemaker: &str, workspace: &Path) -> Vec<String> {
    run(Command::new(tastemaker).arg("validate"), workspace);
    // The check reports source findings by its exit code; validation errors are refused above.
    drop(checked(tastemaker, &["check", "--fix"], workspace));
    let printed = checked(tastemaker, &["check", "--format", "json"], workspace);
    let report: serde_json::Value = serde_json::from_slice(&printed).expect("a report in JSON");
    let own = format!("languages/{LANGUAGE}");
    report
        .get("findings")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|finding| {
            finding["path"]
                .as_str()
                .is_some_and(|path| path.contains(&own))
        })
        .filter(|finding| {
            let rule = finding["rule"].as_str().unwrap_or_default();
            let message = finding["message"].as_str().unwrap_or_default();
            (rule.ends_with(PARTS) && !message.contains(SAMPLES))
                || PLACEMENT.iter().any(|check| rule.contains(check))
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>()
}

#[test]
fn a_fresh_language_compiles_and_lacks_no_part() {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("scaffold-language");
    if scratch.exists() {
        std::fs::remove_dir_all(&scratch).expect("the previous scene removed");
    }
    let crates = scratch.join("crates");
    copy(&root().join("crates/ryo"), &crates.join("ryo")).expect("ryo copied");
    copy(&root().join("tastemaker"), &scratch.join("tastemaker"))
        .expect("the package tastemaker/ copied");
    let workspace = crates.join("ryo");
    run(Command::new("git").arg("init").arg("-q"), &scratch);
    let tastemaker = env!("CARGO_BIN_EXE_tm");
    run(
        Command::new(tastemaker).args([
            "scaffold",
            "language",
            "--at",
            &format!("languages/{LANGUAGE}"),
            "--set",
            &format!("library={LIBRARY}"),
            "--set",
            "ext=ts",
            "--set",
            "relation=syntax_tree",
        ]),
        &workspace,
    );
    let language = workspace.join(format!("languages/{LANGUAGE}"));
    for path in [
        "tests/fixtures/relations/syntax_tree/expected.rs",
        "tests/fixtures/relations/syntax_tree/input.ts",
    ] {
        assert!(language.join(path).is_file(), "missing {path}");
    }
    for (path, expected) in [
        (
            "src/domain/language/constants.rs",
            "EXTENSION: &str = \".ts\"",
        ),
        ("src/domain/language/service_test.rs", "src/main.ts"),
        ("benches/performance.rs", "generated.ts"),
        ("README.md", "`.ts`"),
    ] {
        let body = std::fs::read_to_string(language.join(path)).expect("generated file");
        assert!(body.contains(expected), "{path} lacks {expected}");
    }

    contract::assert_contract(&language);

    // The packages the copied workspace's lock names, fetched first: a machine that built only Tastemaker has not downloaded
    // every one of ryo's (`addr2line`, a dependency for another target, among them), and the check itself runs offline.
    run(Command::new("cargo").arg("fetch"), &workspace);
    run(
        Command::new("cargo")
            .args([
                "check",
                "-j",
                "6",
                "--offline",
                "--all-targets",
                "--all-features",
                "-p",
            ])
            .arg(format!("idiomatic-sh-ryo-{LANGUAGE}"))
            .env("CARGO_TARGET_DIR", env!("CARGO_TARGET_TMPDIR")),
        &workspace,
    );

    let misplaced = misplaced_parts(tastemaker, &workspace);
    assert!(
        misplaced.is_empty(),
        "the scaffolded language lacks or misplaces parts:\n{}",
        misplaced.join("\n")
    );
    std::fs::remove_dir_all(&scratch).expect("the scene removed");
}

/// A capture supplied for a collection also names the nested item the collection requires.
#[test]
fn a_captured_nested_item_is_written_and_held() {
    let scratch = tempfile::tempdir().expect("the scene directory");
    let directory = scratch.path();
    run(Command::new("git").args(["init", "-q"]), directory);
    // The package path is a TOML literal string: a Windows path's backslashes are no escapes there.
    std::fs::write(directory.join("tastemaker.toml"), format!(r#"workspace = {{ name = "nested", dependencies = [{{ package = "idiomatic.arch", path = '{}' }}] }}
closed = false
specs = [{{ alias = "subject", spec = "idiomatic.arch/collection", include = ["collections/{{collection}}"], count = 1, params = {{ item-trait = [] }} }}]
"#, root().join("tastemaker").display())).expect("the collection placement");
    let tastemaker = env!("CARGO_BIN_EXE_tm");
    let scaffold = || {
        run(
            Command::new(tastemaker).args([
                "scaffold",
                "subject",
                "--at",
                "collections/shapes",
                "--set",
                "item=circle",
            ]),
            directory,
        );
    };
    scaffold();
    for path in [
        "collections/shapes/circle.rs",
        "collections/shapes/circle/item.rs",
        "collections/shapes/circle/item_test.rs",
    ] {
        assert!(directory.join(path).is_file(), "missing {path}");
    }
    run(Command::new(tastemaker).arg("validate"), directory);
    let printed = checked(tastemaker, &["--json", "check", "--fix"], directory);
    let report: serde_json::Value = serde_json::from_slice(&printed).expect("check report");
    let missing = report["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .filter(|finding| {
            finding["rule"]
                .as_str()
                .is_some_and(|rule| rule.ends_with("#count"))
        })
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "missing required structure: {missing:?}"
    );
    let item = directory.join("collections/shapes/circle/item.rs");
    std::fs::write(&item, "// authored item remains intact\n").expect("author edit");
    scaffold();
    assert_eq!(
        std::fs::read_to_string(item).expect("authored item"),
        "// authored item remains intact\n"
    );
}

/// A condition is read from the prospective body, including during a dry run.
#[test]
fn a_conditional_use_case_interface_is_scaffolded() {
    for implemented in [false, true] {
        let scratch = tempfile::tempdir().expect("scene directory");
        let directory = scratch.path();
        run(Command::new("git").args(["init", "-q"]), directory);
        let implements = if implemented {
            r#"[{ use_case = "greet", crate = "engine", interface = "Greet" }]"#
        } else {
            "[]"
        };
        std::fs::write(directory.join("tastemaker.toml"), format!(r#"workspace = {{ name = "conditional", dependencies = [{{ package = "idiomatic.arch", path = '{}' }}] }}
closed = false
specs = [{{ alias = "subject", spec = "idiomatic.arch/use-case", include = ["src/inbound/use_cases/{{use_case}}"], count = 1, params = {{ implements = {implements} }} }}]
"#, root().join("tastemaker").display())).expect("placement");
        let tastemaker = env!("CARGO_BIN_EXE_tm");
        // A plan is said on stderr, as every `--help` promises; stdout holds nothing.
        let planned = Command::new(tastemaker)
            .args([
                "--json",
                "--dry-run",
                "scaffold",
                "subject",
                "--at",
                "src/inbound/use_cases/greet",
            ])
            .current_dir(directory)
            .output()
            .expect("the command starts");
        assert!(planned.status.success(), "the dry run ends 0");
        assert!(planned.stdout.is_empty(), "a plan is not said on stdout");
        let dry = String::from_utf8_lossy(&planned.stderr).into_owned();
        assert!(!directory.join("src").exists(), "dry run wrote files");
        let dry: serde_json::Value = serde_json::from_str(&dry).expect("dry report");
        assert_eq!(
            dry["written"]
                .as_array()
                .expect("written")
                .iter()
                .any(|path| path
                    .as_str()
                    .is_some_and(|path| path.ends_with("/interface.rs"))),
            !implemented
        );
        let args = ["scaffold", "subject", "--at", "src/inbound/use_cases/greet"];
        run(Command::new(tastemaker).args(args), directory);
        assert_eq!(
            directory
                .join("src/inbound/use_cases/greet/interface.rs")
                .is_file(),
            !implemented
        );
    }
}

/// The crate templates see files placed by another spec over the same directory.
#[test]
fn a_composed_crate_reads_its_placed_context() {
    let scratch = tempfile::tempdir().expect("scene directory");
    let directory = scratch.path();
    run(Command::new("git").args(["init", "-q"]), directory);
    std::fs::write(directory.join("tastemaker.toml"), format!(r#"workspace = {{ name = "composed", dependencies = [{{ package = "idiomatic.arch", path = '{}' }}] }}
closed = false
specs = [
{{ alias = "edge", spec = "idiomatic.arch/cli", include = ["app"], params = {{ product = "demo", library = "demo", markdown = "absent", writes = [] }} }},
{{ alias = "package", spec = "idiomatic.arch/crate", include = ["app"], params = {{ package = "demo", library = "demo", binary = "demo", distribution = "absent", product = "demo", folder = "demo" }} }}
]
"#, root().join("tastemaker").display())).expect("placement");
    let tastemaker = env!("CARGO_BIN_EXE_tm");
    run(
        Command::new(tastemaker).args([
            "scaffold",
            "edge",
            "--at",
            "app",
            "--set",
            "resource=greeting",
            "--set",
            "command=show",
        ]),
        directory,
    );
    run(
        Command::new(tastemaker).args(["scaffold", "package", "--at", "app"]),
        directory,
    );
    let manifest = std::fs::read_to_string(directory.join("app/Cargo.toml")).expect("manifest");
    assert!(manifest.contains("clap.workspace = true"), "{manifest}");
    let library = std::fs::read_to_string(directory.join("app/src/lib.rs")).expect("library");
    assert!(library.contains("pub mod resources;"), "{library}");
    assert!(directory.join("app/src/cases.rs").is_file());
}

/// `tm scaffold`, over each sample under `crates/tastemaker/examples/`: a template placed at a source file is written, and the project reads it
/// without a finding, whatever its language.
#[test]
fn scaffold_writes_a_file_the_project_reads_without_a_finding() {
    over_samples("scaffold", |sample| {
        let scratch = initialized(sample)?;
        let project = scratch.path();
        let file = format!("src/{TEMPLATE}.{}", sample.extension);
        created(&format!("template/{TEMPLATE}"), sample, project)?;
        given_content(project, sample)?;
        ran(&["add", TEMPLATE, "--at", &file], project)?;
        ran(&["validate"], project)?;
        ran(&["scaffold", TEMPLATE], project)?;
        if !project.join(&file).is_file() {
            return Err(format!("tm scaffold wrote no {file}"));
        }
        let found = reported(project)?;
        if found.is_empty() {
            Ok(())
        } else {
            Err(format!("the scaffolded project reports {found:?}"))
        }
    });
}
