//! Actual positional-store metadata roundtrip and predecode format admission.
use super::super::constants::{FILE, FORMAT};
use super::*;
use ryo_values::domain::files::types::ExecutablePermission;

#[test]
fn known_true_false_and_unavailable_observations_roundtrip_through_the_store() {
    let root = tempfile::tempdir().unwrap();
    let adapter = store_in(root.path(), "permissions");
    for observation in [
        ExecutablePermission::PosixNotExecutable,
        ExecutablePermission::PosixExecutable,
        ExecutablePermission::Unknown,
        ExecutablePermission::Unsupported,
    ] {
        let mut kept = stored(1);
        kept.files[0].executable = observation;
        adapter
            .stored()
            .unwrap()
            .keep(kept.clone(), &KeptRows::default())
            .unwrap();
        let bytes = std::fs::read(root.path().join(FILE)).unwrap();
        assert!(bytes.ends_with(FORMAT));
        assert_eq!(adapter.stored().unwrap().tables().files, kept.files);
    }
}

#[test]
fn missing_or_changed_format_tag_is_a_cache_miss_without_mutating_old_bytes() {
    let root = tempfile::tempdir().unwrap();
    let adapter = store_in(root.path(), "permissions");
    adapter
        .stored()
        .unwrap()
        .keep(stored(1), &KeptRows::default())
        .unwrap();
    let path = root.path().join(FILE);
    let valid = std::fs::read(&path).unwrap();
    let untagged = valid[..valid.len() - FORMAT.len()].to_vec();
    let mut changed = valid.clone();
    *changed.last_mut().unwrap() ^= 1;
    for bytes in [untagged, changed] {
        std::fs::write(&path, &bytes).unwrap();
        assert!(adapter.stored().unwrap().tables().files.is_empty());
        assert_eq!(
            std::fs::read(&path).unwrap(),
            bytes,
            "a rejected cache is not deleted or rewritten during read"
        );
    }
    adapter
        .stored()
        .unwrap()
        .keep(stored(1), &KeptRows::default())
        .unwrap();
    assert!(
        !adapter.stored().unwrap().tables().files.is_empty(),
        "ordinary recomputation publishes a usable versioned store"
    );
}
