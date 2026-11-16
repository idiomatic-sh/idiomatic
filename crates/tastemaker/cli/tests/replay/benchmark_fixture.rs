//! Benchmark-owned scratch manifests use the executing tool's version, and the scratch repository holds every lock of the
//! source repository, so a version another workspace pins is read there too; production workspaces stay untouched.
use sha2::{Digest as _, Sha256};
use std::{
    io::{self, Write as _},
    path::{Path, PathBuf},
};

/// The directories a lock is never looked for in: build trees, caches and git's own.
const SKIPPED: [&str; 4] = ["target", ".tastemaker_cache", ".ryo_cache", ".git"];

/// The path, relative to `root`, of every `tastemaker.lock` beneath `from`.
fn locks(root: &Path, from: &Path, found: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if SKIPPED.iter().any(|skipped| name == *skipped) {
            continue;
        }
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            locks(root, &path, found)?;
        } else if name == "tastemaker.lock" {
            found.push(
                path.strip_prefix(root)
                    .map_err(io::Error::other)?
                    .to_path_buf(),
            );
        }
    }
    Ok(())
}

/// Copies every lock of the source repository to the same path in the scratch repository `scratch`.
pub(crate) fn copy_locks(scratch: &Path) -> io::Result<()> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut found = Vec::new();
    locks(&source, &source, &mut found)?;
    for relative in found {
        let to = scratch.join(&relative);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let _: u64 = std::fs::copy(source.join(&relative), to)?;
    }
    Ok(())
}

/// Change only the canonical top-level tool pin, refusing unfamiliar fixture shapes.
fn rewritten(source: &str, version: &str) -> Result<(String, String), String> {
    let manifest: toml::Table =
        toml::from_str(source).map_err(|error| format!("invalid benchmark manifest: {error}"))?;
    let original = manifest
        .get("tastemaker")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| "benchmark fixture needs an explicit tool pin".to_owned())?;
    let needle = format!("tastemaker = {original:?}");
    let mut occurrences = source.match_indices(&needle).filter(|(start, _)| {
        let end = start + needle.len();
        (*start == 0 || source.as_bytes().get(start - 1) == Some(&b'\n'))
            && (end == source.len() || matches!(source.as_bytes().get(end), Some(b'\n' | b'\r')))
    });
    let (start, _) = occurrences
        .next()
        .ok_or_else(|| "unsupported benchmark tool-pin spelling".to_owned())?;
    if occurrences.next().is_some() {
        return Err("ambiguous benchmark tool pin".to_owned());
    }
    let mut effective = source.to_owned();
    effective.replace_range(
        start..start + needle.len(),
        &format!("tastemaker = {version:?}"),
    );
    Ok((original.to_owned(), effective))
}

/// Normalize only an already copied scratch fixture at `<scratch>/crates/ryo`, give the scratch repository every lock of
/// the source repository, and record exactly what changed on stderr.
pub(crate) fn normalize(root: &Path) -> io::Result<()> {
    let path = root.join("tastemaker.toml");
    let source = std::fs::read_to_string(&path)?;
    let version = env!("CARGO_PKG_VERSION");
    let (original, effective) = rewritten(&source, version).map_err(io::Error::other)?;
    if source != effective {
        std::fs::write(&path, &effective)?;
    }
    let scratch = root
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| io::Error::other("the fixture is not at <scratch>/crates/ryo"))?;
    copy_locks(scratch)?;
    writeln!(
        io::stderr(),
        "BENCH_FIXTURE_VERSION {}",
        serde_json::json!({
            "policy": "scratch-tool-pin-only/v2", "path": path,
            "original_pin": original, "effective_pin": version,
            "original_sha256": format!("{:x}", Sha256::digest(source.as_bytes())),
            "effective_sha256": format!("{:x}", Sha256::digest(effective.as_bytes())),
        })
    )?;
    Ok(())
}

#[test]
fn only_the_top_level_pin_changes() -> Result<(), String> {
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
fn normalization_operates_only_on_the_copied_fixture() -> io::Result<()> {
    let original = tempfile::tempdir()?;
    let scratch = tempfile::tempdir()?;
    let copy = scratch.path().join("crates/ryo");
    std::fs::create_dir_all(&copy)?;
    let source = "tastemaker = \"0.0.0\"\nworkspace = { name = \"fixture\" }\n";
    std::fs::write(original.path().join("tastemaker.toml"), source)?;
    let _: u64 = std::fs::copy(
        original.path().join("tastemaker.toml"),
        copy.join("tastemaker.toml"),
    )?;
    normalize(&copy)?;
    assert_eq!(
        std::fs::read_to_string(original.path().join("tastemaker.toml"))?,
        source
    );
    assert_eq!(
        std::fs::read_to_string(copy.join("tastemaker.toml"))?,
        rewritten(source, env!("CARGO_PKG_VERSION"))
            .map_err(io::Error::other)?
            .1
    );
    for lock in [
        "tastemaker.lock",
        "crates/tastemaker/tastemaker.lock",
        "crates/asymptote/tastemaker.lock",
    ] {
        assert!(
            scratch.path().join(lock).is_file(),
            "the scratch repository holds the source's {lock}"
        );
    }
    Ok(())
}
