//! Filesystem permissions needed by independently materialized launch fixtures.

#[cfg(unix)]
pub(super) fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}
#[cfg(not(unix))]
pub(super) fn make_executable(_: &std::path::Path) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}
