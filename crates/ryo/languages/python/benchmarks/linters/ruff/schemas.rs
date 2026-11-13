//! Private Ruff wire data, decoded without IO into canonical benchmark reports.
use ryo::linter::{RelativePath, RuleId, SourceText};
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::{
    ApplicabilityPolicy, RuleLabel, RunRequest,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Deserialize)]
/// One Ruff finding before source-coordinate normalization.
struct FindingSchema {
    /// Reported rule identifier, including explicit syntax diagnostics.
    code: String,
    /// Source filename reported by the reference process.
    filename: String,
    /// Inclusive start coordinate.
    location: LocationSchema,
    /// Exclusive end coordinate.
    end_location: LocationSchema,
    /// Notebook cell index, refused without an explicit source mapping.
    cell: Option<usize>,
    /// Fixes require real source coordinates and cannot target an unavailable input.
    fix: Option<serde_json::Value>,
}
#[derive(Debug, Deserialize)]
/// One-based physical line and Unicode scalar column from Ruff JSON.
struct LocationSchema {
    /// One-based physical source line.
    row: usize,
    /// One-based Unicode scalar column.
    column: usize,
}

/// Lint findings and warnings are valid; an operational exit is not.
pub(super) fn check_status(output: &ProcessOutput) -> Result<(), Error> {
    if matches!(output.status.0, 0_i32 | 1_i32) {
        return Ok(());
    }
    Err(Error::Invalid(format!(
        "Ruff failed ({}): {}",
        output.status.0,
        String::from_utf8_lossy(&output.stderr.0)
    )))
}

/// Decode against the exact source set already read by the adapter, including clean files.
pub(super) fn decode(
    // Exact requested corpus and retained-state identity.
    request: &RunRequest,
    // Captured reference exit status and wire bytes.
    output: &ProcessOutput,
    // Explicit rule selection accepted from the reference.
    defaults: &BTreeSet<RuleId>,
    // Complete source texts against which all reported coordinates are checked.
    sources: &BTreeMap<RelativePath, SourceText>,
) -> Result<RunReport, Error> {
    RuffReportSchema {
        request,
        output,
        defaults,
        sources,
    }
    .try_into()
}

/// A wire response accompanied by the exact requested source material.
pub(super) struct RuffReportSchema<'a> {
    /// Exact requested corpus and retained-state identity.
    request: &'a RunRequest,
    /// Captured reference exit status and wire bytes.
    output: &'a ProcessOutput,
    /// Explicit rule selection accepted from the reference.
    defaults: &'a BTreeSet<RuleId>,
    /// Complete source texts against which all reported coordinates are checked.
    sources: &'a BTreeMap<RelativePath, SourceText>,
}
impl TryFrom<RuffReportSchema<'_>> for RunReport {
    type Error = Error;
    fn try_from(schema: RuffReportSchema<'_>) -> Result<Self, Error> {
        let RuffReportSchema {
            request,
            output,
            defaults,
            sources,
        } = schema;
        check_status(output)?;
        let declared_missing = |file: &RelativePath| {
            matches!(&request.context_manifest, ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(manifest) if
                ryo_benchmark::domain::suite::service::declares_missing(manifest, file) == ryo_benchmark::domain::suite::types::ContextAbsence::Sealed)
        };
        if sources.keys().any(|file| !request.files.contains(file))
            || request
                .files
                .iter()
                .any(|file| !sources.contains_key(file) && !declared_missing(file))
        {
            return Err(Error::Invalid("requested source set is incomplete".into()));
        }
        let raw: Vec<FindingSchema> = serde_json::from_slice(&output.stdout.0)
            .map_err(|error| Error::Json(Box::new(error)))?;
        if (output.status.0 == 0_i32) != raw.is_empty() {
            return Err(Error::Invalid(
                "Ruff exit status contradicts findings".into(),
            ));
        }
        let mut diagnostics = Vec::new();
        for finding in raw {
            if finding.cell.is_some() {
                return Err(Error::Invalid(
                    "notebook cell locations need a mapped source model".into(),
                ));
            }
            let rule = RuleId::try_from(finding.code.as_str())
                .map_err(|error| Error::Invalid(error.to_string()))?;
            if !defaults.contains(&rule) && finding.code != "invalid-syntax" {
                return Err(Error::Invalid(format!("unexpected rule {}", finding.code)));
            }
            let relative = dunce::simplified(Path::new(&finding.filename))
                .strip_prefix(dunce::simplified(&request.root))
                .map_err(|_outside_root| {
                    Error::Invalid("Ruff returned a path outside the corpus".into())
                })?;
            let file = relative
                .to_str()
                .ok_or_else(|| Error::Invalid("non-UTF8 Ruff filename".into()))?;
            let requested_file = RelativePath(file.into());
            if !request.files.contains(&requested_file) {
                return Err(Error::Invalid("Ruff returned an unrequested file".into()));
            }
            let (start, end) = match sources.get(&requested_file) {
                Some(source) => (
                    offset(&source.0, finding.location.row, finding.location.column)?,
                    offset(
                        &source.0,
                        finding.end_location.row,
                        finding.end_location.column,
                    )?,
                ),
                None if declared_missing(&requested_file)
                    && finding.location.row == 1
                    && finding.location.column == 1
                    && finding.end_location.row == 1
                    && finding.end_location.column == 1
                    && finding.fix.is_none() =>
                {
                    (0, 0)
                }
                None => {
                    return Err(Error::Invalid(
                        "unavailable source requires a file-origin finding without a fix".into(),
                    ));
                }
            };
            if end < start {
                return Err(Error::Invalid("Ruff span is reversed".into()));
            }
            diagnostics.push(BenchmarkFinding {
                rule,
                file: RelativePath(file.into()),
                start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(start),
                end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(end),
            });
        }
        Ok(RunReport {
            execution_metadata:
                ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Unavailable {
                    reason: ryo_benchmark_contracts::domain::reports::types::UnavailableReason(
                        "Ruff JSON findings format has no rule/file execution accounting".into(),
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

/// Translate one-based Unicode scalar coordinates into a UTF-8 byte offset.
pub(super) fn offset(source: &str, row: usize, column: usize) -> Result<usize, Error> {
    if row == 0 || column == 0 {
        return Err(Error::Invalid("Ruff location is zero-based".into()));
    }
    let bytes = source.as_bytes();
    let mut start = 0_usize;
    for _ in 1..row {
        let remaining = bytes
            .get(start..)
            .ok_or_else(|| Error::Invalid("Ruff row outside source".into()))?;
        let relative_newline = remaining
            .iter()
            .position(|byte| matches!(byte, b'\r' | b'\n'))
            .ok_or_else(|| Error::Invalid("Ruff row outside source".into()))?;
        let newline = start
            .checked_add(relative_newline)
            .ok_or_else(|| Error::Invalid("Ruff row outside source".into()))?;
        start = newline
            .checked_add(1)
            .ok_or_else(|| Error::Invalid("Ruff row outside source".into()))?;
        if bytes.get(newline) == Some(&b'\r') && bytes.get(start) == Some(&b'\n') {
            start = start
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("Ruff row outside source".into()))?;
        }
    }
    if row == 1 && source.starts_with('\u{feff}') {
        start += '\u{feff}'.len_utf8();
    }
    let tail = source
        .get(start..)
        .ok_or_else(|| Error::Invalid("Ruff row outside source".into()))?;
    let line = tail.split(['\r', '\n']).next().unwrap_or("");
    let column = column - 1;
    let relative = if column == line.chars().count() {
        line.len()
    } else {
        line.char_indices()
            .nth(column)
            .map(|(byte, _)| byte)
            .ok_or_else(|| Error::Invalid("Ruff column outside source".into()))?
    };
    start
        .checked_add(relative)
        .ok_or_else(|| Error::Invalid("Ruff column outside source".into()))
}

/// Serialize declared configuration into Ruff's documented inline TOML overrides.
/// Exact absolute patterns preserve the shared exact-file contract instead of Ruff's basename shortcut.
pub(super) fn configuration_arguments(
    configuration: &ryo_benchmark_contracts::domain::context::types::RuleConfiguration,
    root: &Path,
) -> Result<Vec<std::ffi::OsString>, Error> {
    let strings = |values: Vec<String>| {
        toml::Value::Array(values.into_iter().map(toml::Value::String).collect())
    };
    let section = |key: &str, value: toml::Value| -> Result<std::ffi::OsString, Error> {
        let lint = toml::Table::from_iter([(key.to_owned(), value)]);
        let document = toml::Table::from_iter([("lint".to_owned(), toml::Value::Table(lint))]);
        toml::to_string(&document)
            .map(Into::into)
            .map_err(|error| Error::Invalid(error.to_string()))
    };
    let mut arguments = Vec::new();
    if !configuration.external_prefixes.is_empty() {
        arguments.push("--config".into());
        arguments.push(section(
            "external",
            strings(
                configuration
                    .external_prefixes
                    .iter()
                    .map(|prefix| prefix.0.clone())
                    .collect(),
            ),
        )?);
    }
    if !configuration.disabled_rules.is_empty() {
        let mut entries = toml::Table::new();
        for (file, rules) in &configuration.disabled_rules {
            let absolute = root.join(file.0.as_ref());
            let text = absolute
                .to_str()
                .ok_or_else(|| Error::Invalid("non-UTF8 configuration path".into()))?;
            let literal = ryo_benchmark::domain::rule_context::service::literal_glob(
                &ryo::linter::RelativePath(text.into()),
            );
            let rules: Vec<_> = rules.iter().map(ToString::to_string).collect();
            entries.insert(literal.0, strings(rules));
        }
        arguments.push("--config".into());
        arguments.push(section("per-file-ignores", toml::Value::Table(entries))?);
    }
    Ok(arguments)
}

#[derive(Debug, Deserialize)]
/// The pinned registry of Ruff's rule phases, as pinned beside the profiles.
struct PinnedPhasesSchema {
    /// The pinned phase facts of each rule.
    rules: BTreeMap<String, PinnedRuleSchema>,
}
#[derive(Debug, Deserialize)]
/// The phase fact of one pinned rule that a worker request must agree with.
struct PinnedRuleSchema {
    /// Whether the rule runs on every source or only on sources the parser accepts.
    applicability: String,
}

/// Decode the pinned phase policy of every rule the registry pins.
pub(super) fn pinned_rule_phases() -> Result<BTreeMap<RuleLabel, ApplicabilityPolicy>, Error> {
    let registry: PinnedPhasesSchema = serde_json::from_str(crate::constants::PINNED_RULE_PHASES)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    Ok(registry
        .rules
        .into_iter()
        .map(|(rule, pinned)| (RuleLabel(rule), ApplicabilityPolicy(pinned.applicability)))
        .collect())
}
