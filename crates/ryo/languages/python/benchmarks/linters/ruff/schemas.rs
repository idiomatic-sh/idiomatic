//! Private Ruff wire data, decoded without IO into canonical benchmark reports.
use ryo::linter::{RelativePath, RuleId, SourceText};
use ryo_benchmark::domain::{
    diagnostics::{
        errors::DiagnosticsError as Error,
        types::{BenchmarkFinding, RunReport},
    },
    measurements::types::ProcessOutput,
    suite::types::RunRequest,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Deserialize)]
struct FindingSchema {
    code: String,
    filename: String,
    location: LocationSchema,
    end_location: LocationSchema,
    cell: Option<usize>,
}
#[derive(Debug, Deserialize)]
struct LocationSchema {
    row: usize,
    column: usize,
}

/// Lint findings and warnings are valid; an operational exit is not.
pub(super) fn check_status(output: &ProcessOutput) -> Result<(), Error> {
    if matches!(output.status.0, 0 | 1) {
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
    request: &RunRequest,
    output: &ProcessOutput,
    defaults: &BTreeSet<RuleId>,
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
    request: &'a RunRequest,
    output: &'a ProcessOutput,
    defaults: &'a BTreeSet<RuleId>,
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
        if request.files.len() != sources.len()
            || request.files.iter().any(|file| !sources.contains_key(file))
        {
            return Err(Error::Invalid("requested source set is incomplete".into()));
        }
        let raw: Vec<FindingSchema> = serde_json::from_slice(&output.stdout.0)
            .map_err(|error| Error::Json(Box::new(error)))?;
        if (output.status.0 == 0) != raw.is_empty() {
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
                .map_err(|_| Error::Invalid("Ruff returned a path outside the corpus".into()))?;
            let file = relative
                .to_str()
                .ok_or_else(|| Error::Invalid("non-UTF8 Ruff filename".into()))?;
            let source = sources
                .get(&RelativePath(file.into()))
                .ok_or_else(|| Error::Invalid("Ruff returned an unrequested file".into()))?;
            let start = offset(&source.0, finding.location.row, finding.location.column)?;
            let end = offset(
                &source.0,
                finding.end_location.row,
                finding.end_location.column,
            )?;
            if end < start {
                return Err(Error::Invalid("Ruff span is reversed".into()));
            }
            diagnostics.push(BenchmarkFinding {
                rule,
                file: RelativePath(file.into()),
                start: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(start),
                end: ryo_benchmark::domain::diagnostics::types::SourceByteOffset(end),
            });
        }
        Ok(RunReport {
            diagnostics,
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
    let mut start = 0;
    for _ in 1..row {
        let newline = bytes[start..]
            .iter()
            .position(|byte| matches!(byte, b'\r' | b'\n'))
            .ok_or_else(|| Error::Invalid("Ruff row outside source".into()))?
            + start;
        start = newline + 1;
        if bytes[newline] == b'\r' && bytes.get(start) == Some(&b'\n') {
            start += 1;
        }
    }
    if row == 1 && source.starts_with('\u{feff}') {
        start += '\u{feff}'.len_utf8();
    }
    let tail = &source[start..];
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
    Ok(start + relative)
}
