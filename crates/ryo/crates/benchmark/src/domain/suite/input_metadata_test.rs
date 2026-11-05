//! Stable optional input identity, distinct truth states and required-observation admission.
use super::input_metadata::digest;
use super::input_metadata::require;
use ryo::linter::RelativePath;
use ryo_benchmark_contracts::domain::workloads::types::FileObservation;
use std::collections::BTreeMap;

#[test]
fn unspecified_identity_stays_legacy_while_declared_mode_and_path_are_bound() {
    assert_eq!(digest(None).unwrap(), None);
    let empty = BTreeMap::new();
    assert!(digest(Some(&empty)).unwrap().is_some());
    let file = RelativePath("λ.py".into());
    let no = BTreeMap::from([(file.clone(), false)]);
    let yes = BTreeMap::from([(file.clone(), true)]);
    assert_ne!(digest(Some(&no)).unwrap(), digest(Some(&yes)).unwrap());
    let first = BTreeMap::from([
        (RelativePath("ab".into()), true),
        (RelativePath("c".into()), false),
    ]);
    let second = BTreeMap::from([
        (RelativePath("a".into()), true),
        (RelativePath("bc".into()), false),
    ]);
    assert_ne!(
        digest(Some(&first)).unwrap(),
        digest(Some(&second)).unwrap()
    );
    assert!(
        require(
            Some(FileObservation::PosixExecutable),
            std::slice::from_ref(&file),
            None
        )
        .is_err()
    );
    assert!(
        require(
            Some(FileObservation::PosixExecutable),
            std::slice::from_ref(&file),
            Some(&empty)
        )
        .is_err()
    );
    assert!(
        require(
            Some(FileObservation::PosixExecutable),
            std::slice::from_ref(&file),
            Some(&no)
        )
        .is_ok(),
        "known false is a complete observation"
    );
}
