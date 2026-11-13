//! Static native-Python children and platform-owned launch fixture setup.

/// Existing CI interpreter convention; absence is a fixture failure, never a skipped test.
pub(super) fn python() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
}

/// Controlled exact streams, without shell quoting, pipelines or newline conversion.
const STREAMS: &str =
    "import os,sys;sys.stdout.write(os.environ['RYO_BENCH_FIXTURE']);sys.stderr.write('issue')";

/// A native child reports the search path it inherited.
pub(super) const PATH: &str = "import os,sys;sys.stdout.write(os.environ['PATH'])";

/// A native child writes more than one pipe buffer to each binary stream.
pub(super) const LARGE_STREAMS: &str = "import sys;sys.stdout.buffer.write(b'\\0' * 131_072);sys.stdout.buffer.flush();sys.stderr.buffer.write(b'\\0' * 131_072);sys.stderr.buffer.flush();sys.exit(0)";

/// On Unix this kills the actual Python child, not a shell whose exit code encodes a signal.
pub(super) const SIGNAL: &str = "import os,signal;os.kill(os.getpid(),signal.SIGTERM)";

/// Run one static fixture snippet with environment isolation.
pub(super) fn snippet(code: &str) -> Vec<std::ffi::OsString> {
    vec!["-I".into(), "-c".into(), code.into()]
}

/// An explicit numeric child exit remains an observation, not benchmark success.
pub(super) fn exiting(status: i32) -> Vec<std::ffi::OsString> {
    snippet(&format!("{STREAMS};sys.exit({status})"))
}

/// Reject a POSIX/MSYS interpreter on Windows instead of interpreting its shell status as native.
pub(super) fn native() -> Vec<std::ffi::OsString> {
    let platform = if cfg!(windows) { "nt" } else { "posix" };
    snippet(&format!(
        "import os;assert os.name == {platform:?};{STREAMS};sys.exit(0)"
    ))
}

/// Extension needed to ask Windows for native image loading rather than script handling.
pub(super) fn executable_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_owned()
    }
}

/// The known fixture's native bad-image error, not an arbitrary child's numeric status.
pub(super) fn image_error() -> i32 {
    if cfg!(windows) { 193 } else { 8 }
}

#[cfg(unix)]
/// Configure executable permissions for an independently created launch fixture.
pub(super) fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}
#[cfg(not(unix))]
/// Validate the image fixture exists; there is no Unix execute bit on Windows.
/// This is never used as proof that a Windows permission test passed.
pub(super) fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
    std::fs::metadata(path).map(|_| ())
}
