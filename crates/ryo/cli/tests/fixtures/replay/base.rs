//! What every replay test shares: the lint over a tree it writes, with the real languages, through the SDK's `Linter`, and a finding as text.
//! Each suite that lints with the real engine includes it whole. A test states its rules in the vocabulary of ryo's rule types
//! (`ryo_core`'s `Rule`, `RuleParameter` and `HostRelation`); this file hands them to the linter as the SDK states them.

#[path = "rules.rs"]
pub(crate) mod rules;

use std::collections::BTreeMap;
use std::path::Path;

use ryo_core::inbound::use_cases::lint::types::{ParameterValue, Rule, RuleParameter};
use ryo_sdk::linter as sdk;
use ryo_sdk::query::RyoQlQuery;
use ryo_values::domain::plan_parts::types::{FlagState, LiteralText, ParameterName, WholeNumber};

/// What a parameter of a rule is bound to, when ryo can bind it.
pub(crate) fn parameter(name: &str, value: &toml::Value) -> Option<RuleParameter> {
    let bound = match value {
        toml::Value::Integer(number) => ParameterValue::Number(WholeNumber(*number)),
        toml::Value::Boolean(flag) => ParameterValue::Flag(if *flag {
            FlagState::Raised
        } else {
            FlagState::Lowered
        }),
        toml::Value::String(each) => ParameterValue::Text(LiteralText(each.as_str().into())),
        toml::Value::Array(items) if items.iter().all(toml::Value::is_str) => {
            ParameterValue::Texts(
                items
                    .iter()
                    .filter_map(|each| each.as_str().map(|text| LiteralText(text.into())))
                    .collect(),
            )
        }
        toml::Value::Array(items) if items.iter().all(toml::Value::is_integer) => {
            ParameterValue::Numbers(
                items
                    .iter()
                    .filter_map(toml::Value::as_integer)
                    .map(WholeNumber)
                    .collect(),
            )
        }
        // A list of sets of text (`list<set<string>>`), each set an array of text.
        toml::Value::Array(items)
            if items.iter().all(|set| {
                set.as_array()
                    .is_some_and(|texts| texts.iter().all(toml::Value::is_str))
            }) =>
        {
            ParameterValue::TextSets(
                items
                    .iter()
                    .filter_map(toml::Value::as_array)
                    .map(|set| {
                        set.iter()
                            .filter_map(|each| each.as_str().map(|text| LiteralText(text.into())))
                            .collect()
                    })
                    .collect(),
            )
        }
        toml::Value::Float(_)
        | toml::Value::Datetime(_)
        | toml::Value::Array(_)
        | toml::Value::Table(_) => return None,
    };
    Some(RuleParameter {
        name: ParameterName(name.into()),
        value: bound,
    })
}

/// The languages the replay links: the ones `ryo lint` links in this build (`ryo_cli::linked_sdk`), so a rule answers what a user's run
/// answers.
pub(crate) fn replay_languages() -> sdk::Languages {
    ryo_cli::linked_sdk()
}

/// The directory a run keeps what it leaves for the next, beneath `root`: where `ryo` keeps it, for the build the tests are.
pub(crate) fn store_at(root: &Path) -> sdk::Cache {
    let edge = ryo_cli::resources::edge_at(
        ryo_language::inbound::use_cases::project::types::RootPath(root.to_string_lossy().into()),
        ryo_language::inbound::use_cases::project::types::RootPath(".".into()),
        ryo_values::domain::runs::types::MachineWorkers(1),
        ryo_cli::output::Format::Quiet,
    );
    sdk::Cache::At(root.join(&*edge.store.0))
}

/// A rule as the linter states it: its id, and the rule itself.
///
/// # Panics
///
/// On a rule the SDK cannot state: an id that is blank, a query that is not text, a glob that is not one.
fn stated(rule: &Rule) -> (sdk::RuleId, sdk::Rule) {
    let glob = |each: &ryo_values::domain::files::types::Glob| {
        sdk::Glob::try_from(&*each.0).expect("a glob the rule's own include or exclude is")
    };
    let parameter_of = |each: &RuleParameter| {
        let name = sdk::ParameterName(each.name.0.clone());
        let text = |value: &LiteralText| sdk::ParameterText(value.0.clone());
        let value = match &each.value {
            ParameterValue::Number(number) => sdk::ParameterValue::Number(sdk::ParameterNumber(number.0)),
            ParameterValue::Flag(FlagState::Raised) => sdk::ParameterValue::Flag(sdk::FlagState::On),
            ParameterValue::Flag(FlagState::Lowered) => sdk::ParameterValue::Flag(sdk::FlagState::Off),
            ParameterValue::Text(value) => sdk::ParameterValue::Text(text(value)),
            ParameterValue::Numbers(numbers) => sdk::ParameterValue::Numbers(sdk::ParameterNumbers(
                numbers.iter().map(|number| number.0).collect(),
            )),
            ParameterValue::Texts(values) => sdk::ParameterValue::Texts(values.iter().map(text).collect()),
            ParameterValue::TextSets(sets) => sdk::ParameterValue::TextSets(
                sets.iter().map(|set| set.iter().map(text).collect()).collect(),
            ),
            other => panic!("a parameter the replay does not state: {other:?}"),
        };
        (name, value)
    };
    assert!(rule.observations.is_empty(), "the replay states no observation");
    (
        sdk::RuleId::try_from(&*rule.id.0).expect("a rule id that is not blank"),
        sdk::Rule {
            observations: Vec::new(),
            query: RyoQlQuery::try_from(&*rule.query.root.text).expect("a query"),
            applicability: match rule.applicability {
                ryo_values::domain::runs::types::RuleApplicability::Always => sdk::RuleApplicability::Always,
                _ => sdk::RuleApplicability::AcceptedSyntax,
            },
            parameters: rule.parameters.iter().map(parameter_of).collect(),
            include: rule.include.iter().map(glob).collect(),
            exclude: rule.exclude.iter().map(glob).collect(),
        },
    )
}

/// A relation the host declares, as the linter states it.
fn hosted_as_stated(relation: &ryo_values::domain::hosted::types::HostRelation) -> sdk::HostRelation {
    use ryo_values::domain::hosted::types::HostKind;
    use ryo_values::domain::row_texts::types::Cell;
    sdk::HostRelation {
        name: sdk::HostRelationName(relation.name.0.clone()),
        columns: relation
            .columns
            .iter()
            .map(|column| sdk::HostColumn {
                name: sdk::HostColumnName(column.name.0.clone()),
                kind: match column.kind {
                    HostKind::File => sdk::HostKind::File,
                    _ => sdk::HostKind::Text,
                },
            })
            .collect(),
        rows: relation
            .rows
            .iter()
            .map(|row| {
                sdk::HostRow(
                    row.0
                        .iter()
                        .map(|cell| match cell {
                            Cell::File(path) => sdk::Cell::File(sdk::RelativePath(path.clone())),
                            Cell::Text(text) => sdk::Cell::Text(sdk::HostText(text.clone())),
                            Cell::Number(number) => sdk::Cell::Number(sdk::HostNumber(number.0)),
                            Cell::Span(span) => sdk::Cell::Span(*span),
                        })
                        .collect(),
                )
            })
            .collect(),
    }
}

/// Lints `root` with `rules`, the facts kept in `cache`: the rows the rules gave and the files nothing read.
///
/// # Errors
///
/// What the lint refuses, as text.
pub(crate) fn lint_with(
    root: &Path,
    rules: Vec<Rule>,
    cache: sdk::Cache,
) -> Result<(Vec<Vec<String>>, Vec<String>), String> {
    let (rows, report) = lint_report(root, rules, cache)?;
    Ok((
        rows,
        report
            .unread
            .into_iter()
            .map(|file| file.path.0.to_string())
            .collect(),
    ))
}

/// Lints `root` with `rules`, the facts kept in `cache`: the rows the rules gave and the report of the lint.
///
/// # Errors
///
/// What the lint refuses, as text.
pub(crate) fn lint_report(
    root: &Path,
    rules: Vec<Rule>,
    cache: sdk::Cache,
) -> Result<(Vec<Vec<String>>, sdk::LintReport), String> {
    // The command line links the TOML language beside the Rust language; a rule that reads a manifest's tree needs it.
    lint_report_hosted(root, rules, cache, Vec::new(), true)
}

/// Lints `root` with `rules`, the facts kept in `cache`, over the relations a host declares beside ryo's own: the rows the rules gave and
/// the report of the lint.
///
/// # Errors
///
/// What the lint refuses, as text.
pub(crate) fn lint_report_hosted(
    root: &Path,
    rules: Vec<Rule>,
    cache: sdk::Cache,
    hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
    manifests_read: bool,
) -> Result<(Vec<Vec<String>>, sdk::LintReport), String> {
    lint_report_with_languages(root, rules, cache, hosted, manifests_read, Vec::new())
}

/// The same production composition with additional languages explicitly selected by a scene.
///
/// # Errors
///
/// What the lint refuses, as text.
pub(crate) fn lint_report_with_languages(
    root: &Path,
    rules: Vec<Rule>,
    cache: sdk::Cache,
    hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
    manifests_read: bool,
    additional: Vec<ryo_sdk::languages::Language>,
) -> Result<(Vec<Vec<String>>, sdk::LintReport), String> {
    // Rules that locate a finding in a manifest read its tree, which only the TOML language gives: those runs link the
    // replay's languages; the others the Rust language alone. A scene's own languages come after.
    let mut languages = if manifests_read {
        replay_languages().0
    } else {
        vec![ryo_rust::linked()]
    };
    languages.extend(additional);
    let linter = sdk::Linter::new(
        sdk::Languages(languages),
        sdk::WorkerPool::from(sdk::Workers(
            std::thread::available_parallelism().unwrap_or(std::num::NonZeroUsize::MIN),
        )),
        cache,
    );
    let request = sdk::LintRequest {
        source_language: sdk::SourceLanguage::Recognized,
        root: root.to_path_buf(),
        project_search_scope: Default::default(),
        given_project_inventory: Default::default(),
        rules: rules.iter().map(stated).collect::<BTreeMap<_, _>>(),
        files: Vec::new(),
        hosted: hosted.iter().map(hosted_as_stated).collect(),
        corpus: sdk::Corpus::Walked,
        beside: Vec::new(),
    };
    let report = linter
        .lint(vec![request])
        .pop()
        .ok_or_else(|| "the linter answered no request".to_owned())?
        .map_err(|error| format!("the rules do not run: {error}"))?;
    let rows = report.findings.iter().map(finding_texts).collect();
    Ok((rows, report))
}

/// How a cell a row does not have reads, apart from a cell of empty text.
pub(crate) const ABSENT: &str = "(absent)";

/// A finding as text: its rule, its subject, its key values joined with commas, its line and its character, each value as it reads (a
/// span as its two ends); its columns left out.
pub(crate) fn finding_texts(finding: &sdk::Finding) -> Vec<String> {
    let text = |slot: &sdk::Slot| match slot {
        sdk::Slot::Is(sdk::Cell::File(each)) => each.0.to_string(),
        sdk::Slot::Is(sdk::Cell::Text(each)) => each.0.to_string(),
        sdk::Slot::Is(sdk::Cell::Number(number)) => number.0.to_string(),
        sdk::Slot::Is(sdk::Cell::Span(span))
            if ryo_relations::domain::relations::service::location(*span)
                == ryo_relations::domain::relations::types::Locatedness::Absent =>
        {
            ryo_relations::domain::relations::constants::UNLOCATED.to_owned()
        }
        sdk::Slot::Is(sdk::Cell::Span(span)) => format!("{}..{}", span.start.0, span.end.0),
        _ => ABSENT.to_owned(),
    };
    let (line, column) = match finding.place {
        sdk::Place::At { line, column } => (line.0.to_string(), column.0.to_string()),
        _ => (ABSENT.to_owned(), ABSENT.to_owned()),
    };
    vec![
        finding.rule.to_string(),
        text(&finding.subject),
        finding.key.0.iter().map(text).collect::<Vec<_>>().join(","),
        line,
        column,
    ]
}
