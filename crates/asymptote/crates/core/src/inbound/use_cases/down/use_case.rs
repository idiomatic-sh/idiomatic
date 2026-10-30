//! The `down` use case: every record the last check found fixed, dropped or lowered in one operation, each new
//! violation an in-source ignore suppresses recorded as suppressed, each rename projection the check cannot carry
//! dropped from its old path, and each record whose code moved to another file carried there. No tool runs: the check's report is read, and refused when a file it fingerprinted has changed.

use std::collections::BTreeSet;
use std::sync::Arc;

use super::errors::DownError;
use super::interface::DownUseCase;
use super::types::{DownReport, DownRequest};
use crate::application::accepted_state::service::AcceptedState;
use crate::application::cached_report::service::CachedReport;
use crate::domain::judgement::service::compare;
use crate::domain::judgement::types::Status;
use crate::domain::operations::constants::COMPACT_AT;
use crate::domain::operations::service::{fold, settled, shape, snapshot};
use crate::domain::operations::types::{Operation, Provenance, Shape};
use crate::domain::perf::service::resolved;
use crate::domain::ratchet::service::{accepting, lowering};
use crate::domain::ratchet::types::{Compaction, Writing};
use crate::domain::renames::service::{relocate, split};
use crate::domain::renames::types::{Carried, Head, Projection};
use crate::domain::reports::service::{measured, observed};
use crate::domain::reports::types::Observed;
use crate::domain::violations::service::{keys, within};
use crate::domain::violations::types::{Key, Suppression, Violation, Within};
use crate::outbound::check_store::interface::CheckStorePort;
use crate::outbound::clock::interface::ClockPort;
use crate::outbound::lock::interface::LockPort;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::repository::interface::RepositoryPort;

/// The ports `down` reaches the outside through.
#[derive(Debug)]
pub struct DownPorts {
    /// The product's `asymptote.toml`.
    pub manifest: Arc<dyn ManifestPort>,
    /// The lock.
    pub lock: Arc<dyn LockPort>,
    /// Preparation of accepted state over the injected operation and history ports.
    pub accepted: AcceptedState,
    /// Freshness of the retained report over the injected input ports.
    pub report: CachedReport,
    /// The kept report and fold.
    pub store: Arc<dyn CheckStorePort>,
    /// Git.
    pub repository: Arc<dyn RepositoryPort>,
    /// Today, the day a new ignore is recorded on.
    pub clock: Arc<dyn ClockPort>,
}

/// `down`, over the ports the edge injects.
#[derive(Debug)]
pub struct DownUseCaseImpl {
    /// The outside `down` reaches: the manifest, the lock, the kept report, the cache and git.
    ports: DownPorts,
}

impl DownUseCaseImpl {
    /// `down` over the ports the edge injects.
    #[must_use]
    pub const fn new(ports: DownPorts) -> Self {
        Self { ports }
    }

    /// Replaces the lock's operations with a snapshot once it holds `COMPACT_AT` of them, when every rename
    /// projection is carried, so a snapshot loses none: the lock's files stay bounded, and a snapshot is written every
    /// `COMPACT_AT` operations, never with every write.
    fn compact(&self, carried: &Carried) -> Result<Compaction, DownError> {
        if !carried.unmatched.is_empty() || self.ports.lock.listing()?.files.len() < COMPACT_AT {
            return Ok(Compaction::Unchanged);
        }
        let operations = self.ports.lock.read()?.0;
        let folded = fold(&operations)?;
        if shape(&operations, &folded) == Shape::Compacted
            || settled(&operations) == Shape::Compacted
        {
            return Ok(Compaction::Unchanged);
        }
        self.ports
            .lock
            .compact(&snapshot(&operations, &folded, &folded.records))?;
        Ok(Compaction::Snapshot)
    }
}

impl DownUseCase for DownUseCaseImpl {
    fn down(&self, request: DownRequest) -> Result<DownReport, DownError> {
        let root = self.ports.manifest.find(&request.start)?;
        let stored = self.ports.store.load();
        let head = self.ports.repository.head()?;
        let report = self.ports.report.current(&stored, &head)?;
        let folded = self.ports.accepted.folded()?;
        let Projection {
            records,
            migrations,
        } = self.ports.accepted.projected(&head, &folded)?;
        let Observed { violations, runs } = observed(&report);
        let outcome = compare(&violations, &records, &runs);
        let seen: BTreeSet<Key> = keys(&violations);
        let carried = relocate(
            split(&migrations, &seen, &outcome),
            &outcome,
            &records,
            &head,
            &|key| self.ports.lock.id(key),
        );
        let lowered = lowering(&outcome, &carried);
        let persisted = &lowered.changes;
        // A new violation an in-source ignore suppresses is resolved: it is recorded, as suppressed, with the drained.
        let ignored: Vec<Violation> = outcome
            .new
            .iter()
            .filter(|violation| {
                matches!(violation.suppression, Suppression::Suppressed(_))
                    && within(&violation.path) == Within::Inside
            })
            .cloned()
            .collect();
        let revision = match &head {
            Head::Commit(revision) => Provenance::At(revision.clone()),
            Head::Detached => Provenance::Unknown,
        };
        let lock = &self.ports.lock;
        let recorded = accepting(
            &records,
            &ignored,
            &revision,
            &self.ports.clock.today(),
            None,
            &|key| lock.id(key),
        )
        .records;
        let ran: Vec<_> = runs
            .iter()
            .filter(|run| run.status == Status::Completed)
            .map(|run| run.tool.clone())
            .collect();
        let perf = resolved(&measured(&report.scanned), &folded.perf, &ran);
        let mut compaction = Compaction::Unchanged;
        if request.writing == Writing::Write
            && !(persisted.added.is_empty()
                && persisted.removed.is_empty()
                && perf.is_empty()
                && recorded.is_empty())
        {
            self.ports.lock.append(&Operation {
                parents: folded.heads.clone(),
                covers: Vec::new(),
                accepted: persisted.added.iter().chain(&recorded).cloned().collect(),
                removed: persisted.removed.clone(),
                perf: Vec::new(),
                perf_removed: perf.iter().map(|record| record.key.clone()).collect(),
            })?;
            // The projections the run cannot carry were just dropped, so a snapshot loses none.
            compaction = self.compact(&Carried::default())?;
        }
        Ok(DownReport {
            root,
            dropped: lowered.dropped,
            lowered: lowered.lowered,
            recorded,
            perf,
            compaction,
            writing: request.writing,
        })
    }
}
