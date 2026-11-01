//! Rules whose queries need what today's ryo lacks run here, over their own Tastemaker scenarios: the
//! query of `rule.toml`, the files of each scenario's `given.files`, and the findings its `then.findings`
//! states, written by hand, never from this run.

use crate::{base, tables};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::types::{ParameterValue, RuleParameter};
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::plan_parts::types::{FlagState, LiteralText, ParameterName, WholeNumber};

/// The rules a spec of the arch package holds, by the directory under the package.
const SPEC_RULES: [(&str, &str); 5] = [
    (
        "captured-workload",
        "specs/crate/0.13.0/rules/captured-workload",
    ),
    ("bdd-runner", "specs/crate/0.13.0/rules/bdd-runner"),
    (
        "pin-revision",
        "specs/distribution/0.3.0/rules/pin-revision",
    ),
    (
        "no-hardcoded-paths",
        "specs/idiomatic-rust/0.1.0/rules/no-hardcoded-paths",
    ),
    (
        "variants-benched",
        "specs/idiomatic-rust/0.1.0/rules/variants-benched",
    ),
];

fn rule_directory(rule: &str) -> PathBuf {
    if let Some(spec) = SPEC_RULES.iter().find(|(name, _)| *name == rule) {
        return Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tastemaker")
            .join(spec.1);
    }
    let held = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/rules")
        .join(rule);
    if held.is_dir() {
        return held;
    }
    let package = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tastemaker");
    let shared = package.join("rules").join(rule);
    // A benchmark-only rule is the benchmarks package's, under the arch package's directory.
    let rules = if shared.is_dir() {
        shared
    } else {
        package.join("benchmarks/rules").join(rule)
    };
    let mut versions: Vec<PathBuf> = std::fs::read_dir(&rules)
        .expect("the rule's versions")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    versions.sort();
    versions.pop().expect("a version of the rule")
}

/// A rule Tastemaker retired keeps its query and scenarios under `tests/fixtures/rules`, as the proof of what the engine decides
/// that today's ryo could not; any other rule is read from the package. Every scenario of the rule gives the findings its
/// `then.findings` state, by the path of the file.
fn check(rule: &str) {
    let directory = rule_directory(rule);
    let manifest = std::fs::read_to_string(directory.join("rule.toml")).expect("the rule");
    let query = tables::string_under(&manifest, &["query"]).expect("a query");
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
        let mut each = crate::rule_from_query(rule, &query, &include);
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
            Arc::new(ryo_core::outbound::facts::factory::memory()),
            Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
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

/// The values a scenario binds the rule's params to: a list of text, or a list of sets of text.
fn parameters_of(given: Option<&toml::Table>) -> Vec<RuleParameter> {
    let Some(params) = given
        .and_then(|given| given.get("params"))
        .and_then(toml::Value::as_table)
    else {
        return Vec::new();
    };
    let words = |value: &toml::Value| -> Vec<LiteralText> {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|each| each.as_str().map(|text| LiteralText(text.into())))
            .collect()
    };
    params
        .iter()
        .map(|(name, value)| {
            let sets = value
                .as_array()
                .is_some_and(|items| items.iter().all(toml::Value::is_array));
            RuleParameter {
                name: ParameterName(name.as_str().into()),
                value: if let Some(number) = value.as_integer() {
                    ParameterValue::Number(WholeNumber(number))
                } else if let Some(flag) = value.as_bool() {
                    ParameterValue::Flag(if flag {
                        FlagState::Raised
                    } else {
                        FlagState::Lowered
                    })
                } else if let Some(text) = value.as_str() {
                    ParameterValue::Text(LiteralText(text.into()))
                } else if sets {
                    ParameterValue::TextSets(
                        value.as_array().into_iter().flatten().map(words).collect(),
                    )
                } else {
                    ParameterValue::Texts(words(value))
                },
            }
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
    let text = |value: &str| ryo_values::domain::records::types::Cell::Text(value.into());
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
                    ryo_values::domain::records::types::Cell::File(
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

#[test]
fn trait_with_one_impl_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("trait-with-one-impl");
}

#[test]
fn workspace_membership_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("workspace-membership");
}

#[test]
fn layer_order_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("layer-order");
}

#[test]
fn module_declares_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("module-declares");
}

#[test]
fn variants_benched_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("variants-benched");
}

#[test]
fn captured_workload_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("captured-workload");
}

#[test]
fn bdd_runner_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("bdd-runner");
}

#[test]
fn case_vocabulary_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("case-vocabulary");
}

#[test]
fn crate_dependency_depth_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("crate-dependency-depth");
}

#[test]
fn distribution_wiring_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("distribution-wiring");
}

#[test]
fn orphan_rust_files_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("orphan-rust-files");
}

#[test]
fn scattered_dispatch_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("scattered-dispatch");
}

#[test]
fn workspace_completeness_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("workspace-completeness");
}

#[test]
fn pin_revision_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("pin-revision");
}

#[test]
fn no_hardcoded_paths_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    check("no-hardcoded-paths");
}
