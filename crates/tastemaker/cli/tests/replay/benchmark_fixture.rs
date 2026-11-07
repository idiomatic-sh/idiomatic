//! Benchmark-owned scratch manifests use the executing tool's version; production workspaces stay untouched.
use sha2::{Digest as _, Sha256};
use std::{io, path::Path};

/// Change only the canonical top-level tool pin, refusing unfamiliar fixture shapes.
fn rewritten(source: &str, version: &str) -> Result<(String, String), &'static str> {
    let manifest: toml::Table = toml::from_str(source).map_err(|_| "invalid benchmark manifest")?;
    let original = manifest
        .get("tastemaker")
        .and_then(toml::Value::as_str)
        .ok_or("benchmark fixture needs an explicit tool pin")?;
    let needle = format!("tastemaker = {original:?}");
    let mut occurrences = source.match_indices(&needle).filter(|(start, _)| {
        let end = start + needle.len();
        (*start == 0 || source.as_bytes().get(start - 1) == Some(&b'\n'))
            && (end == source.len() || matches!(source.as_bytes().get(end), Some(b'\n' | b'\r')))
    });
    let (start, _) = occurrences
        .next()
        .ok_or("unsupported benchmark tool-pin spelling")?;
    if occurrences.next().is_some() {
        return Err("ambiguous benchmark tool pin");
    }
    let mut effective = source.to_owned();
    effective.replace_range(
        start..start + needle.len(),
        &format!("tastemaker = {version:?}"),
    );
    Ok((original.to_owned(), effective))
}

/// Normalize only an already copied scratch fixture and record exactly what changed on stderr.
pub(crate) fn normalize(root: &Path) -> io::Result<()> {
    let path = root.join("tastemaker.toml");
    let source = std::fs::read_to_string(&path)?;
    let version = env!("CARGO_PKG_VERSION");
    let (original, effective) = rewritten(&source, version).map_err(io::Error::other)?;
    if source != effective {
        std::fs::write(&path, &effective)?;
    }
    eprintln!(
        "BENCH_FIXTURE_VERSION {}",
        serde_json::json!({
            "policy": "scratch-tool-pin-only/v1", "path": path,
            "original_pin": original, "effective_pin": version,
            "original_sha256": format!("{:x}", Sha256::digest(source.as_bytes())),
            "effective_sha256": format!("{:x}", Sha256::digest(effective.as_bytes())),
        })
    );
    Ok(())
}

#[test]
fn only_the_top_level_pin_changes() -> Result<(), &'static str> {
    let source = "# fixture\ntastemaker = \"0.1.0\"\nworkspace = { name = \"ryo\" }\n[params]\nversion = \"0.1.0\"\n";
    let (original, effective) = rewritten(source, "0.1.1")?;
    assert_eq!(original, "0.1.0");
    assert_eq!(
        effective,
        "# fixture\ntastemaker = \"0.1.1\"\nworkspace = { name = \"ryo\" }\n[params]\nversion = \"0.1.0\"\n"
    );
    assert_eq!(rewritten(&effective, "0.1.1")?.1, effective);
    Ok(())
}

#[test]
fn invalid_or_unfamiliar_manifest_is_not_guessed() {
    for source in [
        "",
        "tastemaker = [",
        "tastemaker = 1",
        "tastemaker='0.1.0'\n",
    ] {
        assert!(rewritten(source, "0.1.1").is_err(), "{source}");
    }
}

#[test]
fn normalization_operates_only_on_the_copied_fixture() -> std::io::Result<()> {
    let original = tempfile::tempdir()?;
    let copy = tempfile::tempdir()?;
    let source = "tastemaker = \"0.0.0\"\nworkspace = { name = \"fixture\" }\n";
    std::fs::write(original.path().join("tastemaker.toml"), source)?;
    std::fs::copy(
        original.path().join("tastemaker.toml"),
        copy.path().join("tastemaker.toml"),
    )?;
    normalize(copy.path())?;
    assert_eq!(
        std::fs::read_to_string(original.path().join("tastemaker.toml"))?,
        source
    );
    assert_eq!(
        std::fs::read_to_string(copy.path().join("tastemaker.toml"))?,
        rewritten(source, env!("CARGO_PKG_VERSION"))
            .map_err(std::io::Error::other)?
            .1
    );
    Ok(())
}
