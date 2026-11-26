//! Rules whose queries need what today's ryo lacks run here, over their own Tastemaker scenarios: the
//! query of `rule.toml`, the files of each scenario's `given.files`, and the findings its `then.findings`
//! states, written by hand, never from this run.

use crate::base;
use std::path::Path;

use ryo_core::inbound::use_cases::lint::types::RuleParameter;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};

/// A rule is read where `base::rules` finds it: a replay rule under `tests/fixtures/replay-rules`, the proof of what the engine
/// decides that no Tastemaker rule ships, or a rule of the package. Every scenario of the rule gives the findings its
/// `then.findings` state, by the path of the file.
pub(crate) fn check(rule: &str) {
    let manifest =
        std::fs::read_to_string(base::rules::directory(rule).join("rule.toml")).expect("the rule");
    let query = base::rules::query(rule);
    let table: toml::Table = manifest.parse().expect("a rule that reads");
    let scenarios = table
        .get("scenarios")
        .and_then(toml::Value::as_array)
        .expect("scenarios");
    assert!(!scenarios.is_empty(), "{rule} has scenarios");
    for scenario in scenarios {
        let id = scenario
            .get("id")
            .and_then(toml::Value::as_str)
            .expect("an id");
        // What the scenario starts from: the rule's shared `[given]`, the scenario's own keys winning and its files and params
        // added to the shared ones.
        let mut merged = table
            .get("given")
            .and_then(toml::Value::as_table)
            .cloned()
            .unwrap_or_default();
        for (key, value) in scenario
            .get("given")
            .and_then(toml::Value::as_table)
            .into_iter()
            .flatten()
        {
            match (merged.get_mut(key), value) {
                (Some(toml::Value::Table(shared)), toml::Value::Table(own))
                    if key == "files" || key == "params" =>
                {
                    shared.extend(own.clone());
                }
                _ => drop(merged.insert(key.clone(), value.clone())),
            }
        }
        let given = Some(&merged);
        let root = tempfile::tempdir().expect("a directory for the scenario's files");
        for (path, text) in given
            .and_then(|given| given.get("files"))
            .and_then(toml::Value::as_table)
            .into_iter()
            .flatten()
        {
            let file = root.path().join(path);
            std::fs::create_dir_all(file.parent().expect("a directory above the file"))
                .expect("a directory for the file");
            std::fs::write(file, text.as_str().expect("the file's text")).expect("the file");
        }
        let words = |key: &str| -> Vec<String> {
            given
                .and_then(|given| given.get(key))
                .and_then(toml::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|each| each.as_str().map(str::to_owned))
                .collect()
        };
        let include = match words("include") {
            listed if listed.is_empty() => vec!["**".to_owned()],
            listed => listed,
        };
        let mut each = crate::tools::rule_from_query(rule, &query, &include);
        each.parameters = parameters_of(given);
        // A directory an entry excludes holds everything beneath it.
        each.exclude = words("exclude")
            .iter()
            .flat_map(|glob| [glob.clone(), format!("{glob}/**")])
            .map(|glob| Glob(glob.as_str().into()))
            .collect();
        let hosted = if query.contains(PLACED) {
            vec![placed_manifests(root.path())]
        } else {
            Vec::new()
        };
        let (rows, report) = base::lint_report_hosted(
            root.path(),
            vec![each],
        ryo_sdk::linter::Cache::None,
            hosted,
            true,
        )
        .expect("every rule scenario runs");
        let mut found: Vec<String> = rows.iter().filter_map(|row| row.get(1).cloned()).collect();
        let findings: Vec<&toml::Value> = scenario
            .get("then")
            .and_then(|then| then.get("findings"))
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
            .collect();
        let rule_of = |finding: &toml::Value| {
            finding
                .get("rule")
                .and_then(toml::Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        // A rule that cannot decide is an `#unknown` finding of its scenario, whichever file is the reason.
        let unknown_wanted = findings
            .iter()
            .any(|finding| rule_of(finding).ends_with("#unknown"));
        assert_eq!(
            !report.unknowns.is_empty(),
            unknown_wanted,
            "{rule}: unknowns of scenario {id}"
        );
        // A finding of Tastemaker's own checks (`tastemaker#ignore` of a directive the scene applies no rule for) is
        // Tastemaker's to give, not the query's: only the rule's own findings are replayed here.
        let mut wanted: Vec<String> = findings
            .iter()
            .filter(|finding| !rule_of(finding).ends_with("#unknown"))
            .filter(|finding| !rule_of(finding).starts_with("tastemaker#"))
            .filter_map(|finding| finding.get("path")?.as_str().map(str::to_owned))
            .collect();
        wanted.sort();
        wanted.dedup();
        found.sort();
        found.dedup();
        assert_eq!(found, wanted, "{rule}: scenario {id}");
    }
}

#[test]
fn no_language_names_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("no-language-names");
}

/// The relation Tastemaker hosts for the crates it places.
const PLACED: &str = "tastemaker.placed";

/// The values a scenario binds the rule's params to, each as `base::parameter` binds it: a value it cannot bind fails.
fn parameters_of(given: Option<&toml::Table>) -> Vec<RuleParameter> {
    let Some(params) = given
        .and_then(|given| given.get("params"))
        .and_then(toml::Value::as_table)
    else {
        return Vec::new();
    };
    params
        .iter()
        .map(|(name, value)| {
            base::parameter(name, value)
                .unwrap_or_else(|| panic!("the scenario's param `{name}` = {value} is not bound"))
        })
        .collect()
}

/// `tastemaker.placed` as the fixtures' Tastemaker fills it: every manifest of the tree is placed with the `cargo-package` template, and
/// every `smoke_test.rs` with `smoke-test`.
fn placed_manifests(root: &Path) -> HostRelation {
    let columns = ["path", "entry", "package", "template", "version", "spec"];
    let mut manifests = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .expect("a directory of the fixture")
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if let Some(template) = path.file_name().and_then(|name| match name.to_str() {
                Some("Cargo.toml") => Some("cargo-package"),
                Some("smoke_test.rs") => Some("smoke-test"),
                _ => None,
            }) {
                manifests.push((path, template));
            }
        }
    }
    manifests.sort();
    let text = |value: &str| ryo_values::domain::row_texts::types::Cell::Text(value.into());
    HostRelation {
        name: HostName(PLACED.into()),
        columns: columns
            .iter()
            .enumerate()
            .map(|(index, name)| HostColumn {
                name: HostColumnName((*name).into()),
                kind: if index == 0 {
                    HostKind::File
                } else {
                    HostKind::Text
                },
            })
            .collect(),
        rows: manifests
            .iter()
            .map(|(path, template)| {
                let relative = path.strip_prefix(root).expect("a path under the root");
                HostRow(vec![
                    // `/` on every platform, as Tastemaker names a placed file.
                    ryo_values::domain::row_texts::types::Cell::File(
                        relative
                            .to_string_lossy()
                            .replace(std::path::MAIN_SEPARATOR, "/")
                            .into(),
                    ),
                    text(""),
                    text(""),
                    text(template),
                    text(""),
                    text(""),
                ])
            })
            .collect(),
    }
}
