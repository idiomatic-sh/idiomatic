//! An executable a test writes and then launches, written so that no launch in this process can find it busy.
//!
//! The test process runs many tests at once, and a thread that launches a child forks first: until that child execs, it holds
//! a copy of every descriptor this process had open, close-on-exec ones included. A file this process opened for writing,
//! however briefly, may so stay open for writing in another thread's child after this process closed it, and Linux refuses to
//! launch a file that is open for writing (`ETXTBSY`, "Text file busy"). Renaming or linking the written file changes no
//! descriptor, so the file is written by a child process of its own: this process never holds a descriptor on it, so no fork
//! can inherit one.

use std::path::Path;

/// Writes `contents` to `path` as an executable a test then launches, without this process ever opening `path`: a child
/// process writes it, and has ended, before this answers. Off Unix, where a launch never finds a file busy, it is written
/// directly.
///
/// # Errors
/// The writing child cannot be started, cannot be given the contents or ends unsuccessfully, or the mode cannot be set.
#[cfg(unix)]
pub fn write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    use std::os::unix::fs::PermissionsExt as _;
    let mut writer = std::process::Command::new("/bin/sh")
        .args(["-c", "cat > \"$1\"", "sh"])
        .arg(path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()?;
    let given = writer
        .stdin
        .take()
        .ok_or_else(|| std::io::Error::other("the writing child has no standard input"))
        .and_then(|mut stdin| stdin.write_all(contents));
    let ended = writer.wait()?;
    given?;
    if !ended.success() {
        return Err(std::io::Error::other(format!(
            "writing {} ended with {ended}",
            path.display()
        )));
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

/// Writes `contents` to `path` as an executable a test then launches. Off Unix a launch never finds a file busy and there is
/// no mode to set, so it is written directly.
///
/// # Errors
/// The file cannot be written.
#[cfg(not(unix))]
pub fn write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, contents)
}
