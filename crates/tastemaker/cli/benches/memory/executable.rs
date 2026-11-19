//! The one writer of an executable file in this process: a child writes the file, so no descriptor
//! this multi-threaded process holds for writing can leak into a child another thread forks, and the
//! launch that follows never fails with "Text file busy" (ETXTBSY).

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

/// Writes `contents` to `path` through a child `/bin/sh -c 'cat > "$1"'`, then makes it executable
/// by its owner, its group and others (0o755).
#[cfg(unix)]
pub(crate) fn write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    let mut child = Command::new("/bin/sh")
        .args(["-c", "cat > \"$1\"", "sh"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()?;
    let written = child
        .stdin
        .take()
        .ok_or_else(|| std::io::Error::other("the writer has no standard input"))
        .and_then(|mut stdin| stdin.write_all(contents));
    let status = child.wait()?;
    written?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "writing {} ended with {status}",
            path.display()
        )));
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

/// Off POSIX there is no mode to set: the file is written in place.
#[cfg(not(unix))]
pub(crate) fn write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, contents)
}
