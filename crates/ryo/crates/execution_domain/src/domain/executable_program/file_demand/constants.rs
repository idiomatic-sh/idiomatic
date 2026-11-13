//! The constants of the file demand.
use super::types::{FileDemand, FrontierSelection, RootSelection};

/// The demand of a file asked for nothing: no diagnostic or observation root, no frontier captured.
pub const NO_FILE_DEMAND: FileDemand = FileDemand {
    diagnostic: RootSelection::Undemanded,
    observations: RootSelection::Undemanded,
    capture: FrontierSelection::Disabled,
};
