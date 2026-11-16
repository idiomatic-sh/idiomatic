//! The fake of the `check_store` port: what the store keeps, in memory.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use crate::outbound::check_store::errors::CheckStoreError;
use crate::outbound::check_store::interface::CheckStorePort;
use crate::outbound::check_store::types::{KeptReport, Stored};
use asymptote_values::domain::judgement::types::{Inventory, Run, Status};
use asymptote_values::domain::manifest::types::Pathspec;
use asymptote_values::domain::operations::types::{BenchmarkName, MeasureName, Number, Revision};
use asymptote_values::domain::perf::types::{Measurement, Spread};
use asymptote_values::domain::renames::types::Head;
use asymptote_values::domain::reports::types::{
    Digest, FailureOutput, FileRecord, Keeping, Moment, NamedInput, Recorded, Report, Scanned,
    Size, Stamped,
};
use asymptote_values::domain::violations::types::{
    ColumnNumber, Fingerprint, Fingerprinted, Justification, Level, LineNumber, MessageText, Place,
    RepoPath, RuleId, Suppression, ToolId, Violation,
};

/// What the core's tests stand in for `check_store` with.
#[derive(Debug, Default)]
pub struct CheckStoreFake {
    /// What is kept.
    pub stored: Mutex<Stored>,
    /// The number of writes, including ones that repeat what is kept.
    pub saves: std::sync::atomic::AtomicUsize,
    /// Whether every write is refused, as a store directory that cannot be written refuses it.
    pub refusing: bool,
}

impl CheckStoreFake {
    /// What is kept, read.
    #[must_use]
    pub fn kept(&self) -> Stored {
        self.stored
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl CheckStorePort for CheckStoreFake {
    fn load(&self) -> Stored {
        self.kept()
    }

    fn keep(&self, stored: &Stored) -> Result<(), CheckStoreError> {
        if self.refusing {
            return Err(CheckStoreError::Unwritable {
                directory: "/fake".to_owned(),
                reason: "the fake refuses every write".to_owned(),
            });
        }
        let _saves_before = self
            .saves
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *self
            .stored
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = stored.clone();
        Ok(())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;

/// A checked product as a store keeps it: a tool's completed run at a commit, its violations (one at a line and column,
/// one of a whole file, suppressed), its named input, a measurement, and the record of each file it stamped.
#[must_use]
pub fn checked() -> Stored {
    let tool = ToolId("lint".into());
    let violation = |path: &str, place, rule: &str, suppression| Violation {
        path: RepoPath(path.into()),
        place,
        tool: tool.clone(),
        rule: RuleId(rule.into()),
        level: Level::Error,
        suppression,
        message: MessageText(format!("{rule} in {path}").into()),
        fingerprint: Fingerprinted::Known(Fingerprint(7)),
    };
    let violations = vec![
        violation(
            "src/a.rs",
            Place::At(LineNumber(1), ColumnNumber(4)),
            "no-a",
            Suppression::Unsuppressed,
        ),
        violation(
            "src/b.rs",
            Place::File,
            "no-b",
            Suppression::Suppressed(Justification("kept on purpose".into())),
        ),
    ];
    let reported: BTreeSet<RuleId> = violations.iter().map(|each| each.rule.clone()).collect();
    let scanned = Scanned {
        failure_output: FailureOutput::Unavailable,
        run: Run {
            tool: tool.clone(),
            status: Status::Completed,
            inventory: Inventory::Declared(reported.clone()),
            reported,
        },
        violations,
        stamp: Stamped::Digested(Digest("stamp".into())),
        inputs: vec![NamedInput {
            spec: Pathspec("src".into()),
            digest: Digest("inputs".into()),
        }],
        measurements: vec![Measurement {
            benchmark: BenchmarkName("check".into()),
            measure: MeasureName("wall-ms".into()),
            value: Number(12.5),
            lower: Spread::Given(Number(11.0)),
            upper: Spread::Absent,
        }],
        keeping: Keeping::Current,
    };
    let record = |digest: &str| FileRecord {
        size: Size(10),
        modified: Moment(1_000),
        digest: Digest(digest.into()),
    };
    Stored {
        report: KeptReport::Kept(Arc::new(Report {
            head: Head::Commit(Revision("0123abcd".into())),
            scanned: vec![scanned],
        })),
        recorded: Recorded {
            files: BTreeMap::from([
                (RepoPath("src/a.rs".into()), record("a")),
                (RepoPath("src/b.rs".into()), record("b")),
            ]),
            taken: Moment(2_000),
        },
    }
}
