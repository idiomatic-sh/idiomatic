//! A registered rule's source identity, with the effective settings its staged profile runs with.
use super::super::authored::schemas::document;
use super::super::case::schemas::Canonical;
use super::super::constants;
use crate::outbound::inventory::types::RuleInstanceRecord;
use ryo_benchmark_contracts::domain::documents::types::text::value::CanonicalJson;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::ParserSelection;
use std::collections::BTreeMap;

/// A refusal of the staged profile.
fn invalid(text: &str) -> Error {
    Error::Invalid(text.to_owned())
}

/// A text value.
fn text(value: &str) -> Canonical {
    Canonical::Text(value.to_owned())
}

/// The texts of a list value.
fn texts(value: Option<&Canonical>) -> Vec<String> {
    match value {
        Some(Canonical::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                Canonical::Text(text) => Some(text.clone()),
                Canonical::Null
                | Canonical::Flag(_)
                | Canonical::Integer(_)
                | Canonical::Float(_)
                | Canonical::List(_)
                | Canonical::Table(_) => None,
            })
            .collect(),
        Some(
            Canonical::Null
            | Canonical::Flag(_)
            | Canonical::Integer(_)
            | Canonical::Float(_)
            | Canonical::Text(_)
            | Canonical::Table(_),
        )
        | None => Vec::new(),
    }
}

/// The table of a value, empty when it is none.
fn entries(value: Option<&Canonical>) -> BTreeMap<String, Canonical> {
    match value {
        Some(Canonical::Table(table)) => table.clone(),
        Some(
            Canonical::Null
            | Canonical::Flag(_)
            | Canonical::Integer(_)
            | Canonical::Float(_)
            | Canonical::Text(_)
            | Canonical::List(_),
        )
        | None => BTreeMap::new(),
    }
}

/// The profile's observation declarations reachable from the selected rules, producers narrowed to the selection
/// when the profile's policy is `selected`.
fn projected_observations(profile: &BTreeMap<String, Canonical>, selected: &[String]) -> BTreeMap<String, Canonical> {
    let narrowed = profile.get(constants::RULE_OBSERVATION_SELECTION) == Some(&text(constants::SELECTED));
    let declarations = entries(profile.get(constants::RULE_OBSERVATIONS_FIELD));
    let mut pending = selected.iter().filter(|rule| declarations.contains_key(*rule)).cloned().collect::<Vec<_>>();
    let mut resolved = BTreeMap::new();
    while let Some(rule) = pending.pop() {
        let Some(Canonical::List(bindings)) = declarations.get(&rule).filter(|_| !resolved.contains_key(&rule)) else {
            continue;
        };
        let mut bindings = bindings.clone();
        for binding in &mut bindings {
            let Canonical::Table(fields) = binding else {
                continue;
            };
            let producers = texts_of_producers(fields.get(constants::PRODUCERS));
            if narrowed {
                let kept = producers
                    .iter()
                    .filter(|(producer_rule, _kind, _value)| selected.contains(producer_rule))
                    .map(|(_rule, _kind, value)| value.clone())
                    .collect();
                let _previous = fields.insert(constants::PRODUCERS.to_owned(), Canonical::List(kept));
            }
            for (producer_rule, kind, _value) in texts_of_producers(fields.get(constants::PRODUCERS)) {
                if kind == constants::OUTPUT && declarations.contains_key(&producer_rule) {
                    pending.push(producer_rule);
                }
            }
        }
        let _previous = resolved.insert(rule, Canonical::List(bindings));
    }
    resolved
}

/// Each producer of a binding: its rule, its kind and itself.
fn texts_of_producers(value: Option<&Canonical>) -> Vec<(String, String, Canonical)> {
    match value {
        Some(Canonical::List(items)) => items
            .iter()
            .map(|item| {
                let fields = entries(Some(item));
                let field = |key: &str| fields.get(key).and_then(Canonical::as_text).unwrap_or_default().to_owned();
                (field(constants::RULE), field(constants::KIND), item.clone())
            })
            .collect(),
        Some(
            Canonical::Null
            | Canonical::Flag(_)
            | Canonical::Integer(_)
            | Canonical::Float(_)
            | Canonical::Text(_)
            | Canonical::Table(_),
        )
        | None => Vec::new(),
    }
}

/// The settings the staged profile runs its rule with, as the authoring's canonical execution settings name them.
fn effective_settings(profile: &BTreeMap<String, Canonical>, rule: &str) -> Result<Canonical, Error> {
    if profile.keys().any(|key| !constants::REVIEWED_KEYS.contains(&key.as_str())) {
        return Err(invalid(constants::UNREVIEWED_SETTINGS));
    }
    let declared = entries(profile.get(constants::APPLICABILITY_KEY));
    let mut applicability = BTreeMap::new();
    if !declared.is_empty() {
        for selected in std::iter::once(rule.to_owned()).chain(texts(profile.get(constants::SUPPORTING_KEY))) {
            let value = declared.get(&selected).cloned().ok_or_else(|| invalid(constants::NO_APPLICABILITY))?;
            let _previous = applicability.insert(selected, value);
        }
    }
    let mode = profile.get(constants::PROJECT_CONTEXT_MODE).cloned().unwrap_or_else(|| text(constants::ISOLATED));
    if mode != text(constants::ISOLATED) && mode != text(constants::LISTED_DISK) {
        return Err(invalid(constants::CONTEXT_MODE));
    }
    let language = profile.get(constants::SOURCE_LANGUAGE).cloned();
    if language.as_ref().is_some_and(|language| !matches!(language, Canonical::Text(name) if !name.is_empty() && name.trim() == name)) {
        return Err(invalid(constants::SOURCE_LANGUAGE_NAME));
    }
    let observation = profile.get(constants::FILE_OBSERVATION).cloned();
    if observation.as_ref().is_some_and(|observation| observation != &text(constants::POSIX_EXECUTABLE)) {
        return Err(invalid(constants::FILE_OBSERVATION_REQUIREMENT));
    }
    let observations = projected_observations(profile, &[rule.to_owned()]);
    let settings = EffectiveSettings {
        source_language: language,
        performance: Canonical::Table(BTreeMap::from([(
            constants::DIAGNOSTICS_KEY.to_owned(),
            Canonical::Table(constants::DIAGNOSTICS_OFF.iter().map(|name| ((*name).to_owned(), Canonical::Flag(false))).collect()),
        )])),
        rule_context_declaration: profile.get(constants::RULE_CONTEXT).cloned(),
        rule_observations: (!observations.is_empty()).then_some(Canonical::Table(observations)),
        project_search_roots: profile.get(constants::PROJECT_SEARCH_ROOTS).cloned(),
        file_observation: observation,
        defaults: vec![rule.to_owned()],
        parser_configuration: profile.get(constants::PARSER_CONFIGURATION).cloned(),
        project_context_mode: mode,
        rule_applicability: Canonical::Table(applicability),
        flags: Canonical::Table(BTreeMap::new()),
    };
    serde_json::to_value(settings)
        .and_then(serde_json::from_value)
        .map_err(|error| Error::Json(Box::new(error)))
}

/// The settings a registered rule runs with, as the authoring's canonical execution settings name them.
#[derive(serde::Serialize)]
struct EffectiveSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    source_language: Option<Canonical>,
    performance: Canonical,
    #[serde(skip_serializing_if = "Option::is_none")]
    rule_context_declaration: Option<Canonical>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rule_observations: Option<Canonical>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_search_roots: Option<Canonical>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_observation: Option<Canonical>,
    defaults: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parser_configuration: Option<Canonical>,
    project_context_mode: Canonical,
    rule_applicability: Canonical,
    flags: Canonical,
}

/// The registration's `instance` of one rule.
pub(in super::super) fn instance(record: &RuleInstanceRecord) -> Result<CanonicalJson, Error> {
    let profile = document(&record.stage.join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST))?;
    let mut fields = BTreeMap::from([
        (constants::QUERY_FILE, text(&record.query_file.0)),
        (constants::QUERY_SHA256, text(&record.query_sha256.0)),
        (constants::CASES_FILE_KEY, text(&record.cases_file.0)),
        (constants::CASES_SHA256, text(&record.cases_sha256.0)),
        (constants::CASE_COUNT, Canonical::Integer(i64::try_from(record.case_count.0).map_err(|_| invalid(constants::CASE_IDENTITIES))?)),
        (constants::SETTINGS_SOURCE, text(&record.settings_source.0)),
        (constants::SETTINGS_SOURCE_SHA256, text(&record.settings_source_sha256.0)),
        (constants::EVIDENCE_SOURCE_ROOT, text(&record.evidence_source_root.0)),
        (constants::PROFILE_SHA256_KEY, text(&record.profile_sha256.0)),
        (
            constants::MODULE_SHA256,
            Canonical::Table(record.module_sha256.iter().map(|(name, digest)| (name.0.clone(), text(&digest.0))).collect()),
        ),
        (constants::EFFECTIVE_SETTINGS, effective_settings(&profile, &record.rule.0)?),
    ]);
    match &record.parser_configuration {
        ParserSelection::Named(name) => {
            let _previous = fields.insert(constants::PARSER_CONFIGURATION_KEY, text(&name.0));
        }
        ParserSelection::Unspecified => {}
    }
    Canonical::Table(fields.into_iter().map(|(key, value)| (key.to_owned(), value)).collect())
        .text()
        .map(CanonicalJson)
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
