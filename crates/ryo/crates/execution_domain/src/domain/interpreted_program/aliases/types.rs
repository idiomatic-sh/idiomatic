//! The types of the `aliases` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use super::super::types::RegionId;
use crate::domain::file_program::types::GraphObservations;
use crate::domain::operating::errors::OperatingError;

/// What the lineage of a region's inputs is resolved against.
pub(super) struct LineageSources<'a> {
    pub(crate) observation_regions: &'a [Result<RegionId, OperatingError>],
    pub(crate) named: Option<&'a GraphObservations>,
    pub(crate) observed_roots: &'a [Vec<Option<RegionId>>],
}
