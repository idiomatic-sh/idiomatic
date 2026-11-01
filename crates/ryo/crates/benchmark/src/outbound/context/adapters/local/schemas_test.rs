//! Worker schema round trips preserve context identity and opaque dependency fingerprints.
use super::schemas::{ManifestSchema, ModeSchema, ReceiptSchema};
use crate::domain::{
    diagnostics::types::ProjectContextReceipt,
    suite::types::{ArtifactDigest, ContextManifest, ProjectContextMode},
};
use std::{collections::BTreeMap, path::PathBuf};
#[test]
fn manifests_and_receipts_round_trip_without_synthesizing_missing_paths() {
    let manifest = ContextManifest {
        boundary: PathBuf::from("/snapshot"),
        logical_root: PathBuf::from("pkg"),
        auxiliary_files: BTreeMap::from([(
            "pkg/__init__.py".into(),
            ArtifactDigest("auxiliary-hash".into()),
        )]),
        source_files: BTreeMap::from([("pkg/mod.py".into(), ArtifactDigest("source-hash".into()))]),
        missing: vec!["__init__.py".into()],
    };
    let encoded =
        serde_json::to_vec(&ManifestSchema::from(manifest.clone())).expect("manifest JSON");
    let decoded: ManifestSchema = serde_json::from_slice(&encoded).expect("manifest wire");
    assert_eq!(ContextManifest::from(decoded), manifest);
    let receipt = ProjectContextReceipt {
        mode: ProjectContextMode::ListedDisk,
        logical_root: "pkg".into(),
        actual_root: "/snapshot/pkg".into(),
        manifest_hash: ArtifactDigest("context-hash".into()),
        dependencies: BTreeMap::from([
            (
                "__init__.py".into(),
                Some(ryo::linter::dependency_fingerprint(b"")),
            ),
            ("../__init__.py".into(), None),
        ]),
    };
    let encoded = serde_json::to_vec(&ReceiptSchema::from(receipt.clone())).expect("receipt JSON");
    let decoded: ReceiptSchema = serde_json::from_slice(&encoded).expect("receipt wire");
    assert_eq!(ProjectContextReceipt::from(decoded), receipt);
    assert!(serde_json::from_str::<ModeSchema>("\"disk-by-default\"").is_err());
}
