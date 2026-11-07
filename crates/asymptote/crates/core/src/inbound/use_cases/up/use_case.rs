//! The `up` use case: the last check's new violations, those the selectors choose, recorded in one operation with
//! the reason. No tool runs: the check's report is read, and refused when a file it fingerprinted has changed.

use std::collections::BTreeSet;
use std::sync::Arc;

use super::errors::UpError;
use super::interface::UpUseCase;
use super::service::seeded;
use super::types::{UpReport, UpRequest};
use crate::application::accepted_state::service::AcceptedState;
use crate::application::cached_report::service::CachedReport;
use crate::domain::judgement::service::compare;
use crate::domain::operations::constants::COMPACT_AT;
use crate::domain::operations::service::{fold, settled, shape, snapshot};
use crate::domain::operations::types::{Operation, Provenance, Shape};
use crate::domain::ratchet::service::accepting;
use crate::domain::ratchet::types::{Compaction, Writing};
use crate::domain::renames::service::{changes, split};
use crate::domain::renames::types::{Carried, Head, Projection};
use crate::domain::reports::service::observed;
use crate::domain::reports::types::Observed;
use crate::domain::selection::service::{chosen, nearest, unmatched};
use crate::domain::selection::types::Choice;
use crate::domain::violations::service::{keys, within};
use crate::domain::violations::types::{Key, Violation, Within};
use crate::outbound::check_store::interface::CheckStorePort;
use crate::outbound::clock::interface::ClockPort;
use crate::outbound::lock::interface::LockPort;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::matching::interface::MatchingPort;
use crate::outbound::repository::interface::RepositoryPort;

/// The ports `up` reaches the outside through.
#[derive(Debug)]
pub struct UpPorts {
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
    /// Today.
    pub clock: Arc<dyn ClockPort>,
    /// The `--path` globs.
    pub matching: Arc<dyn MatchingPort>,
}

/// `up`, over the ports the edge injects.
#[derive(Debug)]
pub struct UpUseCaseImpl {
    /// The outside `up` reaches: the manifest, the lock, the kept report, the cache, git, the clock and globs.
    ports: UpPorts,
}

impl UpUseCaseImpl {
    /// `up` over the ports the edge injects.
    #[must_use]
    pub const fn new(ports: UpPorts) -> Self {
        Self { ports }
    }

    /// Replaces the lock's operations with a snapshot once it holds `COMPACT_AT` of them, when every rename
    /// projection is carried, so a snapshot loses none: the lock's files stay bounded, and a snapshot is written every
    /// `COMPACT_AT` operations, never with every write.
    fn compact(&self, carried: &Carried) -> Result<Compaction, UpError> {
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

impl UpUseCase for UpUseCaseImpl {
    fn up(&self, request: UpRequest) -> Result<UpReport, UpError> {
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
        let mut picked: Vec<Violation> = Vec::new();
        let mut left: Vec<Violation> = Vec::new();
        for violation in &outcome.new {
            let path = self
                .ports
                .matching
                .matches(&request.selectors.paths, &violation.path)?;
            if chosen(violation, &request.selectors, path) == Choice::Chosen {
                picked.push(violation.clone());
            } else {
                left.push(violation.clone());
            }
        }
        if let Some(at) = unmatched(&picked, &request.selectors.at).first() {
            return Err(UpError::NothingAt {
                at: (*at).clone(),
                nearest: nearest(&outcome.new, at),
            });
        }
        if let Some(outside) = picked
            .iter()
            .find(|violation| within(&violation.path) == Within::Outside)
        {
            return Err(UpError::Outside {
                path: outside.path.0.to_string(),
            });
        }
        let revision = match &head {
            Head::Commit(revision) => Provenance::At(revision.clone()),
            Head::Detached => Provenance::Unknown,
        };
        let lock = &self.ports.lock;
        let made = accepting(
            &records,
            &picked,
            &revision,
            &self.ports.clock.today(),
            Some(&request.reason),
            &|key| lock.id(key),
        );
        if made.unkeyed.0 > 0 {
            return Err(UpError::Unkeyed {
                count: made.unkeyed.0,
            });
        }
        let seen: BTreeSet<Key> = keys(&violations);
        // Accepting new violations is no occasion to drop anything: a projection with no violation at its new path
        // stays where it is, for `down` to resolve.
        let carried = Carried {
            unmatched: Vec::new(),
            ..split(&migrations, &seen, &outcome)
        };
        let persisted = changes(&carried, made.records.clone(), Vec::new());
        let perf = seeded(&request, &folded, &report);
        let mut compaction = Compaction::Unchanged;
        if request.writing == Writing::Write
            && !(persisted.added.is_empty() && persisted.removed.is_empty() && perf.is_empty())
        {
            self.ports.lock.append(&Operation {
                parents: folded.heads.clone(),
                covers: Vec::new(),
                accepted: persisted.added,
                removed: persisted.removed,
                perf: perf.clone(),
                perf_removed: Vec::new(),
            })?;
            compaction = self.compact(&split(&migrations, &seen, &outcome))?;
        }
        Ok(UpReport {
            root,
            recorded: made.records,
            perf,
            left,
            compaction,
            writing: request.writing,
        })
    }
}
