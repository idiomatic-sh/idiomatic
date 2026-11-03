//! Physical observation and staging, confined to the suite's filesystem edge.
use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types::RunRequest};
use std::path::Path;

#[cfg(unix)]
fn observed(path: &Path) -> Result<bool, Error> {
    use std::os::unix::fs::PermissionsExt as _;
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(Error::Invalid(
            "metadata source is not a regular file".into(),
        ));
    }
    Ok(metadata.permissions().mode() & 0o111 != 0)
}
#[cfg(not(unix))]
fn observed(_: &Path) -> Result<bool, Error> {
    Err(Error::Invalid(
        "POSIX executable observations are unsupported on this host".into(),
    ))
}

/// Stage only the declared execute-bit observation; preserve other permission bits.
pub(super) fn stage(path: &Path, executable: bool) -> Result<(), Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let metadata = std::fs::metadata(path)?;
        let mode = metadata.permissions().mode();
        let mode = if executable {
            mode | 0o100
        } else {
            mode & !0o111
        };
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    }
    if observed(path)? != executable {
        return Err(Error::Invalid("staged executable metadata differs".into()));
    }
    Ok(())
}

/// Unconditional per-attempt verification, including Isolated input mode.
pub(super) fn validate(request: &RunRequest) -> Result<(), Error> {
    let Some(values) = &request.file_metadata else {
        return Ok(());
    };
    let root = request.root.canonicalize()?;
    for (file, expected) in values {
        crate::domain::suite::service::relative_path(file.0.as_ref())?;
        if !request.files.contains(file) {
            return Err(Error::Invalid("metadata names an undeclared source".into()));
        }
        let path = root.join(file.0.as_ref()).canonicalize()?;
        if !path.starts_with(&root) {
            return Err(Error::Invalid("metadata source escapes input root".into()));
        }
        if observed(&path)? != *expected {
            return Err(Error::Invalid(
                "declared executable metadata drifted before prepare".into(),
            ));
        }
    }
    Ok(())
}

/// Recompute a complete observation identity from the current physical sources.
pub(super) fn actual_digest(root: &Path, files: &[std::path::PathBuf]) -> Result<String, Error> {
    let root = root.canonicalize()?;
    let mut values = std::collections::BTreeMap::new();
    for file in files {
        crate::domain::suite::service::relative_path(file)?;
        let path = root.join(file).canonicalize()?;
        if !path.starts_with(&root) {
            return Err(Error::Invalid("metadata source escapes input root".into()));
        }
        let name = file
            .to_str()
            .ok_or_else(|| Error::Invalid("metadata source name is not UTF8".into()))?;
        values.insert(ryo::linter::RelativePath(name.into()), observed(&path)?);
    }
    crate::domain::suite::input_metadata::digest(Some(&values))?
        .map(|digest| digest.0)
        .ok_or_else(|| Error::Invalid("missing physical metadata digest".into()))
}
