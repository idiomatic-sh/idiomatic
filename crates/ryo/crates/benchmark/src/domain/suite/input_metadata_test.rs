//! Stable optional input identity, distinct truth states and required-observation admission.
use super::input_metadata::require;
use ryo::linter::RelativePath;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use ryo_benchmark_contracts::domain::workloads::types::FileObservation;
use std::collections::BTreeMap;

#[test]
fn unspecified_identity_stays_legacy_while_declared_mode_and_path_are_bound() {
    assert_eq!(
        ArtifactDigest::file_metadata(None::<std::iter::Empty<FileMetadataEntry<'_>>>).unwrap(),
        None
    );
    let empty = BTreeMap::new();
    assert!(
        ArtifactDigest::file_metadata((Some(&empty)).map(|values| values.iter().map(
            |(path, executable)| FileMetadataEntry {
                path,
                executable: if *executable {
                    FileExecutable::Set
                } else {
                    FileExecutable::Unset
                }
            }
        )))
        .unwrap()
        .is_some()
    );
    let file = RelativePath("λ.py".into());
    let no = BTreeMap::from([(file.clone(), false)]);
    let yes = BTreeMap::from([(file.clone(), true)]);
    assert_ne!(
        ArtifactDigest::file_metadata((Some(&no)).map(|values| values.iter().map(
            |(path, executable)| FileMetadataEntry {
                path,
                executable: if *executable {
                    FileExecutable::Set
                } else {
                    FileExecutable::Unset
                }
            }
        )))
        .unwrap(),
        ArtifactDigest::file_metadata((Some(&yes)).map(|values| values.iter().map(
            |(path, executable)| FileMetadataEntry {
                path,
                executable: if *executable {
                    FileExecutable::Set
                } else {
                    FileExecutable::Unset
                }
            }
        )))
        .unwrap()
    );
    let first = BTreeMap::from([
        (RelativePath("ab".into()), true),
        (RelativePath("c".into()), false),
    ]);
    let second = BTreeMap::from([
        (RelativePath("a".into()), true),
        (RelativePath("bc".into()), false),
    ]);
    assert_ne!(
        ArtifactDigest::file_metadata((Some(&first)).map(|values| values.iter().map(
            |(path, executable)| FileMetadataEntry {
                path,
                executable: if *executable {
                    FileExecutable::Set
                } else {
                    FileExecutable::Unset
                }
            }
        )))
        .unwrap(),
        ArtifactDigest::file_metadata((Some(&second)).map(|values| values.iter().map(
            |(path, executable)| FileMetadataEntry {
                path,
                executable: if *executable {
                    FileExecutable::Set
                } else {
                    FileExecutable::Unset
                }
            }
        )))
        .unwrap()
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
