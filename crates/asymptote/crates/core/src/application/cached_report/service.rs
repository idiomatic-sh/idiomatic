//! Validates the retained report against the injected manifest and source inventory.

use std::sync::Arc;

use super::errors::CachedReportError;
use crate::domain::manifest::constants::MANIFEST;
use crate::domain::renames::types::Head;
use crate::domain::reports::service::{failures, moved, recorded, unread};
use crate::domain::reports::types::{FileDigest, Report};
use crate::domain::violations::types::RepoPath;
use crate::outbound::check_store::types::{KeptReport, Stored};
use crate::outbound::sources::interface::SourcesPort;

/// Checks freshness and shares the retained report; never saves or changes accepted state.
#[derive(Debug)]
pub struct CachedReport {
    /// Stamps the files whose digests tell whether the kept report is still current.
    sources: Arc<dyn SourcesPort>,
}

impl CachedReport {
    /// Bind the contract selected by composition.
    #[must_use]
    pub const fn new(sources: Arc<dyn SourcesPort>) -> Self {
        Self { sources }
    }

    /// The last report, only if its head, manifest, files and completed tool results still hold.
    pub(crate) fn current(
        &self,
        stored: &Stored,
        head: &Head,
    ) -> Result<Arc<Report>, CachedReportError> {
        let KeptReport::Kept(report) = &stored.report else {
            return Err(CachedReportError::NoReport);
        };
        if &report.head != head {
            return Err(CachedReportError::HeadMoved);
        }
        let paths: Vec<RepoPath> = stored.recorded.files.keys().cloned().collect();
        // A file is read only when its record cannot vouch for it: a file untouched since the check is not read.
        let stats = self.sources.stats(&paths);
        let read = self.sources.digests(&unread(&stored.recorded, &stats));
        let now: Vec<FileDigest> = recorded(&stored.recorded, &stats, &read.0)
            .files
            .into_iter()
            .map(|(path, record)| FileDigest {
                path,
                digest: record.digest,
            })
            .collect();
        let changed = moved(&stored.recorded, &now);
        if changed.iter().any(|path| *path.0 == *MANIFEST) {
            return Err(CachedReportError::ManifestMoved);
        }
        if let Some(first) = changed.first() {
            return Err(CachedReportError::Moved {
                first: first.0.to_string(),
                count: changed.len(),
            });
        }
        let failed = failures(report);
        if let Some(first) = failed.first() {
            return Err(CachedReportError::ToolsFailed {
                first: first.tool.0.to_string(),
                count: failed.len(),
            });
        }
        Ok(Arc::clone(report))
    }
}
