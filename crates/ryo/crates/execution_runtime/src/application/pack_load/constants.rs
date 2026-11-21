//! The constants of the `pack_load` operation: why the harness stops a run before any file.

/// Why the harness stops a run of the new engine at an edge that composed no engine.
pub const NO_ENGINE_COMPOSED: &str = "RYO_ENGINE is set and this edge composed no engine port";

/// Why the harness stops a run that has no language to lower the rules against.
pub const NO_LANGUAGE: &str = "RYO_ENGINE lowers the rules against each linked language: none is linked, or the one the request names is not";

/// What a load that read statistics says of them, around how many units of its programs they name.
pub const STATISTICS_MATCHED: [&str; 2] = ["statistics matched ", " units"];
