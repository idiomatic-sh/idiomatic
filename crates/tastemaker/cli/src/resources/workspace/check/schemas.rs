// ignore: idiomatic.arch/file-length - the command's wire shapes of one report: its arguments into the request, and the report out as text, JSON and a SARIF 2.1.0 log, each converted from the same findings with the same suppressions and fingerprints; the command spec places one schemas.rs for a command, the one file its wire shapes are converted in.
//! The shape of the data this command does not control: its arguments become the service's request, and the service's
//! report becomes the answer printed, as text, JSON, or a SARIF 2.1.0 log that reads as the first Tastemaker's did: every
//! finding an error, and the same in-source suppressions. The log carries no `partialFingerprints`: GitHub's
//! `primaryLocationLineHash` is its rolling hash of the line, which its upload and asymptote compute from the file, so
//! the field never holds a second algorithm. Each refusal of a conversion is a `<Name>SchemaError`.

use std::collections::BTreeMap;

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde_sarif::sarif::{
    ArtifactLocation, Invocation, Location, Message, PhysicalLocation, Region, ReportingDescriptor,
    Result as SarifResult, ResultLevel, Run, Sarif, Suppression as SarifSuppression,
    SupressionKind, SupressionStatus, Tool, ToolComponent, Version,
};
use tastemaker_core::domain::findings::types::{Finding, Help, Place, Report, Suppression};
use tastemaker_core::domain::paths::types::RepoPath;
use tastemaker_core::inbound::use_cases::check::errors::CheckError;
use tastemaker_core::inbound::use_cases::check::types::{CheckReport, CheckRequest, Fixing};

use tastemaker_core::domain::proofs::types::Proof;
use tastemaker_core::inbound::use_cases::validate::types::{ValidateReport, ValidateRequest};

use crate::outcome::Outcome;
use crate::output::Format;

/// What a SARIF path carries as written: letters, digits and `/-._~`.
const PATH: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'/')
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');
/// The base every result's path is relative to.
const SOURCE_ROOT: &str = "SOURCE_ROOT";
/// The schema a SARIF log names.
const SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";
/// The tool a SARIF log names.
const TOOL: &str = tastemaker_core::domain::findings::constants::TASTEMAKER;
/// Where the tool is documented: the public repository.
const INFORMATION: &str = "https://github.com/idiomatic-sh/idiomatic";
/// The tool's version, the command line crate's.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Why the arguments were refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum CheckSchemaError {
    /// A path is not text.
    #[error("the path {0:?} is not text")]
    PathNotText(std::path::PathBuf),
}

/// The arguments as the command line gives them, offered to the conversion.
struct CheckArgumentsSchema<'a> {
    /// Report only the findings at or beneath these paths, relative to the current directory.
    paths: Vec<std::path::PathBuf>,
    /// Rewrite a stale or missing `tastemaker.lock`, then check.
    fix: bool,
    /// Where the command runs, and how.
    edge: &'a crate::edge::Edge,
}

impl TryFrom<CheckArgumentsSchema<'_>> for CheckRequest {
    type Error = CheckSchemaError;

    fn try_from(schema: CheckArgumentsSchema<'_>) -> Result<Self, Self::Error> {
        let edge = schema.edge;
        let mut within = Vec::new();
        for path in schema.paths {
            let text = path
                .to_str()
                .ok_or_else(|| CheckSchemaError::PathNotText(path.clone()))?;
            within.push(RepoPath(text.trim_end_matches('/').into()));
        }
        Ok(Self {
            start: edge.start.clone(),
            within,
            fixing: if schema.fix {
                Fixing::Fix
            } else {
                Fixing::Report
            },
            writing: edge.writing,
        })
    }
}

/// The service's request for the arguments. The paths are relative to the start; the service reads them from the
/// repository's root, so a path given here is joined to where the command runs.
///
/// # Errors
///
/// `PathNotText` for a path that is not text.
pub(super) fn request(
    paths: Vec<std::path::PathBuf>,
    fix: bool,
    edge: &crate::edge::Edge,
) -> Result<CheckRequest, CheckSchemaError> {
    CheckArgumentsSchema { paths, fix, edge }.try_into()
}

/// The request that proves every definition before the check, as `tm validate` with no filter asks.
pub(super) fn proving(edge: &crate::edge::Edge) -> ValidateRequest {
    ValidateRequest {
        start: edge.start.clone(),
        only: Vec::new(),
    }
}

/// What check says of `report`, the proofs made before it: one error for each definition and problem, none when every
/// definition holds. A definition is named `<kind> <package>/<name>@<version>`, as its id `<package>/<kind>/<name>@<version>`
/// reads.
pub(super) fn unproven(report: &ValidateReport) -> Vec<CheckError> {
    report
        .proved
        .iter()
        .flat_map(|proved| {
            let id = &proved.subject.id.0;
            let mut parts = id.rsplitn(3, '/');
            let (version, kind, package) = (parts.next(), parts.next(), parts.next());
            let named = match (package, kind, version) {
                (Some(package), Some(kind), Some(version)) => format!("{kind} {package}/{version}"),
                _ => id.to_string(),
            };
            match &proved.proof {
                Proof::Proven => Vec::new(),
                Proof::Failed(problems) => problems
                    .iter()
                    .map(|problem| CheckError::Proof {
                        subject: named.clone(),
                        reason: problem.0.to_string(),
                    })
                    .collect(),
            }
        })
        .collect()
}

/// One finding as JSON reads it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct FindingSchema {
    /// The rule or check.
    pub rule: String,
    /// The file.
    pub path: String,
    /// The first line, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// The first column, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    /// What is wrong.
    pub message: String,
    /// What to do: the text prints it under the finding.
    #[serde(skip)]
    pub help: Option<String>,
    /// What to do, in the JSON, when the finding says other than its rule's `rules` entry; a finding that says what its rule says
    /// carries none.
    #[serde(rename = "help", skip_serializing_if = "Option::is_none")]
    // ignore: idiomatic.arch/pub-item-unreferenced-elsewhere - a serialized field: only this file's serializer reads it, and the schema is the JSON key `help`
    pub own_help: Option<String>,
    /// What identifies it: Tastemaker's own fingerprint, which a SARIF log leaves out, so GitHub's `primaryLocationLineHash` is
    /// only ever computed by the reader, from the line, with one algorithm.
    pub fingerprint: String,
    /// Why an ignore accepts it, for a suppressed finding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The report as text and JSON read it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct ReportSchema {
    /// The workspace.
    pub workspace: String,
    /// The findings.
    pub findings: Vec<FindingSchema>,
    /// The suppressed findings.
    pub suppressed: Vec<FindingSchema>,
    /// What to do, once for each rule whose findings say it: a finding carries `help` itself only when it says something else.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub rules: BTreeMap<String, String>,
}

/// What `check` prints, and how the command ends once it is printed: the printed document is `printed` alone.
#[derive(Clone, Debug)]
pub(super) struct AnswerSchema {
    /// What is printed.
    printed: PrintedSchema,
    /// A failure when there is a finding.
    outcome: Outcome,
}

impl crate::outcome::Ending for AnswerSchema {
    fn ending(&self) -> Outcome {
        self.outcome
    }
}

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.printed.fmt(f)
    }
}

/// What `check` prints: its report, or a SARIF log.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(untagged)]
pub(super) enum PrintedSchema {
    /// Text or JSON.
    Report(ReportSchema),
    /// SARIF.
    Sarif(Box<Sarif>),
}

/// What the printed answer calls every finding: SARIF's word for a result that fails the check.
const ERROR: &str = tastemaker_core::domain::entries::constants::ERROR_WORD;

/// The line and column a finding's place names, each absent when the place does not reach that far.
fn position(place: Place) -> (Option<u32>, Option<u32>) {
    match place {
        Place::Whole => (None, None),
        Place::At(region) => (Some(region.start_line.0), Some(region.start_column.0)),
        Place::Line(line) => (Some(line.0), None),
    }
}

/// A finding as the printed answer and the JSON show it, with the reason it is suppressed when it is.
fn finding_schema(finding: &Finding, reason: Option<String>) -> FindingSchema {
    let (line, column) = position(finding.place);
    FindingSchema {
        rule: tastemaker_core::domain::findings::service::printed(&finding.rule)
            .0
            .to_string(),
        path: finding.path.0.to_string(),
        line,
        column,
        message: finding.message.0.to_string(),
        help: match &finding.help {
            Help::Said(text) => Some(text.0.to_string()),
            Help::Silent => None,
        },
        own_help: None,
        fingerprint: format!("{:016x}", finding.fingerprint.0),
        reason,
    }
}

/// The report as the printed answer and the JSON show it: its findings and its suppressed findings.
fn report_schema(report: &Report) -> ReportSchema {
    let mut findings: Vec<FindingSchema> = report
        .findings
        .iter()
        .map(|finding| finding_schema(finding, None))
        .collect();
    let mut suppressed: Vec<FindingSchema> = report
        .suppressed
        .iter()
        .map(|suppression| {
            finding_schema(&suppression.finding, Some(suppression.reason.0.to_string()))
        })
        .collect();
    // The help of a rule is said once: a rule whose findings all say the same text has that text as its own, and a finding that says
    // it carries none of its own in the JSON. A rule whose findings say different things (a help that names what was found) has none
    // of its own, so each of its findings says its.
    let mut said: BTreeMap<String, Option<&String>> = BTreeMap::new();
    for finding in findings.iter().chain(&suppressed) {
        if let Some(help) = &finding.help {
            let _: &mut Option<&String> = said
                .entry(finding.rule.clone())
                .and_modify(|own| {
                    if *own != Some(help) {
                        *own = None;
                    }
                })
                .or_insert(Some(help));
        }
    }
    let rules: BTreeMap<String, String> = said
        .into_iter()
        .filter_map(|(rule, help)| help.map(|help| (rule, help.clone())))
        .collect();
    for finding in findings.iter_mut().chain(suppressed.iter_mut()) {
        finding.own_help = finding
            .help
            .clone()
            .filter(|help| rules.get(&finding.rule) != Some(help));
    }
    ReportSchema {
        workspace: report.workspace.to_string(),
        findings,
        suppressed,
        rules,
    }
}

impl std::fmt::Display for PrintedSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self::Report(report) = self else {
            return Ok(());
        };
        let place = |finding: &FindingSchema| match (finding.line, finding.column) {
            (Some(line), Some(column)) => format!("{}:{line}:{column}", finding.path),
            (Some(line), None) => format!("{}:{line}", finding.path),
            _ => finding.path.clone(),
        };
        for finding in &report.findings {
            writeln!(
                f,
                "{ERROR}[{}]: {}: {}",
                finding.rule,
                place(finding),
                finding.message
            )?;
            if let Some(help) = &finding.help {
                writeln!(f, "  to fix: {help}")?;
            }
        }
        for finding in &report.suppressed {
            writeln!(
                f,
                "suppressed[{}]: {}: {} ({})",
                finding.rule,
                place(finding),
                finding.message,
                finding.reason.as_deref().unwrap_or_default()
            )?;
        }
        writeln!(
            f,
            "{}: {} error(s), {} suppressed",
            report.workspace,
            report.findings.len(),
            report.suppressed.len()
        )
    }
}

/// `path` as a URI path carries it.
fn encode(path: &str) -> String {
    utf8_percent_encode(path, PATH).to_string()
}

/// `text` as Markdown that renders exactly as written: a fenced code block longer than any backtick run in it.
fn literal(text: &str) -> String {
    let longest = text
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    format!("{fence}\n{text}\n{fence}")
}

/// `text` with its braces doubled, SARIF's literal braces (SARIF 2.1.0, 3.11.5): a single brace opens a placeholder.
fn braces_doubled(text: &str) -> String {
    text.replace('{', "{{").replace('}', "}}")
}

/// A result's message: `text` is the finding's message alone; `markdown` carries the message and `to fix: <help>`
/// inside a fenced code block; both with their braces doubled.
fn message(finding: &Finding) -> Message {
    let text = braces_doubled(&finding.message.0);
    match &finding.help {
        Help::Said(help) => Message::builder()
            .text(text)
            .markdown(braces_doubled(&literal(&format!(
                "{}\n\nto fix: {}",
                finding.message.0, help.0
            ))))
            .build(),
        Help::Silent => Message::builder().text(text).build(),
    }
}

/// `finding` as a result at `error`, as every finding is, suppressed by `suppressions` when there are any.
fn result(finding: &Finding, suppressions: Vec<SarifSuppression>) -> SarifResult {
    let artifact = ArtifactLocation::builder()
        .uri(encode(&finding.path.0))
        .uri_base_id(SOURCE_ROOT)
        .build();
    let physical = match finding.place {
        Place::At(region) => PhysicalLocation::builder()
            .artifact_location(artifact)
            .region(
                Region::builder()
                    .start_line(i64::from(region.start_line.0))
                    .start_column(i64::from(region.start_column.0))
                    .end_line(i64::from(region.end_line.0))
                    .end_column(i64::from(region.end_column.0))
                    .build(),
            )
            .build(),
        Place::Line(line) => PhysicalLocation::builder()
            .artifact_location(artifact)
            .region(Region::builder().start_line(i64::from(line.0)).build())
            .build(),
        Place::Whole => PhysicalLocation::builder()
            .artifact_location(artifact)
            .build(),
    };
    SarifResult::builder()
        .rule_id(tastemaker_core::domain::findings::service::printed(&finding.rule).0)
        .level(ResultLevel::Error)
        .message(message(finding))
        .locations(vec![
            Location::builder().physical_location(physical).build(),
        ])
        .suppressions(suppressions)
        .build()
}

/// A suppressed finding as SARIF carries it: the finding with its in-source, accepted suppression and the reason.
fn suppression(suppressed: &Suppression) -> SarifResult {
    let entry = SarifSuppression::builder()
        .kind(SupressionKind::InSource.to_string())
        .status(SupressionStatus::Accepted.to_string())
        .justification(&*suppressed.reason.0)
        .build();
    result(&suppressed.finding, vec![entry])
}

/// The SARIF log of `report`, its paths relative to `repository`.
fn sarif(report: &Report, repository: &str) -> Sarif {
    let base = format!("file://{}/", encode(repository.trim_end_matches('/')));
    let results = report
        .findings
        .iter()
        .map(|finding| result(finding, Vec::new()))
        .chain(report.suppressed.iter().map(suppression))
        .collect::<Vec<_>>();
    let driver = ToolComponent::builder()
        .name(TOOL)
        .version(VERSION)
        .information_uri(INFORMATION)
        .rules(
            report
                .evaluated
                .iter()
                .map(|rule| {
                    ReportingDescriptor::builder()
                        .id(tastemaker_core::domain::findings::service::printed(rule).0)
                        .build()
                })
                .collect::<Vec<_>>(),
        )
        .build();
    let run = Run::builder()
        .tool(Tool::builder().driver(driver).build())
        .invocations(vec![
            Invocation::builder().execution_successful(true).build(),
        ])
        .original_uri_base_ids(BTreeMap::from([(
            SOURCE_ROOT.to_owned(),
            ArtifactLocation::builder().uri(base).build(),
        )]))
        .results(results)
        .build();
    Sarif::builder()
        .schema(SCHEMA)
        .version(Version::V2_1_0.to_string())
        .runs(vec![run])
        .build()
}

/// The answer printed for the service's report in `format`, and the outcome it ends the command as: a failure when there
/// is a finding.
pub(super) fn answer(report: &CheckReport, format: Format) -> AnswerSchema {
    // Under `-n/--dry-run` the lock `--fix` would have rewritten is said, not written.
    if report.lock == tastemaker_core::inbound::use_cases::check::types::LockOutcome::Planned {
        crate::output::dry_run::planned(&"write the workspace's tastemaker.lock");
    }
    let outcome = if report.report.findings.is_empty() {
        Outcome::Success
    } else {
        Outcome::Failure
    };
    let printed = match format {
        Format::Sarif => {
            PrintedSchema::Sarif(Box::new(sarif(&report.report, &report.repository.0)))
        }
        Format::Terminal | Format::Json | Format::Markdown | Format::Quiet => {
            PrintedSchema::Report(report_schema(&report.report))
        }
    };
    AnswerSchema { printed, outcome }
}

/// What the user does next about a refusal of the arguments.
impl crate::outcome::remedy::Remedy for CheckSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::PathNotText(_) => crate::outcome::remedy::path_not_text(),
        }
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

impl crate::output::Encoded for AnswerSchema {
    fn json(
        &self,
        to: &mut dyn std::io::Write,
        spacing: crate::output::Spacing,
    ) -> std::io::Result<()> {
        match spacing {
            crate::output::Spacing::Compact => serde_json::to_writer(to, &self.printed),
            crate::output::Spacing::Indented => serde_json::to_writer_pretty(to, &self.printed),
        }
        .map_err(std::io::Error::from)
    }
}
