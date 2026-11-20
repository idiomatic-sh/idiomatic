//! The constants of the `pack_load` operation: why the harness stops a run before any file.

/// Why the harness stops a run of the new engine at an edge that composed no engine.
pub const NO_ENGINE_COMPOSED: &str = "RYO_ENGINE is set and this edge composed no engine port";

/// Why the harness stops a run it cannot choose a kind vocabulary for.
pub const NO_ONE_LANGUAGE: &str = "RYO_ENGINE lowers against one linked language: link one, or name the request's source language";
