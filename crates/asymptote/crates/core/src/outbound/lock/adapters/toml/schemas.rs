// ignore: idiomatic.arch/file-length - the operation file is one byte format read in two directions: the serde shapes read it and the hand renderer writes its canonical bytes, which the content address hashes, so a field changes in both at once; the adapter spec places one schemas.rs for its wire forms, and no other role could take half of them.
// ignore: idiomatic.arch/serialize-dont-format - an operation's bytes are its content address: the hand renderer writes the one canonical text (every list sorted, one row a line, each section only when it holds something) that every committed operation was hashed from, a layout the toml serializer does not write
//! The shape of an operation's file: each wire shape is a `<Name>Schema`, read by serde, and its checked conversion
//! into the core's type is an `impl TryFrom`. Rendering is by hand, so the bytes, and with them the content address,
//! are canonical: every list sorted, each section written only when it holds something.
//!
//! An operation lists its parents, and a snapshot the operations it covers; then the keys it removes, the suppressed
//! findings it accepts (their reason is the in-source ignore beside the code, so they carry none), the suppressed keys
//! it removes and the perf keys it removes; then its accepted records and its perf records, each group under the
//! reasons it was accepted for, written once.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::constants::{LOCK_VERSION, PERF};
use crate::domain::judgement::constants::ACCEPTED;
use crate::domain::operations::types::{
    BenchmarkName, Count, Day, MeasureName, Number, Operation, OperationId, PerfKey, PerfRecord,
    Provenance, Reason, Record, Removal, Revision,
};
use crate::domain::violations::service::{level_of, normalize, word};
use crate::domain::violations::types::{
    Fingerprint, Id, Key, LevelWord, MessageText, RepoPath, RuleId, Standing, ToolId,
};

/// An operation as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OperationSchema {
    /// The format version, which must be `LOCK_VERSION`.
    version: u32,
    /// The digests of the operations this one follows: always written, empty for the first.
    parents: Vec<String>,
    /// The digests of the operations a snapshot replaces.
    #[serde(default)]
    covers: Vec<String>,
    /// The reported findings this operation drains from the lock, by key.
    #[serde(default)]
    removed: Vec<KeySchema>,
    /// The suppressed findings this operation accepts.
    #[serde(default)]
    suppressed: Vec<RowSchema>,
    /// The suppressed findings this operation drains from the lock, by key.
    #[serde(default, rename = "suppressed-removed")]
    suppressed_removed: Vec<KeySchema>,
    /// The perf records this operation drops.
    #[serde(default, rename = "perf-removed")]
    perf_removed: Vec<PerfKeySchema>,
    /// The reported findings this operation accepts, each group under its reasons.
    #[serde(default)]
    accepted: Vec<GroupSchema>,
    /// The perf bests this operation seeds or raises, each group under its reasons.
    #[serde(default)]
    perf: Vec<PerfGroupSchema>,
}

/// A finding's key as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct KeySchema {
    /// The tool that reported the finding.
    tool: String,
    /// The rule the finding breaks.
    rule: String,
    /// The product-relative file the finding is in.
    path: String,
    /// The level's word, `note`, `warning` or `error`.
    level: String,
    /// The position-independent fingerprint, in hexadecimal.
    fingerprint: String,
}

/// One accepted record as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RowSchema {
    /// The tool that reported the finding.
    tool: String,
    /// The rule the finding breaks.
    rule: String,
    /// The product-relative file the finding is in.
    path: String,
    /// The level's word, `note`, `warning` or `error`.
    level: String,
    /// The position-independent fingerprint, in hexadecimal.
    fingerprint: String,
    /// How many findings share the key.
    count: u32,
    /// The tool's message when the finding was accepted.
    message: String,
    /// The full object id of the revision the finding was accepted at, when known.
    #[serde(default)]
    since_rev: Option<String>,
    /// The day the finding was accepted.
    since: String,
}

/// Records accepted for the same reasons.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupSchema {
    /// Why, never empty.
    reasons: Vec<String>,
    /// The records.
    records: Vec<RowSchema>,
}

/// What a perf record or a removed perf key is for, as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PerfKeySchema {
    /// The tool whose benchmark it is.
    tool: String,
    /// The benchmark the tool reported.
    benchmark: String,
    /// What the benchmark measured.
    measure: String,
}

/// One perf record as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PerfRowSchema {
    /// The tool whose benchmark it is.
    tool: String,
    /// The benchmark the tool reported.
    benchmark: String,
    /// What the benchmark measured.
    measure: String,
    /// The best value the measure has reached, which a later run is judged against.
    best: f64,
}

/// Perf records set for the same reasons.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PerfGroupSchema {
    /// Why, never empty.
    reasons: Vec<String>,
    /// The records.
    records: Vec<PerfRowSchema>,
}

/// Why an operation's text was refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum OperationSchemaError {
    /// It is not TOML of an operation's shape.
    #[error("{0}")]
    Unparsable(String),
    /// A field of a record is wrong.
    #[error("{0}")]
    Field(String),
    /// It states another format version than the one this adapter reads.
    #[error("format version {0} is not {LOCK_VERSION}")]
    Version(u32),
}

/// An operation as written, offered to the conversion with what derives each record's id.
pub(super) struct VerifiedSchema<'a>(OperationSchema, &'a dyn Fn(&Key) -> Id);

/// The reasons a group was accepted for, refused when none is given or one is empty.
fn reasons(written: Vec<String>, what: &str) -> Result<Vec<Reason>, OperationSchemaError> {
    if written.is_empty() || written.iter().any(String::is_empty) {
        return Err(OperationSchemaError::Field(format!(
            "{what} has no attributable reason"
        )));
    }
    Ok(written
        .into_iter()
        .map(|reason| Reason(reason.into()))
        .collect())
}

/// The digests of `written`, each refused unless it is a lowercase BLAKE3 digest, sorted and deduplicated.
fn digests(written: Vec<String>, what: &str) -> Result<Vec<OperationId>, OperationSchemaError> {
    let mut ids = Vec::with_capacity(written.len());
    for id in written {
        if !is_hex(&id, &[64]) {
            return Err(OperationSchemaError::Field(format!(
                "{what} {id:?} is not a lowercase BLAKE3 digest"
            )));
        }
        ids.push(OperationId(id.into()));
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

impl TryFrom<VerifiedSchema<'_>> for Operation {
    type Error = OperationSchemaError;

    fn try_from(schema: VerifiedSchema<'_>) -> Result<Self, Self::Error> {
        let VerifiedSchema(schema, id_of) = schema;
        if schema.version != LOCK_VERSION {
            return Err(OperationSchemaError::Version(schema.version));
        }
        let mut accepted = Vec::new();
        for group in schema.accepted {
            let why = reasons(group.reasons, "a group of records")?;
            if group.records.is_empty() {
                return Err(OperationSchemaError::Field(
                    "a group of records holds no record".to_owned(),
                ));
            }
            for row in group.records {
                accepted.push(record(row, Standing::Reported, &why, id_of)?);
            }
        }
        for row in schema.suppressed {
            accepted.push(record(row, Standing::Suppressed, &[], id_of)?);
        }
        let mut removed = Vec::new();
        for (rows, standing) in [
            (schema.removed, Standing::Reported),
            (schema.suppressed_removed, Standing::Suppressed),
        ] {
            for row in rows {
                let key = key(row, standing)?;
                removed.push(Removal {
                    id: id_of(&key),
                    key,
                });
            }
        }
        let mut perf = Vec::new();
        for group in schema.perf {
            let why = reasons(group.reasons, "a group of perf records")?;
            if group.records.is_empty() {
                return Err(OperationSchemaError::Field(
                    "a group of perf records holds no record".to_owned(),
                ));
            }
            for row in group.records {
                perf.push(perf_record(row, &why).map_err(OperationSchemaError::Field)?);
            }
        }
        let perf_removed = schema
            .perf_removed
            .into_iter()
            .map(perf_key)
            .collect::<Result<std::collections::BTreeSet<_>, _>>()
            .map_err(OperationSchemaError::Field)?
            .into_iter()
            .collect();
        Ok(Self {
            parents: digests(schema.parents, "parent")?,
            covers: digests(schema.covers, "covered operation")?,
            accepted,
            removed,
            perf,
            perf_removed,
        })
    }
}

/// Whether `text` is a day, `YYYY-MM-DD`, its month 01 to 12 and its day 01 to 31.
fn is_day(text: &str) -> bool {
    let digits = |part: &str, low: u32, high: u32, length: usize| {
        part.len() == length
            && part.bytes().all(|byte| byte.is_ascii_digit())
            && part
                .parse::<u32>()
                .is_ok_and(|number| (low..=high).contains(&number))
    };
    let mut parts = text.split('-');
    matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(year), Some(month), Some(day), None)
            if digits(year, 0, 9999, 4) && digits(month, 1, 12, 2) && digits(day, 1, 31, 2)
    )
}

/// Whether `text` is a full lowercase hex object id or digest of `length` digits.
pub(super) fn is_hex(text: &str, lengths: &[usize]) -> bool {
    lengths.contains(&text.len())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Reads an operation's canonical text, deriving each record's id with `id_of`.
///
/// # Errors
///
/// When it is not an operation, or a record is wrong.
pub(super) fn parse(
    text: &str,
    id_of: &dyn Fn(&Key) -> Id,
) -> Result<Operation, OperationSchemaError> {
    let schema: OperationSchema = toml::from_str(text)
        .map_err(|error| OperationSchemaError::Unparsable(error.message().to_owned()))?;
    Operation::try_from(VerifiedSchema(schema, id_of))
}

/// The key a written key is, refused when a field is wrong.
fn key(row: KeySchema, standing: Standing) -> Result<Key, OperationSchemaError> {
    let field = |reason: String| OperationSchemaError::Field(reason);
    let fingerprint = u64::from_str_radix(&row.fingerprint, 16).map_err(|error| {
        field(format!(
            "fingerprint {:?} is not hexadecimal: {error}",
            row.fingerprint
        ))
    })?;
    if row.path.starts_with('/') || std::path::Path::new(&row.path).is_absolute() {
        return Err(field(format!(
            "path {:?} is absolute, not product-relative",
            row.path
        )));
    }
    if *normalize(&RepoPath(row.path.as_str().into())).0 != *row.path {
        return Err(field(format!(
            "path {:?} is not in normalized form",
            row.path
        )));
    }
    let level = level_of(LevelWord(&row.level))
        .ok_or_else(|| field(format!("unknown level {:?}", row.level)))?;
    Ok(Key {
        tool: ToolId(row.tool.into()),
        rule: RuleId(row.rule.into()),
        path: RepoPath(row.path.into()),
        level,
        standing,
        fingerprint: Fingerprint(fingerprint),
    })
}

/// The record a written row is, accepted for `why`, its id derived with `id_of`, refused when a field is wrong.
fn record(
    row: RowSchema,
    standing: Standing,
    why: &[Reason],
    id_of: &dyn Fn(&Key) -> Id,
) -> Result<Record, OperationSchemaError> {
    let field = |reason: String| OperationSchemaError::Field(reason);
    if row.count == 0 {
        return Err(field(format!("record at {} has a count of zero", row.path)));
    }
    if !is_day(&row.since) {
        return Err(field(format!(
            "since {:?} is not a day, YYYY-MM-DD",
            row.since
        )));
    }
    let since_rev = match row.since_rev {
        Some(revision) if is_hex(&revision, &[40, 64]) => Provenance::At(Revision(revision.into())),
        Some(revision) => {
            return Err(field(format!(
                "since revision {revision:?} is not a full object id"
            )));
        }
        None => Provenance::Unknown,
    };
    let key = key(
        KeySchema {
            tool: row.tool,
            rule: row.rule,
            path: row.path,
            level: row.level,
            fingerprint: row.fingerprint,
        },
        standing,
    )?;
    Ok(Record {
        id: id_of(&key),
        key,
        count: Count(row.count),
        message: MessageText(row.message.into()),
        since_rev,
        since: Day(row.since.into()),
        reasons: why.to_vec(),
    })
}

/// `value` as the inside of a TOML basic string.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if other.is_control() => {
                // `\u` and the control character's code in four upper-case hex digits.
                let code = u32::from(other);
                out.push_str("\\u");
                for shift in [12_u32, 8, 4, 0] {
                    out.extend(
                        char::from_digit((code >> shift) & 0xF, 16)
                            .map(|digit| digit.to_ascii_uppercase()),
                    );
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// `value` as a TOML basic string, quotes included.
fn quoted(value: &str) -> String {
    format!("\"{}\"", escape(value))
}

/// A list of quoted texts, on one line.
fn listed<'a>(values: impl Iterator<Item = &'a str>) -> String {
    values.map(quoted).collect::<Vec<_>>().join(", ")
}

/// A key's fields, between a row's braces.
fn key_fields(key: &Key) -> String {
    format!(
        "tool = {}, rule = {}, path = {}, level = \"{}\", fingerprint = \"{:016x}\"",
        quoted(&key.tool.0),
        quoted(&key.rule.0),
        quoted(&key.path.0),
        word(key.level).0,
        key.fingerprint.0
    )
}

/// A record's fields, between a row's braces.
fn record_fields(record: &Record) -> String {
    let mut fields = format!(
        "{}, count = {}, message = {}",
        key_fields(&record.key),
        record.count.0,
        quoted(&record.message.0)
    );
    if let Provenance::At(revision) = &record.since_rev {
        write!(fields, ", since_rev = {}", quoted(&revision.0)).unwrap_or_default();
    }
    write!(fields, ", since = {}", quoted(&record.since.0)).unwrap_or_default();
    fields
}

/// `name = [ ... ]` with one row a line, written only when there are rows.
fn section(out: &mut String, name: &str, rows: &[String]) {
    if rows.is_empty() {
        return;
    }
    writeln!(out, "\n{name} = [").unwrap_or_default();
    for row in rows {
        writeln!(out, "  {{ {row} }},").unwrap_or_default();
    }
    out.push_str("]\n");
}

/// The reasons of a group, sorted and deduplicated.
fn reason_set(reasons: &[Reason]) -> Vec<&str> {
    let mut set: Vec<&str> = reasons.iter().map(|reason| &*reason.0).collect();
    set.sort_unstable();
    set.dedup();
    set
}

/// A perf key's three names as the fields of a row.
fn perf_names(key: &PerfKey) -> String {
    format!(
        "tool = {}, benchmark = {}, measure = {}",
        quoted(&key.tool.0),
        quoted(&key.benchmark.0),
        quoted(&key.measure.0)
    )
}

/// `operation`'s canonical text: every list sorted, each section written only when it holds something.
#[must_use]
pub(super) fn render(operation: &Operation) -> String {
    let ids = |ids: &[OperationId]| {
        let mut sorted: Vec<&str> = ids.iter().map(|id| &*id.0).collect();
        sorted.sort_unstable();
        sorted.dedup();
        listed(sorted.into_iter())
    };
    let mut out = format!(
        "version = {LOCK_VERSION}\nparents = [{}]\n",
        ids(&operation.parents)
    );
    if !operation.covers.is_empty() {
        writeln!(out, "covers = [{}]", ids(&operation.covers)).unwrap_or_default();
    }
    let mut removed: Vec<&Removal> = operation.removed.iter().collect();
    removed.sort_by(|left, right| left.id.cmp(&right.id));
    let keys = |standing: Standing| -> Vec<String> {
        removed
            .iter()
            .filter(|removal| removal.key.standing == standing)
            .map(|removal| key_fields(&removal.key))
            .collect()
    };
    let mut accepted: Vec<&Record> = operation.accepted.iter().collect();
    accepted.sort_by(|left, right| left.id.cmp(&right.id));
    let suppressed: Vec<String> = accepted
        .iter()
        .filter(|record| record.key.standing == Standing::Suppressed)
        .map(|record| record_fields(record))
        .collect();
    let mut perf_removed: Vec<&PerfKey> = operation.perf_removed.iter().collect();
    perf_removed.sort();
    section(&mut out, "removed", &keys(Standing::Reported));
    section(&mut out, "suppressed", &suppressed);
    section(&mut out, "suppressed-removed", &keys(Standing::Suppressed));
    let removed_perf: Vec<String> = perf_removed.into_iter().map(perf_names).collect();
    section(&mut out, "perf-removed", &removed_perf);
    let mut groups: BTreeMap<Vec<&str>, Vec<String>> = BTreeMap::new();
    for record in accepted
        .iter()
        .filter(|record| record.key.standing == Standing::Reported)
    {
        groups
            .entry(reason_set(&record.reasons))
            .or_default()
            .push(record_fields(record));
    }
    let mut perf: Vec<&PerfRecord> = operation.perf.iter().collect();
    perf.sort_by(|left, right| left.key.cmp(&right.key));
    let mut perf_groups: BTreeMap<Vec<&str>, Vec<String>> = BTreeMap::new();
    for each in perf {
        // TOML reads a float only with a decimal point or an exponent: `Debug` keeps `40.0` where `Display` writes `40`.
        perf_groups
            .entry(reason_set(&each.reasons))
            .or_default()
            .push(format!(
                "{}, best = {:?}",
                perf_names(&each.key),
                each.best.0
            ));
    }
    for (table, grouped) in [(ACCEPTED, groups), (PERF, perf_groups)] {
        for (why, rows) in grouped {
            writeln!(
                out,
                "\n[[{table}]]\nreasons = [{}]\nrecords = [",
                listed(why.into_iter())
            )
            .unwrap_or_default();
            for row in rows {
                writeln!(out, "  {{ {row} }},").unwrap_or_default();
            }
            out.push_str("]\n");
        }
    }
    out
}

/// The key a written key names.
///
/// # Errors
///
/// When a part of it is empty.
fn perf_key(row: PerfKeySchema) -> Result<PerfKey, String> {
    named(row.tool, row.benchmark, row.measure)
}

/// The perf key of the three names, refused when one is empty.
fn named(tool: String, benchmark: String, measure: String) -> Result<PerfKey, String> {
    if tool.is_empty() || benchmark.is_empty() || measure.is_empty() {
        return Err("a perf record names an empty tool, benchmark or measure".to_owned());
    }
    Ok(PerfKey {
        tool: ToolId(tool.into()),
        benchmark: BenchmarkName(benchmark.into()),
        measure: MeasureName(measure.into()),
    })
}

/// The record a written row is, set for `why`.
///
/// # Errors
///
/// When a name is empty, or the best is not finite or is negative.
fn perf_record(row: PerfRowSchema, why: &[Reason]) -> Result<PerfRecord, String> {
    let key = named(row.tool, row.benchmark, row.measure)?;
    if !row.best.is_finite() || row.best < 0.0_f64 {
        return Err(format!(
            "perf record {}/{}/{}: `best` is not a number of zero or more",
            key.tool.0, key.benchmark.0, key.measure.0
        ));
    }
    Ok(PerfRecord {
        key,
        best: Number(row.best),
        reasons: why.to_vec(),
    })
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
