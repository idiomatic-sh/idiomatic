// ignore: idiomatic.arch/file-length - the adapter spec places one schemas.rs for an adapter's wire forms: the store's tables are one postcard format, each table's writing beside its reading and every row naming the one strings table, so a column changes in both directions at once, and no other role could take half of it.
//! The tables of `check`'s store, written with postcard: flat rows of integers, each text stored once in `strings`
//! and named by its number. `tools` holds each tool's run, `rules` the rules a run said it evaluated, `inputs` what
//! each pathspec of a tool's inputs came to, `violations` what it reported, `measurements` what a perf tool measured,
//! and `files` the stamp of each file the judgement rests on. The build is kept once. A store that does not decode,
//! or one whose rows name a text it does not hold, is a cold miss.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::constants::{
    COMPLETED, DETACHED, EXITED, RETAINED, SIGNALLED, TIMED_OUT, UNREADABLE, UNSTARTABLE,
};
use crate::domain::judgement::types::{Code, Failure, Inventory, Problem, Run, Status, Tail};
use crate::domain::manifest::types::{Pathspec, Seconds};
use crate::domain::operations::types::{BenchmarkName, MeasureName, Number, Revision};
use crate::domain::perf::types::{Measurement, Spread};
use crate::domain::renames::types::Head;
use crate::domain::reports::types::{
    Digest, FileRecord, Keeping, Moment, NamedInput, Recorded, Report, Scanned, Size, Stamped,
};
use crate::domain::violations::service::{level_of, word};
use crate::domain::violations::types::{
    ColumnNumber, Fingerprint, Fingerprinted, Justification, LevelWord, LineNumber, MessageText,
    Place, RepoPath, RuleId, Suppression, ToolId, Violation,
};
use crate::outbound::check_store::types::{Build, KeptReport, Stored};

/// One tool's run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct ToolRowSchema {
    /// The tool's id, as `asymptote.toml` writes it.
    tool_id: u32,
    /// The stamp of its inputs; none when they could not be listed.
    inputs_digest: Option<u32>,
    /// How it ended: `completed`, `retained` (completed by an earlier check), or how it failed.
    outcome: u32,
    /// Its exit code, when it exited with one that is no verdict.
    exit_code: Option<i32>,
}

/// A rule a tool's run said it evaluated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct RuleRowSchema {
    /// The tool.
    tool_id: u32,
    /// The rule.
    rule: u32,
}

/// What one pathspec of a tool's inputs came to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct InputRowSchema {
    /// The tool.
    tool_id: u32,
    /// The pathspec, as `inputs` writes it.
    path: u32,
    /// The digest of the node it names in the input tree.
    digest: u32,
}

/// One violation a tool reported.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct ViolationRowSchema {
    /// The tool.
    tool_id: u32,
    /// The rule.
    rule: u32,
    /// Its fingerprint; none when its line could not be read.
    fingerprint: Option<u64>,
    /// The tool's message.
    description: u32,
    /// The file.
    path: u32,
    /// Its line, when it has one.
    line: Option<u32>,
    /// Its column, when it has one.
    column: Option<u32>,
    /// `note`, `warning` or `error`.
    level: u32,
    /// Whether an in-source ignore accepts it.
    accepted: bool,
    /// Why, as the ignore says it, when one accepts it.
    reasons: Option<u32>,
}

/// What a perf tool measured.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct MeasurementRowSchema {
    /// The tool.
    tool_id: u32,
    /// The benchmark.
    benchmark: u32,
    /// The measure.
    measure: u32,
    /// The median.
    value: f64,
    /// The spread's low end, when the tool gave one.
    lower: Option<f64>,
    /// The spread's high end, when the tool gave one.
    upper: Option<f64>,
}

/// One file the judgement rests on: the stamp of its bytes and what its size and modification time were then.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct FileRowSchema {
    /// The file.
    path: u32,
    /// The digest of its bytes.
    digest: u32,
    /// Its size.
    size: u64,
    /// Its modification time, in nanoseconds since the Unix epoch.
    modified: u64,
}

/// The store: the build and what is kept once, then each table.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct StoreSchema {
    /// The build that wrote it: another build's store opens empty.
    build: String,
    /// The commit the check ran on, or `DETACHED`; none when no check kept a report.
    head: Option<u32>,
    /// When the file records were taken, in nanoseconds since the Unix epoch.
    taken: u64,
    /// Every text the rows name, each once, by its position.
    strings: Vec<String>,
    /// The tools' runs.
    tools: Vec<ToolRowSchema>,
    /// The rules they said they evaluated.
    rules: Vec<RuleRowSchema>,
    /// What each tool's pathspecs came to.
    inputs: Vec<InputRowSchema>,
    /// What the tools reported.
    violations: Vec<ViolationRowSchema>,
    /// What the perf tools measured.
    measurements: Vec<MeasurementRowSchema>,
    /// The files the judgement rests on.
    files: Vec<FileRowSchema>,
}

/// The texts a store being written names, each numbered once.
#[derive(Default)]
struct StringsSchema {
    /// The number of each text.
    ids: HashMap<String, u32>,
    /// The texts, by number.
    texts: Vec<String>,
}

/// The number of `text` in `strings`, numbering it when it is new.
fn id(strings: &mut StringsSchema, text: &str) -> u32 {
    if let Some(known) = strings.ids.get(text) {
        return *known;
    }
    let next = u32::try_from(strings.texts.len()).unwrap_or(u32::MAX);
    strings.texts.push(text.to_owned());
    let _: Option<u32> = strings.ids.insert(text.to_owned(), next);
    next
}

/// Nanoseconds as the store keeps them, past `u64` saturated.
fn nanoseconds(moment: Moment) -> u64 {
    u64::try_from(moment.0).unwrap_or(u64::MAX)
}

/// The outcome word and exit code of a run.
const fn outcome(status: &Status, keeping: Keeping) -> (&'static str, Option<i32>) {
    match (status, keeping) {
        (Status::Completed, Keeping::Current) => (COMPLETED, None),
        (Status::Completed, Keeping::Retained) => (RETAINED, None),
        (Status::Failed(Failure::Exited { code, .. }), _) => (EXITED, Some(code.0)),
        (Status::Failed(Failure::Signalled { .. }), _) => (SIGNALLED, None),
        (Status::Failed(Failure::TimedOut { .. }), _) => (TIMED_OUT, None),
        (Status::Failed(Failure::Unstartable(_)), _) => (UNSTARTABLE, None),
        (Status::Failed(Failure::Unreadable { .. }), _) => (UNREADABLE, None),
    }
}

/// Adds the rows of one scan to `store`.
fn add_scan(store: &mut StoreSchema, strings: &mut StringsSchema, scan: &Scanned) {
    let tool_id = id(strings, &scan.run.tool.0);
    let (ended, exit_code) = outcome(&scan.run.status, scan.keeping);
    store.tools.push(ToolRowSchema {
        tool_id,
        inputs_digest: match &scan.stamp {
            Stamped::Digested(stamp) => Some(id(strings, &stamp.0)),
            Stamped::Unstamped => None,
        },
        outcome: id(strings, ended),
        exit_code,
    });
    if let Inventory::Declared(rules) = &scan.run.inventory {
        for rule in rules {
            let rule = id(strings, &rule.0);
            store.rules.push(RuleRowSchema { tool_id, rule });
        }
    }
    for input in &scan.inputs {
        let (path, digest) = (id(strings, &input.spec.0), id(strings, &input.digest.0));
        store.inputs.push(InputRowSchema {
            tool_id,
            path,
            digest,
        });
    }
    for found in &scan.violations {
        let (line, column) = match found.place {
            Place::File => (None, None),
            Place::Line(line) => (Some(line.0), None),
            Place::At(line, column) => (Some(line.0), Some(column.0)),
        };
        store.violations.push(ViolationRowSchema {
            tool_id,
            rule: id(strings, &found.rule.0),
            fingerprint: match found.fingerprint {
                Fingerprinted::Known(print) => Some(print.0),
                Fingerprinted::Unknown => None,
            },
            description: id(strings, &found.message.0),
            path: id(strings, &found.path.0),
            line,
            column,
            level: id(strings, word(found.level).0),
            accepted: matches!(found.suppression, Suppression::Suppressed(_)),
            reasons: match &found.suppression {
                Suppression::Suppressed(why) => Some(id(strings, &why.0)),
                Suppression::Unsuppressed => None,
            },
        });
    }
    let bound = |spread: Spread| match spread {
        Spread::Given(number) => Some(number.0),
        Spread::Absent => None,
    };
    for measured in &scan.measurements {
        store.measurements.push(MeasurementRowSchema {
            tool_id,
            benchmark: id(strings, &measured.benchmark.0),
            measure: id(strings, &measured.measure.0),
            value: measured.value.0,
            lower: bound(measured.lower),
            upper: bound(measured.upper),
        });
    }
}

/// What the store keeps, offered to the conversion with the build that keeps it.
pub(super) struct KeptSchema<'a>(&'a Stored, &'a Build);

impl From<KeptSchema<'_>> for StoreSchema {
    fn from(kept: KeptSchema<'_>) -> Self {
        let KeptSchema(stored, build) = kept;
        let mut strings = StringsSchema::default();
        let mut store = Self {
            build: build.0.to_string(),
            taken: nanoseconds(stored.recorded.taken),
            ..Self::default()
        };
        if let KeptReport::Kept(report) = &stored.report {
            store.head = Some(match &report.head {
                Head::Commit(revision) => id(&mut strings, &revision.0),
                Head::Detached => id(&mut strings, DETACHED),
            });
            for scan in &report.scanned {
                add_scan(&mut store, &mut strings, scan);
            }
        }
        for (path, record) in &stored.recorded.files {
            store.files.push(FileRowSchema {
                path: id(&mut strings, &path.0),
                digest: id(&mut strings, &record.digest.0),
                size: record.size.0,
                modified: nanoseconds(record.modified),
            });
        }
        store.strings = strings.texts;
        store
    }
}

/// The status and keeping an outcome word and exit code name. A failed run is kept only to say that it failed: what
/// it said is not.
fn status(word: &str, exit_code: Option<i32>) -> Option<(Status, Keeping)> {
    let failed = |failure| Some((Status::Failed(failure), Keeping::Current));
    if word == COMPLETED {
        Some((Status::Completed, Keeping::Current))
    } else if word == RETAINED {
        Some((Status::Completed, Keeping::Retained))
    } else if word == EXITED {
        failed(Failure::Exited {
            code: Code(exit_code?),
            stderr: Tail::default(),
        })
    } else if word == SIGNALLED {
        failed(Failure::Signalled {
            stderr: Tail::default(),
        })
    } else if word == TIMED_OUT {
        failed(Failure::TimedOut { after: Seconds(0) })
    } else if word == UNSTARTABLE {
        failed(Failure::Unstartable(Problem::default()))
    } else if word == UNREADABLE {
        failed(Failure::Unreadable {
            problem: Problem::default(),
            stderr: Tail::default(),
        })
    } else {
        None
    }
}

/// The violation a row records, its texts read from `text`.
fn violation(
    row: &ViolationRowSchema,
    text: &dyn Fn(u32) -> Option<Box<str>>,
) -> Option<Violation> {
    let written = text(row.level)?;
    Some(Violation {
        path: RepoPath(text(row.path)?),
        place: match (row.line, row.column) {
            (None, None) => Place::File,
            (Some(line), None) => Place::Line(LineNumber(line)),
            (Some(line), Some(column)) => Place::At(LineNumber(line), ColumnNumber(column)),
            (None, Some(_)) => return None,
        },
        tool: ToolId(text(row.tool_id)?),
        rule: RuleId(text(row.rule)?),
        level: level_of(LevelWord(&written))?,
        suppression: if row.accepted {
            Suppression::Suppressed(Justification(match row.reasons {
                Some(why) => text(why)?,
                None => Box::default(),
            }))
        } else {
            Suppression::Unsuppressed
        },
        message: MessageText(text(row.description)?),
        fingerprint: row.fingerprint.map_or(Fingerprinted::Unknown, |print| {
            Fingerprinted::Known(Fingerprint(print))
        }),
    })
}

/// The scans the rows of `store` record, in the order of `tools`.
fn scans(store: &StoreSchema, text: &dyn Fn(u32) -> Option<Box<str>>) -> Option<Vec<Scanned>> {
    let mut violations: BTreeMap<u32, Vec<Violation>> = BTreeMap::new();
    for row in &store.violations {
        violations
            .entry(row.tool_id)
            .or_default()
            .push(violation(row, text)?);
    }
    let mut rules: BTreeMap<u32, BTreeSet<RuleId>> = BTreeMap::new();
    for row in &store.rules {
        let _: bool = rules
            .entry(row.tool_id)
            .or_default()
            .insert(RuleId(text(row.rule)?));
    }
    let mut inputs: BTreeMap<u32, Vec<NamedInput>> = BTreeMap::new();
    for row in &store.inputs {
        inputs.entry(row.tool_id).or_default().push(NamedInput {
            spec: Pathspec(text(row.path)?),
            digest: Digest(text(row.digest)?),
        });
    }
    let mut measurements: BTreeMap<u32, Vec<Measurement>> = BTreeMap::new();
    for row in &store.measurements {
        let bound =
            |value: Option<f64>| value.map_or(Spread::Absent, |value| Spread::Given(Number(value)));
        measurements
            .entry(row.tool_id)
            .or_default()
            .push(Measurement {
                benchmark: BenchmarkName(text(row.benchmark)?),
                measure: MeasureName(text(row.measure)?),
                value: Number(row.value),
                lower: bound(row.lower),
                upper: bound(row.upper),
            });
    }
    let mut scanned = Vec::with_capacity(store.tools.len());
    for row in &store.tools {
        let (status, keeping) = status(&text(row.outcome)?, row.exit_code)?;
        // A failed run holds no rows: the rows of its tool are those of the completed run kept beside it.
        let failed = matches!(status, Status::Failed(_));
        let found = if failed {
            Vec::new()
        } else {
            violations.remove(&row.tool_id).unwrap_or_default()
        };
        // A run said which rules it evaluated when `rules` holds any of its.
        let declared = if failed {
            None
        } else {
            rules.remove(&row.tool_id)
        };
        scanned.push(Scanned {
            run: Run {
                tool: ToolId(text(row.tool_id)?),
                status,
                inventory: declared.map_or(Inventory::Undeclared, Inventory::Declared),
                reported: found.iter().map(|found| found.rule.clone()).collect(),
            },
            violations: found,
            stamp: match row.inputs_digest {
                Some(stamp) => Stamped::Digested(Digest(text(stamp)?)),
                None => Stamped::Unstamped,
            },
            inputs: if failed {
                Vec::new()
            } else {
                inputs.remove(&row.tool_id).unwrap_or_default()
            },
            measurements: if failed {
                Vec::new()
            } else {
                measurements.remove(&row.tool_id).unwrap_or_default()
            },
            keeping,
        });
    }
    Some(scanned)
}

impl TryFrom<StoreSchema> for Stored {
    type Error = ();

    fn try_from(store: StoreSchema) -> Result<Self, Self::Error> {
        let text = |at: u32| -> Option<Box<str>> {
            Some(
                store
                    .strings
                    .get(usize::try_from(at).ok()?)? // ignore: idiomatic.arch/no-ok-on-result - a place past what usize holds names no text, and a row naming none refuses the whole store
                    .as_str()
                    .into(),
            )
        };
        let report = match store.head {
            None => KeptReport::Nothing,
            Some(head) => {
                let head = text(head).ok_or(())?;
                KeptReport::Kept(Arc::new(Report {
                    head: if *head == *DETACHED {
                        Head::Detached
                    } else {
                        Head::Commit(Revision(head))
                    },
                    scanned: scans(&store, &text).ok_or(())?,
                }))
            }
        };
        let mut files = BTreeMap::new();
        for row in &store.files {
            drop(files.insert(
                RepoPath(text(row.path).ok_or(())?),
                FileRecord {
                    size: Size(row.size),
                    modified: Moment(u128::from(row.modified)),
                    digest: Digest(text(row.digest).ok_or(())?),
                },
            ));
        }
        Ok(Self {
            report,
            recorded: Recorded {
                files,
                taken: Moment(u128::from(store.taken)),
            },
        })
    }
}

/// What the store's bytes hold; bytes that do not decode as a store, that another build wrote, or whose rows name a
/// text they do not hold, are none.
pub(super) fn read(bytes: &[u8], build: &Build) -> Option<Stored> {
    let store = postcard::from_bytes::<StoreSchema>(bytes).ok()?; // ignore: idiomatic.arch/no-ok-on-result - bytes that do not decode as a store are no store: it is rebuilt, never migrated
    if *store.build != *build.0 {
        return None;
    }
    Stored::try_from(store).ok() // ignore: idiomatic.arch/no-ok-on-result - rows that name a text the store does not hold are no store: it is rebuilt, never migrated
}

/// The store's bytes for `stored`, kept by `build`.
pub(super) fn write(stored: &Stored, build: &Build) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(&StoreSchema::from(KeptSchema(stored, build)))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
