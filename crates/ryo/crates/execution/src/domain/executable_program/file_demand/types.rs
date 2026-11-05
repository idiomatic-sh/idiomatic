//! Immutable request-wide file root activation metadata.
use super::super::types::RegionId;

#[derive(Debug)]
pub(crate) enum FileRoot {
    Ordinary(RegionId),
    Observation { region: RegionId, output: usize },
}
#[derive(Debug)]
pub(crate) struct FileMode {
    pub readers: Box<[RegionId]>,
    pub roots: Box<[FileRoot]>,
    pub observations: Box<[usize]>,
}
#[derive(Debug)]
pub(crate) struct FileRuleDemand {
    pub outputs: usize,
    pub modes: Box<[FileMode]>,
}

/// Sparse immutable reverse obligations for each declared external root.
#[derive(Debug)]
pub(crate) struct FileReaderGraph {
    pub consumers: std::collections::HashMap<
        RegionId,
        std::collections::HashMap<RegionId, Box<[(RegionId, usize)]>>,
    >,
}
