//! The clippy process protocol: the package manager's JSON message stream, and the dependency files that name what rustc read.
use super::constants;
use ryo::linter::{RelativePath, RuleId};
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{BenchmarkFinding, RunReport};
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Serialize)]
/// The manifest of a case's crate: one package with no dependency, its own workspace.
struct CaseManifestSchema {
    /// The case's package.
    package: CasePackageSchema,
    /// An empty workspace table, so no enclosing workspace claims the case.
    workspace: CaseWorkspaceSchema,
}

#[derive(Debug, Serialize)]
/// The package table of a case's manifest.
struct CasePackageSchema {
    /// The package name.
    name: &'static str,
    /// The package version.
    version: &'static str,
    /// The edition the case is read in.
    edition: &'static str,
}

#[derive(Debug, Serialize)]
/// The empty workspace table of a case's manifest.
struct CaseWorkspaceSchema {}

#[derive(Debug, Deserialize)]
/// One line of the message stream; only compiler messages carry findings.
struct LineSchema {
    /// `compiler-message`, `compiler-artifact`, `build-finished` and the like.
    reason: String,
    /// The compiler's diagnostic, on a `compiler-message` line.
    message: Option<MessageSchema>,
}

#[derive(Debug, Deserialize)]
/// One rustc diagnostic as the JSON error format writes it.
struct MessageSchema {
    /// `clippy::<lint>` for a clippy finding, `E0xxx` or a rustc lint otherwise, absent for a parse error.
    code: Option<CodeSchema>,
    /// `error`, `warning` and the like.
    level: String,
    /// What the diagnostic says, quoted by a refusal.
    message: String,
    /// The places the diagnostic names; the first primary one is the finding's.
    spans: Vec<SpanSchema>,
}

#[derive(Debug, Deserialize)]
/// The code of a diagnostic.
struct CodeSchema {
    /// The lint or error code.
    code: String,
}

#[derive(Debug, Deserialize)]
/// A place in a source file, in UTF-8 bytes.
struct SpanSchema {
    /// The file, relative to the workspace root.
    file_name: String,
    /// First byte.
    byte_start: usize,
    /// Byte after the last.
    byte_end: usize,
    /// Whether this is the place the diagnostic is about.
    is_primary: bool,
}

/// A run that finished, with findings or with the parse error that stopped the build, is valid; an operational exit is not.
pub(super) fn check_status(output: &ProcessOutput) -> Result<(), Error> {
    if matches!(output.status.0, 0_i32 | constants::BUILD_FAILED_STATUS) {
        return Ok(());
    }
    Err(Error::Invalid(format!(
        "clippy failed ({}): {}",
        output.status.0,
        String::from_utf8_lossy(&output.stderr.0)
    )))
}

/// The manifest text of a case's crate.
pub(super) fn case_manifest() -> Result<String, Error> {
    toml::to_string(&CaseManifestSchema {
        package: CasePackageSchema {
            name: constants::CASE_PACKAGE,
            version: constants::CASE_VERSION,
            edition: constants::CASE_EDITION,
        },
        workspace: CaseWorkspaceSchema {},
    })
    .map_err(|error| Error::Invalid(error.to_string()))
}

/// The rule directory id of a clippy code: `clippy::needless_return` is `needless_return`.
pub(super) fn rule_of_code(code: &str) -> Result<RuleId, Error> {
    let lint = code
        .strip_prefix(constants::LINT_PREFIX)
        .filter(|lint| !lint.is_empty())
        .ok_or_else(|| Error::Invalid(format!("{} {code}", constants::UNREADABLE_CODE)))?;
    RuleId::try_from(lint).map_err(|error| Error::Invalid(error.to_string()))
}

/// The `-W` argument that selects a rule directory id: `needless_return` is `clippy::needless_return`.
pub(super) fn selector_of_rule(rule: &RuleId) -> Result<String, Error> {
    Ok(format!("{}{rule}", constants::LINT_PREFIX))
}

/// The sources under `root` one dependency file names, relative to it: every `.rs` prerequisite of its first rule, a path written
/// relative to the workspace root or absolute, a space inside it escaped as `\ `.
pub(super) fn dep_info_sources(text: &str, root: &Path) -> Result<BTreeSet<RelativePath>, Error> {
    let mut sources = BTreeSet::new();
    let Some((_, prerequisites)) = text
        .lines()
        .next()
        .and_then(|line| line.split_once(constants::RULE_SEPARATOR))
    else {
        return Err(Error::Invalid(constants::UNREADABLE_DEP_INFO.into()));
    };
    for prerequisite in prerequisites
        .replace(constants::ESCAPED_SPACE, constants::SPACE_PLACEHOLDER)
        .split_whitespace()
    {
        // rustc names an `include!`d file as written beside its includer (`src/../examples/demo.rs`): the path is resolved
        // lexically, so it is the one the project tracks, and one that climbs above the root is not under it.
        let mut path = PathBuf::new();
        for component in root
            .join(prerequisite.replace(constants::SPACE_PLACEHOLDER, constants::SPACE))
            .components()
        {
            match component {
                Component::ParentDir => {
                    let _parent = path.pop();
                }
                Component::CurDir => {}
                Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                    path.push(component);
                }
            }
        }
        if let Ok(relative) = path.strip_prefix(root)
            && path
                .extension()
                .is_some_and(|extension| extension == constants::SOURCE_EXTENSION)
        {
            let _new = sources.insert(RelativePath(relative.to_string_lossy().into()));
        }
    }
    Ok(sources)
}

/// Decode a run of the selected rules over the requested files into canonical findings.
pub(super) fn decode(
    // Exact requested corpus.
    request: &RunRequest,
    // Captured exit status and wire bytes.
    output: &ProcessOutput,
    // The rules the run selected; a clippy code outside them is skipped.
    selected: &BTreeSet<RuleId>,
) -> Result<RunReport, Error> {
    ClippyReportSchema {
        request,
        output,
        selected,
    }
    .try_into()
}

/// A wire response accompanied by the request and the selection it answers.
pub(super) struct ClippyReportSchema<'a> {
    /// Exact requested corpus.
    request: &'a RunRequest,
    /// Captured exit status and wire bytes.
    output: &'a ProcessOutput,
    /// The rules the run selected; a clippy code outside them is skipped.
    selected: &'a BTreeSet<RuleId>,
}
impl TryFrom<ClippyReportSchema<'_>> for RunReport {
    type Error = Error;
    fn try_from(schema: ClippyReportSchema<'_>) -> Result<Self, Error> {
        let ClippyReportSchema {
            request,
            output,
            selected,
        } = schema;
        check_status(output)?;
        let syntax = RuleId::try_from(constants::INVALID_SYNTAX)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        let mut findings = BTreeSet::new();
        let mut stopped = false;
        for line in output
            .stdout
            .0
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
        {
            let line: LineSchema =
                serde_json::from_slice(line).map_err(|error| Error::Json(Box::new(error)))?;
            let Some(message) = line
                .message
                .filter(|_| line.reason == constants::COMPILER_MESSAGE)
            else {
                continue;
            };
            let code = message.code.as_ref().map(|code| code.code.as_str());
            let stops = message.level == constants::ERROR_LEVEL;
            let primary = message.spans.iter().find(|span| span.is_primary);
            let rule = match code {
                Some(constants::UNKNOWN_SELECTED_LINT | constants::UNKNOWN_ATTRIBUTE_LINT) => {
                    return Err(Error::Invalid(format!(
                        "{}: {}",
                        constants::UNKNOWN_LINT,
                        message.message
                    )));
                }
                Some(code) if code.starts_with(constants::LINT_PREFIX) => {
                    stopped |= stops;
                    let rule = rule_of_code(code)?;
                    if !selected.contains(&rule) {
                        continue;
                    }
                    rule
                }
                Some(_) if stops => {
                    return Err(Error::Invalid(format!(
                        "{}: {}",
                        constants::DOES_NOT_COMPILE,
                        message.message
                    )));
                }
                None if stops && primary.is_some() => {
                    stopped = true;
                    syntax.clone()
                }
                Some(_) | None => continue,
            };
            let span =
                primary.ok_or_else(|| Error::Invalid(constants::UNLABELLED_DIAGNOSTIC.into()))?;
            let file = RelativePath(span.file_name.as_str().into());
            if !request.files.contains(&file) {
                return Err(Error::Invalid(format!(
                    "{}: {}",
                    constants::UNREQUESTED_FILE,
                    span.file_name
                )));
            }
            let _new = findings.insert((rule, file, span.byte_start, span.byte_end));
        }
        if output.status.0 == constants::BUILD_FAILED_STATUS && !stopped {
            return Err(Error::Invalid(constants::FAILED_WITHOUT_ERROR.into()));
        }
        let diagnostics = findings
            .into_iter()
            .map(|(rule, file, start, end)| BenchmarkFinding {
                rule,
                file,
                start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(start),
                end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(end),
            })
            .collect();
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
