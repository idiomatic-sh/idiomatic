//! What the replay suites that hold ryo's manifest rows to each ecosystem's own tool share: the rows ryo writes of a project, what a
//! tool says of it, and the comparison of the two, line by line. What only the replay's suites of the other ecosystems ask is `ecosystems`'.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

/// The query of the members every reference check compares: each workspace with each package it holds.
const MEMBERS: &str =
    "let m = select * from \"workspace-member\"; emit m subject workspace key(workspace, package);";

/// A rule written as Tastemaker writes it, as ryo is asked to run it.
#[must_use]
pub(crate) fn rule_from_query(
    id: &str,
    query: &str,
    include: &[String],
) -> ryo_core::inbound::use_cases::lint::types::Rule {
    ryo_core::inbound::use_cases::lint::types::Rule {
        observations: Vec::new(),
        id: ryo_values::domain::runs::types::RuleId(id.into()),
        query: ryo_values::domain::query_tokens::types::QueryBundle::from(
            ryo_values::domain::plan_parts::types::QueryText(query.into()),
        ),
        applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
        parameters: Vec::new(),
        include: include
            .iter()
            .map(|glob| ryo_values::domain::files::types::Glob(glob.as_str().into()))
            .collect(),
        exclude: Vec::new(),
    }
}

/// What ryo says of the manifests below `root`, read by Rust and `languages`: one line for each finding of the members query and of
/// `dependencies`, the id of its rule and its key, each column as ryo shows it, joined by commas.
pub(crate) fn ryo_lines(
    root: &Path,
    languages: Vec<ryo_core::outbound::language::factory::LanguageConnection>,
    dependencies: &str,
) -> BTreeSet<String> {
    let rules = vec![
        rule_from_query("members", MEMBERS, &["**".to_owned()]),
        rule_from_query("dependencies", dependencies, &["**".to_owned()]),
    ];
    let (rows, _) = crate::base::lint_report_with_languages(
        root,
        rules,
        Arc::new(ryo_execution_runtime::outbound::facts::factory::memory()),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
        Vec::new(),
        false,
        languages,
    )
    .expect("the rules run");
    // A finding's row is its rule, its subject, its key, its line and its character.
    rows.iter()
        .map(|row| {
            let cell = |place: usize| row.get(place).expect("a finding's row has its cells");
            format!("{} {}", cell(0), cell(2))
        })
        .collect()
}

/// What Windows itself needs of the environment to start a program, passed down beside `PATH`; none on other platforms.
const SYSTEM: &[&str] = if cfg!(windows) {
    &[
        "SystemRoot",
        "SystemDrive",
        "ComSpec",
        "PATHEXT",
        "TEMP",
        "TMP",
    ]
} else {
    &[]
};

/// The program `program` names on the build's `PATH`: on Windows the first of its `.exe`, `.cmd` and `.bat` (npm and pnpm are
/// batch files there, which a bare name does not find); the name as it is elsewhere.
fn found(program: &str) -> std::path::PathBuf {
    if cfg!(windows) {
        for directory in std::env::split_paths(env!("PATH")) {
            for extension in ["exe", "cmd", "bat"] {
                let candidate = directory.join(program).with_extension(extension);
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }
    std::path::PathBuf::from(program)
}

/// What `program` prints when it runs `arguments` in `directory`, given the `PATH` the build that made this test had and `variables`:
/// nothing else is passed down. What it says on stderr when it fails.
pub(crate) fn tool_result(
    program: &str,
    arguments: &[&str],
    directory: &Path,
    variables: &[(&str, &str)],
) -> Result<String, String> {
    let output = std::process::Command::new(found(program))
        .args(arguments)
        .current_dir(directory)
        .env_clear()
        .env("PATH", env!("PATH"))
        .envs(
            SYSTEM
                .iter()
                .filter_map(|name| Some((*name, std::env::var_os(name)?))),
        )
        .envs(variables.iter().copied())
        .output()
        .unwrap_or_else(|error| panic!("`{program}` runs: {error}"));
    if output.status.success() {
        Ok(String::from_utf8(output.stdout).expect("the tool writes UTF-8"))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

/// The path of `path` relative to `root`, none when it lies outside.
pub(crate) fn relative(root: &Path, path: &Path) -> Option<String> {
    // Both canonical, as a tool prints its own spelling of a place (Windows' long names where the root has a short one),
    // and joined by `/`, as ryo keys every path.
    let canonical = |place: &Path| place.canonicalize().unwrap_or_else(|_| place.to_path_buf());
    let (root, path) = (canonical(root), canonical(path));
    path.strip_prefix(&root).ok().map(|inside| {
        inside
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    })
}

/// One line for each line only one side has, naming the side; none when ryo and the tool agree.
pub(crate) fn differences(
    name: &str,
    tool: &str,
    ours: &BTreeSet<String>,
    theirs: &BTreeSet<String>,
) -> Vec<String> {
    let mut found: Vec<String> = ours
        .difference(theirs)
        .map(|line| format!("{name}: only ryo says {line}"))
        .collect();
    found.extend(
        theirs
            .difference(ours)
            .map(|line| format!("{name}: only {tool} says {line}")),
    );
    found
}

/// The JSON document `text` is.
pub(crate) fn json(text: &str) -> serde_json::Value {
    serde_json::from_str(text).expect("the tool writes JSON")
}

/// The text of the JSON value `value` at `key`, empty when it is absent or not text.
pub(crate) fn text_at<'value>(value: &'value serde_json::Value, key: &str) -> &'value str {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
}
