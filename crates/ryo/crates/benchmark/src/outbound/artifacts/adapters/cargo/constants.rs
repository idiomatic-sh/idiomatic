//! The text this part uses.
/// The cargo verb that builds bench executables.
pub(super) const BENCH: &str = "bench";
/// Build without running.
pub(super) const NO_RUN: &str = "--no-run";
/// Refuse a lock file change.
pub(super) const LOCKED: &str = "--locked";
/// Machine-readable build messages.
pub(super) const MESSAGE_FORMAT: &str = "--message-format=json";
/// The option naming the bench's manifest.
pub(super) const MANIFEST_PATH: &str = "--manifest-path";
/// The bench's manifest file.
pub(super) const CARGO_TOML: &str = "Cargo.toml";
/// The suffix of a seal record beside its sealed executable.
pub(super) const SEAL_SUFFIX: &str = "-seal.json";
/// The refusal on a platform where a nested build waits on the outer one's lock.
pub(super) const OPERATIONS_RUN_ON_UNIX: &str = "bench operations run on Unix only: on this platform `cargo run` holds the build lock a nested build waits on";
/// The refusal of a failed build.
pub(super) const BUILD_FAILED: &str = "the bench worker build failed";
/// The refusal of a build that names no bench executable or several.
pub(super) const ONE_BENCH_EXECUTABLE: &str = "expected exactly one bench executable of the bench's package";
/// The refusal of a reference tool the search path does not hold.
pub(super) const TOOL_NOT_FOUND: &str = "reference tool not found on the search path";
/// The refusal of a child whose output was not captured.
pub(super) const NO_OUTPUT: &str = "the sealed executable's output was not captured";
/// Owner read, write and execute; group and others read and execute.
pub(super) const EXECUTABLE_MODE: u32 = 0o755;
