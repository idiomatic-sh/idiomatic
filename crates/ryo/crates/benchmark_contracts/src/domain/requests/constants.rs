//! Request coordinate refusal vocabulary.
pub(super) const INVALID_SHARD: &str = "invalid case shard coordinates";
/// The applicability policy under which a rule runs on every file.
pub const ALWAYS: &str = "always";
/// The applicability policy under which a rule is skipped on a file the parser rejects.
pub const ACCEPTED_SYNTAX: &str = "accepted-syntax";
/// The filename of a rule's query inside its rule directory.
pub const QUERY_FILE: &str = "query.ryoql";
/// The module source a logical module name is probed with before the real source is read.
pub const MODULE_NAME_PROBE: &str = "module source";
/// The filename of a profile's manifest inside its profile directory.
pub const PROFILE_MANIFEST: &str = "benchmark.toml";
/// The worker argument naming the request file the parent wrote for it.
pub const REQUEST_FILE: &str = "--request-file";
