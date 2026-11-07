//! `tm create`, over each sample: the rule it writes gives its scenarios files in the project's own language.

use crate::languages::{RULE, created, created_rule, initialized, over_samples, ran};

/// The files each scenario of `rule` is given: the path and the text of each, in order.
fn scenario_files(rule: &toml::Table) -> Vec<(String, String)> {
    rule.get("scenarios")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|scenario| scenario.get("given")?.get("files")?.as_table().cloned())
        .flat_map(|files| {
            files
                .into_iter()
                .map(|(path, text)| (path, text.as_str().unwrap_or_default().to_owned()))
        })
        .collect()
}

#[test]
fn create_writes_a_rule_whose_scenarios_are_in_the_project_language() {
    over_samples("create rule", |sample| {
        let scratch = initialized(sample)?;
        created(&format!("rule/{RULE}"), sample, scratch.path())?;
        let files = scenario_files(&created_rule(scratch.path())?);
        let suffix = format!(".{}", sample.extension);
        let foreign: Vec<&String> = files
            .iter()
            .map(|(path, _)| path)
            .filter(|path| !path.ends_with(&suffix))
            .collect();
        if !files.is_empty() && foreign.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "the rule's scenario files {foreign:?} are not `{suffix}` files"
            ))
        }
    });
}

#[test]
fn create_with_no_extension_is_refused_naming_the_argument() {
    over_samples("create rule with no extension", |sample| {
        let scratch = initialized(sample)?;
        match ran(&["create", &format!("rule/{RULE}")], scratch.path()) {
            Ok(()) => Err("tm create wrote a rule with no extension named".to_owned()),
            Err(said) if said.contains("--set extension=<ext>") => Ok(()),
            Err(said) => Err(format!(
                "tm create refused without naming `--set extension=<ext>`:\n{said}"
            )),
        }
    });
}

/// The `tm create` lines of the product README's `console` blocks, run in order as written in a fresh project: each succeeds.
#[test]
fn the_readme_create_lines_run_as_written() {
    let readme = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../README.md"),
    )
    .expect("the README");
    let mut shell = false;
    let mut lines = Vec::new();
    for line in readme.lines() {
        if line.starts_with("```") {
            shell = line == "```console";
        } else if shell && let Some(arguments) = line.strip_prefix("$ tm create ") {
            lines.push(arguments.to_owned());
        } else {
            // Not a `tm create` line of a `console` block.
        }
    }
    assert!(!lines.is_empty(), "the README has `tm create` lines");
    let scratch = tempfile::tempdir().expect("the scene directory");
    let project = scratch.path();
    std::fs::create_dir_all(project.join("src")).expect("src");
    std::fs::write(
        project.join("src/lib.rs"),
        "pub fn value() -> u8 {\n    1\n}\n",
    )
    .expect("src/lib.rs");
    let repository = std::process::Command::new("git")
        .args(["init", "-q"])
        .arg(project)
        .status()
        .expect("git starts");
    assert!(repository.success(), "git init succeeds");
    ran(&["init"], project).expect("tm init succeeds");
    let failures: Vec<String> = lines
        .iter()
        .filter_map(|arguments| {
            let mut words = vec!["create"];
            words.extend(arguments.split_whitespace());
            ran(&words, project).err()
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// The extensions of the languages the command line links, one each.
const EXTENSIONS: [&str; 10] = [
    "css", "go", "html", "js", "json", "md", "py", "rs", "toml", "ts",
];

/// The rule `tm create` writes proves itself by its own scenarios whatever the extension it is given, in every language the
/// command line links.
#[test]
fn a_created_rule_proves_itself_in_every_linked_language() {
    let scratch = tempfile::tempdir().expect("the scene directory");
    let project = scratch.path();
    let repository = std::process::Command::new("git")
        .args(["init", "-q"])
        .arg(project)
        .status()
        .expect("git starts");
    assert!(repository.success(), "git init succeeds");
    ran(&["init"], project).expect("tm init succeeds");
    let failures: Vec<String> = EXTENSIONS
        .iter()
        .filter_map(|extension| {
            let rule = format!("rule/empty-{extension}");
            ran(
                &["create", &rule, "--set", &format!("extension={extension}")],
                project,
            )
            .and_then(|()| ran(&["validate", &rule], project))
            .err()
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
