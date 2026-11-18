//! The text this part uses.
/// The build verb that builds bench executables.
pub(super) const BENCH: &str = "bench";
/// Build without running.
pub(super) const NO_RUN: &str = "--no-run";
/// Refuse a lock file change.
pub(super) const LOCKED: &str = "--locked";
/// Machine-readable build messages.
pub(super) const MESSAGE_FORMAT: &str = "--message-format=json";
/// The suffix of a seal record beside its sealed executable.
pub(super) const SEAL_SUFFIX: &str = "-seal.json";
/// The refusal on a platform where a nested build waits on the outer one's lock.
pub(super) const OPERATIONS_RUN_ON_UNIX: &str = "bench operations run on Unix only: on this platform the outer build holds the lock a nested build waits on";
/// The refusal of a failed build.
pub(super) const BUILD_FAILED: &str = "the bench worker build failed";
/// The refusal of a build that names no bench executable or several.
pub(super) const ONE_BENCH_EXECUTABLE: &str =
    "expected exactly one bench executable of the bench's package";
/// The refusal of a reference tool the search path does not hold.
pub(super) const TOOL_NOT_FOUND: &str = "reference tool not found on the search path";
/// The shell a child writes a sealed executable with.
#[cfg(unix)]
pub(super) const SHELL: &str = "/bin/sh";
/// What that child runs: it copies the built executable (`$1`) to the sealed copy (`$2`) and makes it executable (0755:
/// owner read, write and execute; group and others read and execute). A child writes it, so no descriptor this
/// multi-threaded process holds for writing can leak into a child another thread forks, and the launch that follows never
/// fails with "Text file busy" (ETXTBSY).
#[cfg(unix)]
pub(super) const COPY_EXECUTABLE: [&str; 3] = ["-c", "cat \"$1\" > \"$2\" && chmod 755 \"$2\"", "sh"];
/// The refusal of a child whose output was not captured.
pub(super) const NO_OUTPUT: &str = "the sealed executable's output was not captured";
/// The size of one read of a child's output.
pub(super) const CHUNK: usize = 8192;
