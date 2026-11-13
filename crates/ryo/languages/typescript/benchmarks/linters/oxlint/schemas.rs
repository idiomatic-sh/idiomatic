//! Private oxlint wire data, decoded without IO into canonical benchmark reports.
use super::constants;
use ryo::linter::{RelativePath, RuleId};
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{BenchmarkFinding, RunReport};
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize)]
/// The JSON document `--format json` prints: the diagnostics and what the run covered.
struct ReportSchema {
    /// Every diagnostic, in the order the run printed them.
    diagnostics: Vec<DiagnosticSchema>,
    /// How many files the run linted.
    number_of_files: usize,
    /// How many rules the run enabled.
    number_of_rules: usize,
}
#[derive(Debug, Deserialize)]
/// One oxlint diagnostic before its rule and span are normalized.
struct DiagnosticSchema {
    /// `<plugin>(<rule>)`; a parse error has none.
    code: Option<String>,
    /// The file as it was named on the command line.
    filename: String,
    /// The labelled spans, the first the diagnostic's own.
    labels: Vec<LabelSchema>,
}
#[derive(Debug, Deserialize)]
/// One labelled span of a diagnostic.
struct LabelSchema {
    /// Where the label is in the source: its UTF-8 byte `offset` and `length`, beside its `line` and `column`.
    span: BTreeMap<String, usize>,
}

/// Lint findings are valid; an operational exit is not.
pub(super) fn check_status(output: &ProcessOutput) -> Result<(), Error> {
    if matches!(output.status.0, 0_i32 | 1_i32) {
        return Ok(());
    }
    Err(Error::Invalid(format!(
        "oxlint failed ({}): {}",
        output.status.0,
        String::from_utf8_lossy(&output.stderr.0)
    )))
}

/// The rule directory id of an oxlint code: `eslint(no-debugger)` is `eslint.no-debugger`.
pub(super) fn rule_of_code(code: &str) -> Result<RuleId, Error> {
    let (plugin, rule) = code
        .strip_suffix(')')
        .and_then(|head| head.split_once('('))
        .filter(|(plugin, rule)| !plugin.is_empty() && !rule.is_empty())
        .ok_or_else(|| Error::Invalid(format!("{} {code}", constants::UNREADABLE_CODE)))?;
    RuleId::try_from(format!("{plugin}{}{rule}", constants::ID_SEPARATOR).as_str())
        .map_err(|error| Error::Invalid(error.to_string()))
}

/// The `-D` argument that selects a rule directory id: `eslint.no-debugger` is `eslint/no-debugger`.
pub(super) fn selector_of_rule(rule: &RuleId) -> Result<String, Error> {
    let (plugin, name) = rule
        .as_ref()
        .split_once(constants::ID_SEPARATOR)
        .filter(|(plugin, name)| !plugin.is_empty() && !name.is_empty())
        .ok_or_else(|| Error::Invalid(format!("{} {rule}", constants::UNREADABLE_RULE)))?;
    Ok(format!("{plugin}{}{name}", constants::PLUGIN_SEPARATOR))
}

/// Decode a run of the selected rules over the requested files into canonical findings.
pub(super) fn decode(
    // Exact requested corpus.
    request: &RunRequest,
    // Captured exit status and wire bytes.
    output: &ProcessOutput,
    // The rules the run selected; any other code is refused.
    selected: &BTreeSet<RuleId>,
) -> Result<RunReport, Error> {
    OxlintReportSchema {
        request,
        output,
        selected,
    }
    .try_into()
}

/// A wire response accompanied by the request and the selection it answers.
pub(super) struct OxlintReportSchema<'a> {
    /// Exact requested corpus.
    request: &'a RunRequest,
    /// Captured exit status and wire bytes.
    output: &'a ProcessOutput,
    /// The rules the run selected; any other code is refused.
    selected: &'a BTreeSet<RuleId>,
}
impl TryFrom<OxlintReportSchema<'_>> for RunReport {
    type Error = Error;
    fn try_from(schema: OxlintReportSchema<'_>) -> Result<Self, Error> {
        let OxlintReportSchema {
            request,
            output,
            selected,
        } = schema;
        check_status(output)?;
        let report: ReportSchema = serde_json::from_slice(&output.stdout.0)
            .map_err(|error| Error::Json(Box::new(error)))?;
        if (output.status.0 == 0_i32) != report.diagnostics.is_empty() {
            return Err(Error::Invalid(constants::CONTRADICTORY_STATUS.into()));
        }
        if report.number_of_files != request.files.len() {
            return Err(Error::Invalid(constants::FILES_NOT_LINTED.into()));
        }
        if report.number_of_rules != selected.len() {
            return Err(Error::Invalid(constants::RULES_NOT_ENABLED.into()));
        }
        let syntax = RuleId::try_from(constants::INVALID_SYNTAX)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        let mut diagnostics = Vec::new();
        for diagnostic in report.diagnostics {
            let rule = match diagnostic.code.as_deref() {
                None => syntax.clone(),
                Some(code) => {
                    let rule = rule_of_code(code)?;
                    if !selected.contains(&rule) {
                        return Err(Error::Invalid(format!(
                            "{} {code}",
                            constants::UNEXPECTED_RULE
                        )));
                    }
                    rule
                }
            };
            let file = RelativePath(diagnostic.filename.into());
            if !request.files.contains(&file) {
                return Err(Error::Invalid(constants::UNREQUESTED_FILE.into()));
            }
            let span = &diagnostic
                .labels
                .first()
                .ok_or_else(|| Error::Invalid(constants::UNLABELLED_DIAGNOSTIC.into()))?
                .span;
            let field = |name: &str| {
                span.get(name).copied().ok_or_else(|| {
                    Error::Invalid(format!("{} {name}", constants::SPAN_FIELD_MISSING))
                })
            };
            let start = field(constants::SPAN_OFFSET)?;
            let end = start
                .checked_add(field(constants::SPAN_LENGTH)?)
                .ok_or_else(|| Error::Invalid(constants::SPAN_OVERFLOW.into()))?;
            diagnostics.push(BenchmarkFinding {
                rule,
                file,
                start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(start),
                end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(end),
            });
        }
        Ok(RunReport {
            execution_metadata:
                ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Unavailable {
                    reason: ryo_benchmark_contracts::domain::reports::types::UnavailableReason(
                        constants::NO_ACCOUNTING.into(),
                    ),
                },
            project_context:
                ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
            diagnostics,
            skipped: Vec::new(),
            coverage:
                ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Unaccounted,
            files: request.files.clone(),
        })
    }
}
