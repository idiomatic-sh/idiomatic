//! Cross-language source identities remain portable across native platforms.
use super::*;
#[test]
fn source_identity_uses_utf8_name_order_on_every_platform() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/reference-discrepancy-canonical.json"
    )))
    .expect("cross-language encoding fixture");
    let root = tempfile::tempdir().expect("source boundary");
    let mut files = Vec::new();
    for (name, source) in fixture["source_files"].as_object().expect("source map") {
        let file = std::path::PathBuf::from(name);
        let target = root.path().join(&file);
        std::fs::create_dir_all(target.parent().expect("source parent")).expect("source directory");
        std::fs::write(target, source.as_str().expect("source UTF8")).expect("source bytes");
        files.push(file);
    }
    files.reverse();
    verify(CaseArtifacts {
        profile: root.path(),
        root: root.path(),
        files: &files,
        source_sha256: fixture["source_sha256"]
            .as_str()
            .expect("Python source digest"),
        evidence: &[],
    })
    .expect("identical UTF8 digest with BMP and supplementary-plane filenames");
}
