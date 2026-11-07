//! A file of the ignore corpus adapter's test support: a file whose execute bits change under the adapter's eyes.

use crate::outbound::corpus::errors::CorpusError;

/// The inventory and the stat observe each posix execute bit without changing the bytes or the modification time.
#[cfg(unix)]
pub(crate) fn execute_bits() -> Result<usize, CorpusError> {
    use super::walking::{known_entry, walking};
    use crate::outbound::corpus::interface::CorpusPort as _;
    use crate::outbound::corpus::types::PathStat;
    use ryo_language::domain::files::types::FilePath;
    use ryo_language::inbound::use_cases::project::types::RootPath;
    use ryo_values::domain::files::types::ExecutablePermission;
    use std::os::unix::fs::PermissionsExt as _;

    let root = tempfile::tempdir().expect("a temporary directory");
    let path = root.path().join("case.py");
    std::fs::write(&path, b"print(1)\n").expect("a file");
    let adapter = walking(RootPath(root.path().to_str().expect("a utf-8 path").into()));
    let file = FilePath("case.py".into());
    let PathStat::Found(original) = adapter.stat(&file)? else {
        panic!("file is present");
    };
    for (mode, expected) in [
        (0o644, false),
        (0o744, true),
        (0o654, true),
        (0o645, true),
        (0o644, false),
    ] {
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))
            .expect("permissions");
        let PathStat::Found(observed) = adapter.stat(&file)? else {
            panic!("file remains present");
        };
        assert_eq!(
            observed.executable,
            if expected {
                ExecutablePermission::PosixExecutable
            } else {
                ExecutablePermission::PosixNotExecutable
            }
        );
        assert_eq!(observed.size, original.size);
        assert_eq!(observed.modified, original.modified);
        assert_eq!(
            known_entry(&adapter.inventory()?.0[0]).executable,
            observed.executable
        );
        assert_eq!(std::fs::read(&path).expect("the file"), b"print(1)\n");
    }
    Ok(1)
}

/// A platform without posix execute bits has none to observe: the row holds there as it does here.
#[cfg(not(unix))]
pub(crate) fn execute_bits() -> Result<usize, CorpusError> {
    Ok(1)
}
