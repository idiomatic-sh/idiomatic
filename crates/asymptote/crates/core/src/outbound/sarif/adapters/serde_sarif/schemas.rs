// ignore: idiomatic.arch/file-length - the adapter spec places one schemas.rs for an adapter's wire forms, the only file that names serde-sarif's model: a SARIF log's checked conversion is one reading (version, rules, locations, URIs and their bases, coordinates), each refusal beside the field it guards, and the printed log is that model built back, so no other role could take half of it
//! The shape of a tool's SARIF log, read through `serde_sarif`'s model of it, and its checked conversion into the
//! core's results: every located result of every run, its URI resolved through `originalUriBaseIds` and decoded.

use std::collections::{BTreeMap, BTreeSet};

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde_sarif::sarif::{
    Artifact, ArtifactLocation, Location, Message, PhysicalLocation, Region, Result as SarifResult,
    ResultBaselineState, ResultKind, ResultLevel, Run, Sarif, Tool, ToolComponent, Version,
};

use super::constants::{
    ACCEPTED, END_COLUMN, END_LINE, FILE_SCHEME, INFORMATION, LINE_HASH, PRODUCT_ROOT, SCHEMA,
    START_COLUMN, START_LINE, VERSION,
};
use crate::domain::judgement::types::{Inventory, Judgement, Run as Judged};
use crate::domain::manifest::types::ProductRoot;
use crate::domain::violations::constants::WHOLE_FILE;
use crate::domain::violations::service::normalize;
use crate::domain::violations::types::{
    ColumnNumber, Fingerprint, Fingerprinted, Justification, Level, LineNumber, MessageText, Place,
    RepoPath, RuleId, Standing, Suppression, Violation,
};
use crate::outbound::sarif::errors::SarifError;
use crate::outbound::sarif::types::{Bound, Coordinate, End, Read, Reported, SarifText, Uri};

/// A log as the tool wrote it, offered to the conversion.
pub(super) struct LogSchema(pub Sarif);

impl TryFrom<LogSchema> for Read {
    type Error = SarifError;

    fn try_from(schema: LogSchema) -> Result<Self, Self::Error> {
        let LogSchema(log) = schema;
        // Only 2.1.0 is read: a log of another version, or of none, is no log this reader understands.
        if log.version.as_str() != Some(VERSION) {
            return Err(SarifError::Unparsable {
                reason: format!("version {} is not {VERSION}", log.version),
            });
        }
        let inventory = inventory(&log);
        let mut seen = BTreeSet::new();
        let mut results = Vec::new();
        for run in log.runs {
            if run
                .invocations
                .iter()
                .flatten()
                .any(|invocation| !invocation.execution_successful)
            {
                return Err(SarifError::Incomplete {
                    tool: run.tool.driver.name,
                });
            }
            let artifacts = run.artifacts.unwrap_or_default();
            let bases = run.original_uri_base_ids.unwrap_or_default();
            for result in run.results.into_iter().flatten() {
                // A result identical to one already read is the same violation reported again: a compiler reports a
                // line once per compilation unit that builds it.
                if let Some(reported) = reported(result, &artifacts, &bases, &run.tool)?
                    && seen.insert(reported.clone())
                {
                    results.push(reported);
                }
            }
        }
        Ok(Self { results, inventory })
    }
}

/// What `log` reports.
///
/// # Errors
///
/// `Unlocatable` when a result's location cannot be resolved to a local file.
pub(super) fn read(log: Sarif) -> Result<Read, SarifError> {
    Read::try_from(LogSchema(log))
}

/// The rules the log's runs say they evaluated: every `tool.driver.rules` id, when every run declares them, at least one in all, and
/// none says it did not complete; a partial set would resolve records of rules that merely did not run.
fn inventory(log: &Sarif) -> Inventory {
    let mut rules = BTreeSet::new();
    if log.runs.is_empty() {
        return Inventory::Undeclared;
    }
    for run in &log.runs {
        if run.results.iter().flatten().any(|result| {
            matches!(
                result.kind,
                Some(ResultKind::NotApplicable | ResultKind::Open)
            )
        }) {
            return Inventory::Undeclared;
        }
        let Some(declared) = run.tool.driver.rules.as_ref() else {
            return Inventory::Undeclared;
        };
        rules.extend(declared.iter().map(|rule| RuleId(rule.id.as_str().into())));
    }
    // A log that declares no rule says nothing of what ran, and a retained report holds no rule rows to tell it by: it is
    // undeclared fresh and from the cache alike.
    if rules.is_empty() {
        return Inventory::Undeclared;
    }
    Inventory::Declared(rules)
}

/// Whether an accepted in-source suppression covers `result`, and the justification it gives, empty where it gives
/// none.
fn suppressed(result: &SarifResult) -> Suppression {
    result
        .suppressions
        .iter()
        .flatten()
        .find(|suppression| {
            suppression
                .status
                .as_ref()
                .is_none_or(|status| status.as_str() == Some(ACCEPTED))
        })
        .map_or(Suppression::Unsuppressed, |suppression| {
            Suppression::Suppressed(Justification(
                suppression
                    .justification
                    .as_deref()
                    .unwrap_or_default()
                    .into(),
            ))
        })
}

/// `result` as the core reads it; none for a result that is no violation (`pass`, `informational`,
/// `notApplicable`). A violation with no rule or no physical location is refused: no record could hold it, and a
/// finding dropped unjudged would pass a check that should fail.
fn reported(
    result: SarifResult,
    artifacts: &[Artifact],
    bases: &BTreeMap<String, ArtifactLocation>,
    tool: &Tool,
) -> Result<Option<Reported>, SarifError> {
    if matches!(
        result.kind,
        Some(ResultKind::Pass | ResultKind::Informational | ResultKind::NotApplicable)
    ) {
        return Ok(None);
    }
    let Some(rule) = rule_id(&result, tool)? else {
        return Err(SarifError::Unparsable {
            reason: "a result names no rule".to_owned(),
        });
    };
    let Some(physical) = result
        .locations
        .iter()
        .flatten()
        .find_map(|location| location.physical_location.clone())
    else {
        return Err(SarifError::Unlocatable {
            rule,
            reason: "it has no physical location".to_owned(),
        });
    };
    let unlocatable = |reason: String| SarifError::Unlocatable {
        rule: rule.clone(),
        reason,
    };
    let artifact = physical
        .artifact_location
        .ok_or_else(|| unlocatable("a physical location with no artifactLocation".to_owned()))?;
    let path = path(&artifact, artifacts, bases).map_err(&unlocatable)?;
    // A coordinate the tool wrote and no line can have is a log that cannot be read, not one without the
    // coordinate: an invalid start line read as absent would make a whole-file result of it. SARIF's coordinates
    // are one-based, so zero is out of range too.
    let number = |value: Option<i64>, field: &str| match value {
        None => Ok(Bound::Absent),
        Some(value) => match u32::try_from(value) {
            Ok(coordinate) if coordinate >= 1 => Ok(Bound::Given(Coordinate(coordinate))),
            Ok(_) | Err(_) => Err(unlocatable(format!(
                "its region's {field} {value} is out of range"
            ))),
        },
    };
    let (start, column, end) = match physical.region.as_ref() {
        Some(region) => (
            number(region.start_line, START_LINE)?,
            number(region.start_column, START_COLUMN)?,
            End {
                line: number(region.end_line, END_LINE)?,
                column: number(region.end_column, END_COLUMN)?,
            },
        ),
        None => (
            Bound::Absent,
            Bound::Absent,
            End {
                line: Bound::Absent,
                column: Bound::Absent,
            },
        ),
    };
    let place = match (start, column) {
        (Bound::Given(line), Bound::Given(column)) => {
            Place::At(LineNumber(line.0), ColumnNumber(column.0))
        }
        (Bound::Given(line), Bound::Absent) => Place::Line(LineNumber(line.0)),
        (Bound::Absent, _) => Place::File,
    };
    let suppression = suppressed(&result);
    let supplied = result
        .partial_fingerprints
        .as_ref()
        .and_then(|prints| prints.get(LINE_HASH))
        .and_then(|hash| hash.split(':').next())
        .and_then(|hex| u64::from_str_radix(hex, 16).ok()) // ignore: idiomatic.arch/no-ok-on-result - a lineHash that is no hexadecimal number is no fingerprint the tool gave: the result is fingerprinted by its place
        .map_or(Fingerprinted::Unknown, |raw| {
            Fingerprinted::Known(Fingerprint(raw))
        });
    Ok(Some(Reported {
        path: normalize(&RepoPath(path.into())),
        place,
        end,
        rule: RuleId(rule.as_str().into()),
        level: match result.level {
            Some(ResultLevel::Note) => Level::Note,
            Some(ResultLevel::Warning) => Level::Warning,
            Some(ResultLevel::Error | ResultLevel::None) | None => Level::Error,
        },
        suppression,
        message: MessageText(result.message.text.unwrap_or_default().into()),
        fingerprint: match place {
            Place::File => Fingerprinted::Known(WHOLE_FILE),
            Place::Line(_) | Place::At(..) => Fingerprinted::Unknown,
        },
        supplied,
    }))
}

/// Resolves the equivalent SARIF rule references without losing a result that uses metadata.
fn rule_id(result: &SarifResult, tool: &Tool) -> Result<Option<String>, SarifError> {
    let invalid = |reason: &str| SarifError::Unparsable {
        reason: reason.to_owned(),
    };
    let reference = result.rule.as_ref();
    let named = reference.and_then(|rule| rule.id.as_ref());
    if matches!((result.rule_id.as_ref(), named), (Some(left), Some(right)) if left != right) {
        return Err(invalid("result.ruleId disagrees with result.rule.id"));
    }
    let specified = |index: Option<i64>| index.filter(|index| *index != -1);
    let nested = specified(reference.and_then(|rule| rule.index));
    let outer = specified(result.rule_index);
    if matches!((outer, nested), (Some(left), Some(right)) if left != right) {
        return Err(invalid("result.ruleIndex disagrees with result.rule.index"));
    }
    let direct = result.rule_id.as_ref().or(named);
    let Some(index) = outer.or(nested) else {
        return Ok(direct.cloned());
    };
    let component = match reference.and_then(|rule| rule.tool_component.as_ref()) {
        None => &tool.driver,
        Some(component) => {
            let requested = specified(component.index);
            let name = component.name.as_ref();
            let guid = component.guid.as_ref();
            let extensions = tool
                .extensions
                .iter()
                .flatten()
                .enumerate()
                .map(|(position, tool)| (i64::try_from(position).ok(), tool)); // ignore: idiomatic.arch/no-ok-on-result - a position past what i64 holds is one no toolComponent index names, so it matches none
            std::iter::once((None, &tool.driver))
                .chain(extensions)
                .find_map(|(position, tool)| {
                    let matches = requested.is_none_or(|asked| Some(asked) == position)
                        && name.is_none_or(|name| name == &tool.name)
                        && guid.is_none_or(|guid| tool.guid.as_ref() == Some(guid));
                    matches.then_some(tool)
                })
                .ok_or_else(|| invalid("result.rule.toolComponent does not identify a component"))?
        }
    };
    let descriptor = usize::try_from(index)
        .ok() // ignore: idiomatic.arch/no-ok-on-result - an index no usize holds is outside the component's rules, which the next line refuses
        .and_then(|index| component.rules.as_ref()?.get(index))
        .ok_or_else(|| invalid("result rule index is outside the component's rules"))?;
    if direct
        .is_some_and(|id| id != &descriptor.id && !id.starts_with(&format!("{}/", descriptor.id)))
    {
        return Err(invalid("rule id conflicts with indexed descriptor"));
    }
    Ok(Some(direct.unwrap_or(&descriptor.id).clone()))
}

/// The local path an artifact location names: its URI, its index into the run's artifacts, or both agreeing.
fn path(
    location: &ArtifactLocation,
    artifacts: &[Artifact],
    bases: &BTreeMap<String, ArtifactLocation>,
) -> Result<String, String> {
    let direct = location
        .uri
        .as_ref()
        .map(|_| from_uri(location, bases))
        .transpose()?;
    let indexed = location
        .index
        .map(|index| {
            let artifact = usize::try_from(index)
                .ok() // ignore: idiomatic.arch/no-ok-on-result - an index no usize holds is outside run.artifacts, which the next line refuses
                .and_then(|position| artifacts.get(position))
                .ok_or_else(|| format!("index {index} is outside run.artifacts"))?;
            let inner = artifact
                .location
                .as_ref()
                .filter(|inner| inner.uri.is_some())
                .ok_or_else(|| format!("run.artifacts[{index}] has no uri"))?;
            from_uri(inner, bases)
        })
        .transpose()?;
    match (direct, indexed) {
        (Some(direct), Some(indexed))
            if normalize(&RepoPath(direct.as_str().into()))
                != normalize(&RepoPath(indexed.as_str().into())) =>
        {
            Err(format!(
                "uri identifies {direct:?}, but index identifies {indexed:?}"
            ))
        }
        (Some(path), _) | (None, Some(path)) => Ok(path),
        (None, None) => Err("neither uri nor index is present".to_owned()),
    }
}

/// The local path a URI names, resolved through its base and decoded.
fn from_uri(
    location: &ArtifactLocation,
    bases: &BTreeMap<String, ArtifactLocation>,
) -> Result<String, String> {
    let uri = location
        .uri
        .as_deref()
        .ok_or_else(|| "uri is absent".to_owned())?;
    let resolved = if has_scheme(uri) {
        if location.uri_base_id.is_some() {
            return Err("an absolute uri also names a uriBaseId".to_owned());
        }
        uri.to_owned()
    } else if let Some(base) = location.uri_base_id.as_deref() {
        format!("{}{uri}", resolve(base, bases)?)
    } else {
        uri.to_owned()
    };
    local(&resolved)
}

/// The URI a base id stands for, followed through the bases it names in turn.
fn resolve(first: &str, bases: &BTreeMap<String, ArtifactLocation>) -> Result<String, String> {
    let mut id = first.to_owned();
    let mut seen = BTreeSet::new();
    let mut resolved = String::new();
    loop {
        if !seen.insert(id.clone()) {
            return Err(format!("uriBaseId {id:?} forms a cycle"));
        }
        let location = bases
            .get(&id)
            .ok_or_else(|| format!("uriBaseId {id:?} is not in originalUriBaseIds"))?;
        let uri = location
            .uri
            .as_deref()
            .ok_or_else(|| format!("uriBaseId {id:?} has no uri"))?;
        if !uri.ends_with('/')
            || uri.contains(['?', '#'])
            || uri.split('/').any(|segment| segment == "..")
        {
            return Err(format!("uriBaseId {id:?} is no directory uri"));
        }
        resolved.insert_str(0, uri);
        if has_scheme(uri) {
            return Ok(resolved);
        }
        let Some(next) = location.uri_base_id.as_deref() else {
            return Ok(resolved);
        };
        next.clone_into(&mut id);
    }
}

/// The path a resolved URI names on this machine: a `file` URI's path, or a relative reference, percent-decoded.
fn local(uri: &str) -> Result<String, String> {
    let bare = uri.split(['#', '?']).next().unwrap_or_default();
    let encoded = if let Some(rest) = bare.strip_prefix(FILE_SCHEME) {
        if let Some(path) = rest.strip_prefix("localhost/") {
            format!("/{path}")
        } else if rest.starts_with('/') {
            rest.to_owned()
        } else {
            return Err("the file uri names a remote host".to_owned());
        }
    } else if let Some(path) = bare.strip_prefix("file:") {
        path.to_owned()
    } else if has_scheme(bare) {
        return Err("the uri names no local file".to_owned());
    } else {
        bare.to_owned()
    };
    let escaped = encoded.split('%').skip(1).all(|after| {
        after
            .as_bytes()
            .get(..2)
            .is_some_and(|digits| digits.iter().all(u8::is_ascii_hexdigit))
    });
    if !escaped {
        return Err("the uri has malformed percent encoding".to_owned());
    }
    percent_encoding::percent_decode_str(&encoded)
        .decode_utf8()
        .map(std::borrow::Cow::into_owned)
        .map_err(|error| format!("the uri does not decode to UTF-8: {error}"))
}

/// Whether `uri` starts with a scheme.
fn has_scheme(uri: &str) -> bool {
    uri.split_once(':').is_some_and(|(scheme, _)| {
        scheme
            .as_bytes()
            .split_first()
            .is_some_and(|(first, rest)| {
                first.is_ascii_alphabetic()
                    && rest.iter().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.')
                    })
            })
    })
}

/// What the log `text` reports, read through serde-sarif's model of it.
///
/// # Errors
///
/// `Unparsable` when the text is no SARIF log, and every refusal of `read`.
pub(super) fn decoded(text: &str) -> Result<Read, SarifError> {
    let log: Sarif = serde_json::from_str(text).map_err(|error| SarifError::Unparsable {
        reason: error.to_string(),
    })?;
    read(log)
}

/// The printed log of `runs` against `outcome`, every result's path relative to `root`: one run per tool, each result's
/// `baselineState` saying how it stands against the lock.
pub(super) fn written(root: &ProductRoot, runs: &[Judged], outcome: &Judgement) -> SarifText {
    let base = format!(
        "{FILE_SCHEME}{}/",
        encode(&RepoPath(root.0.trim_end_matches('/').into())).0
    );
    let resolved: Vec<Violation> = outcome
        .resolved
        .iter()
        .map(|resolved| Violation {
            path: resolved.record.key.path.clone(),
            place: Place::File,
            tool: resolved.record.key.tool.clone(),
            rule: resolved.record.key.rule.clone(),
            level: resolved.record.key.level,
            // The lock keeps no reason for a suppressed record: its reason was the ignore beside the code.
            suppression: match resolved.record.key.standing {
                Standing::Reported => Suppression::Unsuppressed,
                Standing::Suppressed => Suppression::Suppressed(Justification::default()),
            },
            message: resolved.record.message.clone(),
            fingerprint: Fingerprinted::Known(resolved.record.key.fingerprint),
        })
        .collect();
    let runs = runs
        .iter()
        .map(|run| {
            let of = |found: &&Violation| found.tool == run.tool;
            let mut mine: Vec<SarifResult> = Vec::new();
            for (found, baseline) in [
                (&outcome.new, ResultBaselineState::New),
                (&outcome.accepted, ResultBaselineState::Unchanged),
                (&resolved, ResultBaselineState::Absent),
            ] {
                mine.extend(
                    found
                        .iter()
                        .filter(of)
                        .map(|found| result(found, &baseline)),
                );
            }
            Run::builder()
                .tool(
                    Tool::builder()
                        .driver(
                            ToolComponent::builder()
                                .name(&*run.tool.0)
                                .information_uri(INFORMATION)
                                .build(),
                        )
                        .build(),
                )
                .original_uri_base_ids(BTreeMap::from([(
                    PRODUCT_ROOT.to_owned(),
                    ArtifactLocation::builder().uri(base.clone()).build(),
                )]))
                .results(mine)
                .build()
        })
        .collect::<Vec<_>>();
    let sarif = Sarif::builder()
        .schema(SCHEMA)
        .version(serde_json::to_value(Version::V2_1_0).unwrap_or_default())
        .runs(runs)
        .build();
    SarifText(
        serde_json::to_string_pretty(&sarif)
            .unwrap_or_default()
            .into(),
    )
}

/// What a SARIF path carries as written: letters, digits and `/-._~`.
const PATH: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'/')
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// `path` as a URI path carries it.
fn encode(path: &RepoPath) -> Uri {
    Uri(utf8_percent_encode(&path.0, PATH).to_string().into())
}

/// One printed result.
fn result(found: &Violation, baseline: &ResultBaselineState) -> SarifResult {
    let artifact = ArtifactLocation::builder()
        .uri(&*encode(&found.path).0)
        .uri_base_id(PRODUCT_ROOT)
        .build();
    let physical = match found.place {
        Place::At(line, column) => PhysicalLocation::builder()
            .artifact_location(artifact)
            .region(
                Region::builder()
                    .start_line(i64::from(line.0))
                    .start_column(i64::from(column.0))
                    .build(),
            )
            .build(),
        Place::Line(line) => PhysicalLocation::builder()
            .artifact_location(artifact)
            .region(Region::builder().start_line(i64::from(line.0)).build())
            .build(),
        Place::File => PhysicalLocation::builder()
            .artifact_location(artifact)
            .build(),
    };
    let level = match found.level {
        Level::Note => ResultLevel::Note,
        Level::Warning => ResultLevel::Warning,
        Level::Error => ResultLevel::Error,
    };
    let accepted = |why: &Justification| {
        let kind =
            serde_json::to_value(serde_sarif::sarif::SupressionKind::InSource).unwrap_or_default();
        let status = serde_json::to_value(serde_sarif::sarif::SupressionStatus::Accepted)
            .unwrap_or_default();
        // A justification is written only where the ignore gave one.
        if why.0.is_empty() {
            serde_sarif::sarif::Suppression::builder()
                .kind(kind)
                .status(status)
                .build()
        } else {
            serde_sarif::sarif::Suppression::builder()
                .kind(kind)
                .status(status)
                .justification(&*why.0)
                .build()
        }
    };
    let suppressions = match &found.suppression {
        Suppression::Suppressed(why) => vec![accepted(why)],
        Suppression::Unsuppressed => Vec::new(),
    };
    let mut printed = SarifResult::builder()
        .rule_id(&*found.rule.0)
        .level(level)
        .message(Message::builder().text(&*found.message.0).build())
        .locations(vec![
            Location::builder().physical_location(physical).build(),
        ])
        .baseline_state(serde_json::to_value(baseline).unwrap_or_default())
        .suppressions(suppressions)
        .build();
    // The fingerprint goes where code scanning reads a result's identity from, and where a log is read back from.
    printed.partial_fingerprints = match found.fingerprint {
        Fingerprinted::Known(print) => Some(BTreeMap::from([(
            LINE_HASH.to_owned(),
            format!("{:016x}", print.0),
        )])),
        Fingerprinted::Unknown => None,
    };
    printed
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
