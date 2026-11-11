//! The external root exit scenario.
use super::super::super::fixtures::scenario::Delivery;
use super::*;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::row_emission::types::RowEmission;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use std::borrow::Cow;
/// What a second, alias or capture reader observed of the external root.
#[derive(Debug, PartialEq)]
pub(in super::super) enum Alias {
    /// It delivered rows, the same as the transferred rows or not.
    Rows { same: bool },
    /// It was refused before running.
    Refused(OperatingError),
    /// It answered another outcome.
    Other(Delivery),
}

/// The second delivery of rule 0: its refusal, the rows its sink saw, and a retry's rows after a
/// refusal with whether that retry's emission was complete.
#[derive(Debug, PartialEq)]
pub(in super::super) struct Delivered {
    pub(in super::super) refusal: Option<OperatingError>,
    pub(in super::super) callbacks: usize,
    pub(in super::super) retried: Option<(usize, bool)>,
}

/// The owner's retained state: working and owned bytes, and whether the equality backing is released.
#[derive(Debug, PartialEq)]
pub(in super::super) struct Finished {
    pub(in super::super) working: usize,
    pub(in super::super) owned: usize,
    pub(in super::super) backing_released: bool,
}

/// What the capture rule observed on each of its two capture passes, and its repeated collected failures.
#[derive(Debug, PartialEq)]
pub(in super::super) struct Captured {
    pub(in super::super) failures: Cow<'static, [Delivery]>,
    /// The frontiers each pass captured.
    pub(in super::super) frontiers: Cow<'static, [usize]>,
    /// Whether every capture decoded to the transferred rows.
    pub(in super::super) decodes_transferred: bool,
    /// Whether every capture and commit left the backing, bytes, readiness, builds and reads unchanged.
    pub(in super::super) quiet: bool,
}

/// What the shared external root observed from preparation to the file's finish.
#[derive(Debug, PartialEq)]
pub(in super::super) struct External {
    /// Whether rules 0 and 1 compile to one emit root.
    pub(in super::super) shared_root: bool,
    /// How often the root is queued, and its remaining readers before anything ran.
    pub(in super::super) queued: usize,
    pub(in super::super) readers: usize,
    /// The rows rule 0 delivered.
    pub(in super::super) transferred_rows: usize,
    /// Whether that first emission was complete.
    pub(in super::super) emitted_complete: bool,
    /// The root's attempt and remaining readers after the first delivery.
    pub(in super::super) published: (AttemptCheckpoint, usize),
    /// Whether a required equality lease is charged and its backing alive.
    pub(in super::super) holds_required: bool,
    pub(in super::super) backing_alive: bool,
    /// Equality builds, and whether a later reader hit the shared backing.
    pub(in super::super) builds: (u64, bool),
    pub(in super::super) second: Delivered,
    pub(in super::super) alias: Alias,
    pub(in super::super) final_consumer: Delivery,
    /// Whether all later delivery left readiness, reads, builds, bytes and the checkpoint unchanged.
    pub(in super::super) replay_quiet: bool,
    pub(in super::super) capture: Option<Captured>,
    /// Whether the file's finish answered a frontier readiness.
    pub(in super::super) frontier_readiness: bool,
    pub(in super::super) finished: Finished,
}

/// Two diagnostic obligations share an external Union root. Distinct Join
/// consumers acquire the real producer's equality lease during the same batch.
/// Unlike an internal-only producer, the externally held Union must stay live
/// after both Join readers release and until the file ends.
pub(in super::super) fn external_root_exit(
    exit: ExternalRootExit,
) -> Result<External, OperatingError> {
    let PreparedCase {
        prepared,
        services,
        intents,
        capture_rule,
    } = prepare_case(exit)?;
    let root = |rule: RuleIndex| *prepared.program().rules()[rule.0].emit.as_ref().unwrap();
    let producer = root(RuleIndex(0));
    let shared_root = producer == root(RuleIndex(1));
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &intents,
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )?;
    file.owner.derived().set_optional_limit(
        ryo_execution_store::application::file_dag::derived::owner::index::types::ByteLimit(0),
    )?;
    // Deduplication applies to queue entries, not external reader obligations.
    let queued = file
        .prepared_roots
        .iter()
        .filter(|&&id| id == producer)
        .count();
    let readers = file
        .execution
        .retry_checkpoint(file.execution.file_state(), producer)?
        .remaining
        .0;
    let columns = [
        FactColumn(vec![to_bits(Cell::File(FileId(0))).0; 2]),
        FactColumn(vec![
            to_bits(Cell::Number(Integer(2))).0,
            to_bits(Cell::Number(Integer(3))).0,
        ]),
        FactColumn(vec![
            to_bits(Cell::Number(Integer(9))).0,
            to_bits(Cell::Number(Integer(7))).0,
        ]),
    ];
    let reads = std::cell::Cell::new(0);
    let read = |relation| {
        if relation == Relation::LanguageTarget {
            reads.set(reads.get() + 1);
            Ok(Rows::Stored(ColumnsView {
                kinds: kinds(relation),
                columns: &columns,
            }))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let demand = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    let mut cells = Vec::new();
    let emitted_complete = matches!(
        file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |row| {
            cells.extend_from_slice(row);
            Ok(())
        })?,
        RowEmission::Complete
    );
    let transferred = Table {
        kinds: kinds(Relation::LanguageTarget).to_vec(),
        count: RowCount(cells.len() / 3),
        cells,
    };
    let checkpoint = file
        .execution
        .retry_checkpoint(file.execution.file_state(), producer)?;
    let required = file.owner.derived().working_bytes();
    let backing = file
        .owner
        .derived()
        .view_witness(ryo_derived::domain::request::types::DerivedKind::Equality)
        .expect("actual equality backing");
    let backing_alive = backing.liveness() == BackingLiveness::Alive;
    let builds = file.owner.derived().counts();
    let counts = file.readiness_counts();
    let source_reads = reads.get();
    let refuses = matches!(
        exit,
        ExternalRootExit::RejectedDelivery | ExternalRootExit::CaptureCallbackRefusal
    );
    let mut callbacks = 0;
    let delivery = file
        .emit_rows(RuleIndex(0), &read, &mut symbols, &mut |_| {
            callbacks += 1;
            if refuses {
                Err(OperatingError::WrongBinding)
            } else {
                Ok(())
            }
        })
        .err();
    let retried = if refuses {
        let mut repeated = 0;
        let complete = matches!(
            file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |_| {
                repeated += 1;
                Ok(())
            })?,
            RowEmission::Complete
        );
        Some((repeated, complete))
    } else {
        None
    };
    let alias = file
        .rule(RuleIndex(1), demand, &read, &mut symbols)
        .map_or_else(Alias::Refused, |alias| match alias.emit {
            RootOutcome::Complete(alias) => Alias::Rows {
                same: alias == transferred,
            },
            other => Alias::Other(Delivery::from(other)),
        });
    let final_consumer = Delivery::from(file.rule(RuleIndex(3), demand, &read, &mut symbols)?.emit);
    let replay_quiet = file.readiness_counts() == counts
        && reads.get() == source_reads
        && file.owner.derived().counts() == builds
        && file.owner.derived().working_bytes() == required
        && file
            .execution
            .retry_checkpoint(file.execution.file_state(), producer)?
            == checkpoint;
    let capture = match capture_rule {
        Some(capture_rule) => {
            let mut failures = vec![];
            if matches!(exit, ExternalRootExit::CaptureCollectedFailure) {
                for _ in 0..2 {
                    let failed = file.rule(capture_rule, demand, &read, &mut symbols)?;
                    failures.push(Delivery::from(failed.emit));
                }
            }
            let mut arena = vec![];
            let mut places = vec![];
            let mut words = ryo_relations::domain::relations::types::Words::default();
            let mut frontiers = vec![];
            let mut decodes_transferred = true;
            let mut quiet = true;
            for _ in 0..2 {
                let mut captures = file.capture_frontiers(
                    capture_rule,
                    ryo_values::domain::stored_facts::types::Bucket(0),
                    &read,
                    &mut symbols,
                    ryo_execution_domain::domain::frontier_capture::types::CaptureScope {
                        file: FileId(0),
                        rule_digest: Digest([0; 16]),
                        worker: ryo_relations::domain::relations::types::WorkerNumber(0),
                        retained: ryo_execution_domain::domain::finish::types::Retained {
                            arena: ryo_execution_domain::domain::finish::types::Arena(&mut arena),
                            places: &mut places,
                        },
                        words: &mut words,
                    },
                )?;
                frontiers.push(captures.len());
                let captured = captures
                    .pop()
                    .expect("one declared physical frontier obligation");
                let input = captured.input;
                let captured = captured.outcome?;
                let decoded = ryo_values::domain::table_words::service::decode_complete(
                    &captured.words,
                    &mut vec![],
                )
                .expect("actual captured canonical publication decodes");
                decodes_transferred &= decoded == transferred;
                file.commit_frontier(capture_rule, input)?;
                quiet &= backing.liveness() == BackingLiveness::Alive
                    && file.owner.derived().working_bytes() == required
                    && file.readiness_counts() == counts
                    && file.owner.derived().counts() == builds
                    && reads.get() == source_reads;
            }
            Some(Captured {
                failures: Cow::Owned(failures),
                frontiers: Cow::Owned(frontiers),
                decodes_transferred,
                quiet,
            })
        }
        None => None,
    };
    // Retain the owner solely to inspect real byte accounting after finish. A
    // Weak-only check would miss a stale cache retaining its required charge.
    let owner = Rc::clone(&file.owner);
    let frontier_readiness = file.finish()?.is_some();
    Ok(External {
        shared_root,
        queued,
        readers,
        transferred_rows: transferred.count.0,
        emitted_complete,
        published: (checkpoint.attempt, checkpoint.remaining.0),
        holds_required: required > 0,
        backing_alive,
        builds: (builds.0.0, builds.1.0 > 0),
        second: Delivered {
            refusal: delivery,
            callbacks,
            retried,
        },
        alias,
        final_consumer,
        replay_quiet,
        capture,
        frontier_readiness,
        finished: Finished {
            working: owner.derived().working_bytes().0,
            owned: owner.derived().owned_bytes().0,
            backing_released: backing.liveness() == BackingLiveness::Released,
        },
    })
}
