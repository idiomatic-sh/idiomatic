//! The `corpus` port's fake runs the port's contract.

use super::super::contract::check;
use super::CorpusFake;

#[test]
fn succeeds() {
    check(&CorpusFake::default());
}

#[test]
fn a_file_that_changes_while_it_is_read_is_refused_as_unstable() {
    use crate::outbound::corpus::errors::CorpusError;
    use crate::outbound::corpus::interface::CorpusPort as _;
    use ryo_language::domain::files::types::FilePath;
    use ryo_values::domain::files::types::{CorpusEntry, FileBytes, FileSize, Nanoseconds};
    let entry = CorpusEntry {
        path: FilePath("unstable.rs".into()),
        size: FileSize(0),
        modified: Nanoseconds(0),
    };
    let refused = CorpusFake::default().read(&entry, &mut FileBytes::default());
    assert!(matches!(refused, Err(CorpusError::Unstable { .. })));
}
