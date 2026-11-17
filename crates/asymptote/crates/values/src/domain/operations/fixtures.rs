//! The test support of the `operations` concept: the records and operations its tests fold, and the scenes that span
//! several folds.

use std::collections::BTreeMap;

use super::errors::OperationsError;
use super::service::{fold, settled, shape, snapshot};
use super::types::{
    BenchmarkName, Count, Day, Folded, MeasureName, Number, Operation, OperationId, PerfKey,
    PerfRecord, Provenance, Reason, Record, Removal, Shape,
};
use crate::domain::violations::types::{
    Fingerprint, Id, Key, Level, MessageText, RepoPath, RuleId, Standing, ToolId,
};

/// The value the tests use.
#[must_use]
pub fn record(count: u32, reason: &str) -> Record {
    Record {
        id: Id("a".into()),
        key: Key {
            tool: ToolId("t".into()),
            rule: RuleId("r".into()),
            path: RepoPath("p".into()),
            level: Level::Error,
            standing: Standing::Reported,
            fingerprint: Fingerprint(1),
        },
        count: Count(count),
        message: MessageText("m".into()),
        since_rev: Provenance::Unknown,
        since: Day("2026-10-19".into()),
        reasons: vec![Reason(reason.into())],
    }
}

/// Another record than `record`'s, in the same file.
#[must_use]
pub fn other(count: u32, reason: &str) -> Record {
    let mut other = record(count, reason);
    other.id = Id("b".into());
    other.key.fingerprint = Fingerprint(2);
    other
}

/// The value the tests use.
#[must_use]
pub fn id(text: &str) -> OperationId {
    OperationId(text.into())
}

/// The value the tests use.
#[must_use]
pub fn operation(parents: &[&str], accepted: Vec<Record>, removed: Vec<Record>) -> Operation {
    Operation {
        parents: parents.iter().map(|parent| id(parent)).collect(),
        covers: Vec::new(),
        accepted,
        removed: removed
            .into_iter()
            .map(|record| Removal {
                id: record.id,
                key: record.key,
            })
            .collect(),
        perf: Vec::new(),
        perf_removed: Vec::new(),
    }
}

/// The value the tests use.
#[must_use]
pub fn kept(folded: &Folded) -> String {
    folded
        .records
        .iter()
        .map(|record| {
            let reasons = record
                .reasons
                .iter()
                .map(|reason| &*reason.0)
                .collect::<Vec<_>>();
            format!("{}:{}:{}", record.id.0, record.count.0, reasons.join("|"))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Replaces every operation of `operations` with the snapshot of them that carries `records`, as `compact` does.
fn compacted(
    operations: &mut BTreeMap<OperationId, Operation>,
    name: &str,
    records: &[Record],
) -> Result<(), OperationsError> {
    let folded = fold(operations)?;
    let written = snapshot(operations, &folded, records);
    operations.retain(|held, _| !written.covers.contains(held));
    operations.extend([(id(name), written)]);
    Ok(())
}

/// Operation 0 accepts a record, 1 removes it, and 2 accepts it from 0: 1 and 2 are concurrent, so the removal wins,
/// as it must once 0 and 1 are replaced by a snapshot.
///
/// # Errors
///
/// The error of the operation the scene runs.
pub fn removal_across_snapshot() -> Result<String, OperationsError> {
    let mut operations: BTreeMap<OperationId, Operation> = [
        ("0", operation(&[], vec![record(1, "one")], Vec::new())),
        ("1", operation(&["0"], Vec::new(), vec![record(1, "one")])),
    ]
    .into_iter()
    .map(|(name, held)| (id(name), held))
    .collect();
    let branch = operation(&["0"], vec![record(1, "two")], Vec::new());
    let mut uncompacted = operations.clone();
    uncompacted.extend([(id("2"), branch.clone())]);
    compacted(&mut operations, "s", &[])?;
    operations.extend([(id("2"), branch)]);
    Ok(format!(
        "uncompacted:{} compacted:{}",
        kept(&fold(&uncompacted)?),
        kept(&fold(&operations)?)
    ))
}

/// Operations 0 and 1 are replaced by a snapshot, it and 2 by a second: an operation whose parent is 0 still folds.
///
/// # Errors
///
/// The error of the operation the scene runs.
pub fn coverage_across_snapshots() -> Result<String, OperationsError> {
    let mut operations: BTreeMap<OperationId, Operation> = [
        ("0", operation(&[], vec![record(1, "one")], Vec::new())),
        ("1", operation(&["0"], vec![other(1, "one")], Vec::new())),
    ]
    .into_iter()
    .map(|(name, held)| (id(name), held))
    .collect();
    let both = fold(&operations)?.records;
    compacted(&mut operations, "s1", &both)?;
    operations.extend([(
        id("2"),
        operation(&["s1"], Vec::new(), vec![other(1, "one")]),
    )]);
    let one = fold(&operations)?.records;
    compacted(&mut operations, "s2", &one)?;
    operations.extend([(
        id("3"),
        operation(&["0"], vec![record(1, "late")], Vec::new()),
    )]);
    Ok(kept(&fold(&operations)?))
}

/// 70 operations that accept a record and remove it again: whether the lock is due a snapshot by its shape and by its
/// count, and by its count right after one.
///
/// # Errors
///
/// The error of the operation the scene runs.
pub fn drained() -> Result<String, OperationsError> {
    let mut operations = BTreeMap::new();
    for at in 0..70_u32 {
        let parents: Vec<String> = at
            .checked_sub(1)
            .map(|before| format!("{before:03}"))
            .into_iter()
            .collect();
        let parents: Vec<&str> = parents.iter().map(String::as_str).collect();
        let held = if at % 2 == 0 {
            operation(&parents, vec![record(1, "one")], Vec::new())
        } else {
            operation(&parents, Vec::new(), vec![record(1, "one")])
        };
        operations.extend([(id(&format!("{at:03}")), held)]);
    }
    let folded = fold(&operations)?;
    let word = |shape: Shape| match shape {
        Shape::Compacted => "compacted",
        Shape::Open => "open",
    };
    let said = format!(
        "{} {}",
        word(shape(&operations, &folded)),
        word(settled(&operations))
    );
    compacted(&mut operations, "z", &[])?;
    Ok(format!("{said} {}", word(settled(&operations))))
}

/// Writes a chain of 70 operations into `operations`, each accepting a record of its own, then a snapshot, then 64
/// more: whether the lock is due a snapshot before the snapshot, right after it, 63 operations after it and 64 after.
///
/// # Errors
///
/// The error of the operation the scene runs.
pub fn since(operations: &mut BTreeMap<OperationId, Operation>) -> Result<String, OperationsError> {
    let numbered = |at: u32| id(&format!("{at:03}"));
    let state =
        |graph: &BTreeMap<OperationId, Operation>| -> Result<&'static str, OperationsError> {
            Ok(match settled(graph) {
                Shape::Compacted => "compacted",
                Shape::Open => "open",
            })
        };
    let mut said = Vec::new();
    for at in 0..134_u32 {
        let parents: Vec<OperationId> = at.checked_sub(1).map(numbered).into_iter().collect();
        let added = if at == 70 {
            said.push(state(operations)?);
            let folded = fold(operations)?;
            // The snapshot replaces every operation before it, whose files it deletes.
            let written = snapshot(operations, &folded, &folded.records);
            operations.retain(|name, _| !written.covers.contains(name));
            written
        } else {
            let mut accepted = record(1, "one");
            accepted.id = Id(format!("r{at}").into());
            accepted.key.fingerprint = Fingerprint(u64::from(at));
            Operation {
                parents,
                ..operation(&[], vec![accepted], Vec::new())
            }
        };
        operations.extend([(numbered(at), added)]);
        if at == 70 || at == 132 {
            said.push(state(operations)?);
        }
    }
    said.push(state(operations)?);
    Ok(said.join(" "))
}

/// One accepted record whose count alternates between 1 and 2 over 70 operations, each the child of the one before: whether
/// the lock is due a snapshot by its shape and by its count, and by its count right after one.
///
/// # Errors
///
/// The error of the operation the scene runs.
pub fn flickering() -> Result<String, OperationsError> {
    let mut operations = BTreeMap::new();
    for at in 0..70_u32 {
        let parents: Vec<String> = at
            .checked_sub(1)
            .map(|before| format!("{before:03}"))
            .into_iter()
            .collect();
        let parents: Vec<&str> = parents.iter().map(String::as_str).collect();
        let count = if at % 2 == 0 { 1 } else { 2 };
        operations.extend([(
            id(&format!("{at:03}")),
            operation(&parents, vec![record(count, "one")], Vec::new()),
        )]);
    }
    let folded = fold(&operations)?;
    let word = |shape: Shape| match shape {
        Shape::Compacted => "compacted",
        Shape::Open => "open",
    };
    let said = format!(
        "{} {}",
        word(shape(&operations, &folded)),
        word(settled(&operations))
    );
    compacted(&mut operations, "z", &folded.records)?;
    let after = fold(&operations)?;
    Ok(format!(
        "{said} {} {}",
        word(shape(&operations, &after)),
        word(settled(&operations))
    ))
}

/// A perf record `best` of the one key the perf scenes use.
fn perf(best: f64) -> PerfRecord {
    PerfRecord {
        key: perf_key(),
        best: Number(best),
        reasons: vec![Reason("one".into())],
    }
}

/// The key the perf scenes use.
fn perf_key() -> PerfKey {
    PerfKey {
        tool: ToolId("t".into()),
        benchmark: BenchmarkName("b".into()),
        measure: MeasureName("m".into()),
    }
}

/// The perf bests a fold kept, joined.
fn bests(folded: &Folded) -> String {
    folded
        .perf
        .iter()
        .map(|record| record.best.0.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Operation 0 sets a perf record at 100, 1 removes it, and 2 sets it at 200 from 0: 1 and 2 are concurrent, so the removal wins,
/// as it must once 0 and 1 are replaced by a snapshot.
///
/// # Errors
///
/// The error of the operation the scene runs.
pub fn perf_removal_across_snapshot() -> Result<String, OperationsError> {
    let set = |parents: &[&str], best: f64| Operation {
        perf: vec![perf(best)],
        ..operation(parents, Vec::new(), Vec::new())
    };
    let mut operations: BTreeMap<OperationId, Operation> = [
        ("0", set(&[], 100.0_f64)),
        (
            "1",
            Operation {
                perf_removed: vec![perf_key()],
                ..operation(&["0"], Vec::new(), Vec::new())
            },
        ),
    ]
    .into_iter()
    .map(|(name, held)| (id(name), held))
    .collect();
    let branch = set(&["0"], 200.0_f64);
    let mut uncompacted = operations.clone();
    uncompacted.extend([(id("2"), branch.clone())]);
    compacted(&mut operations, "s", &[])?;
    operations.extend([(id("2"), branch)]);
    Ok(format!(
        "uncompacted:{} compacted:{}",
        bests(&fold(&uncompacted)?),
        bests(&fold(&operations)?)
    ))
}
