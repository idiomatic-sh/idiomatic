//! A file of the local context adapter's test support: tracked trees with symbolic links, and the scenarios that seal
//! them. A tracked link is sealed by its exact target text, which must name a sealed file of the same snapshot; a
//! changed, non-canonical or unsealed link, a requested source that is a link, and a target that is not UTF-8 are
//! refused.
use super::super::adapter::{seal_existing, validate};
use ryo_benchmark_contracts::domain::context::types::{ContextManifest, LinkTarget};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::{collections::BTreeMap, path::Path};

/// Make `at` a symbolic link to `target`.
#[cfg(unix)]
pub(crate) fn link(target: &Path, at: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, at)
}

/// Platforms without symbolic links cannot build this fixture.
#[cfg(not(unix))]
pub(crate) fn link(_target: &Path, _at: &Path) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}
/// Write a tracked tree under `root`: each file with its bytes and each link with its target text, and return every
/// tracked path.
fn tree(
    root: &Path,
    files: &[&str],
    links: &[(&str, &str)],
) -> Result<Vec<std::path::PathBuf>, Error> {
    let mut tracked = Vec::new();
    for file in files {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().expect("a file has a parent"))?;
        std::fs::write(&path, file.as_bytes())?;
        tracked.push(std::path::PathBuf::from(file));
    }
    for (at, target) in links {
        let path = root.join(at);
        std::fs::create_dir_all(path.parent().expect("a link has a parent"))?;
        link(Path::new(target), &path)?;
        tracked.push(std::path::PathBuf::from(at));
    }
    Ok(tracked)
}
/// Seal `root` with no requested source.
fn sealed(root: &Path, tracked: &[std::path::PathBuf]) -> Result<ContextManifest, Error> {
    seal_existing(root, &[], tracked)
}
/// Whether `result` is refused as invalid with exactly `message`.
fn refused_with<T>(result: Result<T, Error>, message: &str) -> bool {
    matches!(result, Err(Error::Invalid(text)) if text == message)
}
pub(crate) fn seal_records_a_tracked_link_to_a_sealed_file() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let root = scratch.path();
    let tracked = tree(root, &["a/img.png"], &[("b/img.png", "../a/img.png")])?;
    let manifest = sealed(root, &tracked)?;
    assert_eq!(
        manifest.links,
        BTreeMap::from([("b/img.png".into(), LinkTarget("../a/img.png".into()))])
    );
    assert!(
        manifest
            .auxiliary_files
            .contains_key(Path::new("a/img.png"))
    );
    assert!(
        !manifest
            .auxiliary_files
            .contains_key(Path::new("b/img.png"))
    );
    assert!(!manifest.source_files.contains_key(Path::new("b/img.png")));
    validate(&manifest, root)
}
pub(crate) fn validate_refuses_a_sealed_link_changed_after_the_seal() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let root = scratch.path();
    let tracked = tree(
        root,
        &["a/img.png", "a/other.png"],
        &[("b/img.png", "../a/img.png")],
    )?;
    let manifest = sealed(root, &tracked)?;
    let at = root.join("b/img.png");
    std::fs::remove_file(&at)?;
    link(Path::new("../a/other.png"), &at)?;
    assert!(refused_with(
        validate(&manifest, root),
        &format!("context link changed: {}", Path::new("b/img.png").display())
    ));
    std::fs::remove_file(&at)?;
    assert!(matches!(validate(&manifest, root), Err(Error::Io(_))));
    std::fs::write(&at, b"a/img.png")?;
    let _verified_file =
        validate(&manifest, root).expect_err("a regular file at a sealed link is refused");
    std::fs::remove_file(&at)?;
    link(Path::new("../a/img.png"), &at)?;
    validate(&manifest, root)?;
    std::fs::rename(root.join("b"), root.join("real"))?;
    link(Path::new("real"), &root.join("b"))?;
    let parent = validate(&manifest, root).expect_err("a symlinked parent is refused");
    assert!(
        matches!(&parent, Error::Invalid(text) if text.starts_with("symlink in context")),
        "{parent}"
    );
    Ok(())
}
pub(crate) fn validate_refuses_a_path_equal_retarget() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let root = scratch.path();
    let tracked = tree(root, &["a/img.png"], &[("b/img.png", "../a/img.png")])?;
    let manifest = sealed(root, &tracked)?;
    let at = root.join("b/img.png");
    for text in ["../a//img.png", "../a/./img.png", "../a/img.png/"] {
        std::fs::remove_file(&at)?;
        link(Path::new(text), &at)?;
        assert!(
            refused_with(
                validate(&manifest, root),
                &format!("context link changed: {}", Path::new("b/img.png").display())
            ),
            "{text}"
        );
    }
    Ok(())
}
pub(crate) fn seal_refuses_non_canonical_link_text() -> Result<(), Error> {
    for text in [
        "/abs/img.png",
        "../a//img.png",
        "../a/./img.png",
        "./../a/img.png",
        "../a/img.png/",
        "../a/../a/img.png",
        "../../../../x",
    ] {
        let scratch = tempfile::tempdir()?;
        let root = scratch.path();
        let tracked = tree(root, &["a/img.png"], &[("b/img.png", text)])?;
        assert!(
            refused_with(
                sealed(root, &tracked),
                super::super::constants::CONTEXT_LINK_TARGET_NOT_CANONICAL
            ),
            "{text}"
        );
    }
    Ok(())
}
/// The target grammar is slash-only: a backslash, with or without dots, names nothing on any platform. The manifest
/// alone is refused, before any link is read, so this row needs no symbolic link and runs everywhere.
pub(crate) fn validate_refuses_backslash_link_text() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let root = std::path::absolute(scratch.path())?;
    let tracked = tree(&root, &["b/a/img.png"], &[])?;
    let manifest = sealed(&root, &tracked)?;
    for text in [
        r"a\.\img.png",
        r"a\img.png",
        r".\a\img.png",
        r"..\b\a\img.png",
    ] {
        let mut linked = manifest.clone();
        let _absent = linked
            .links
            .insert("b/link.png".into(), LinkTarget(text.into()));
        assert!(
            refused_with(
                validate(&linked, &root),
                super::super::constants::CONTEXT_LINK_TARGET_NOT_CANONICAL
            ),
            "{text}"
        );
    }
    Ok(())
}
pub(crate) fn seal_refuses_a_link_to_an_unsealed_target() -> Result<(), Error> {
    let untracked = tempfile::tempdir()?;
    std::fs::create_dir_all(untracked.path().join("a"))?;
    std::fs::write(untracked.path().join("a/img.png"), b"untracked")?;
    let tracked = tree(untracked.path(), &[], &[("b/img.png", "../a/img.png")])?;
    assert!(refused_with(
        sealed(untracked.path(), &tracked),
        super::super::constants::CONTEXT_LINK_TARGET_UNSEALED
    ));
    let directory = tempfile::tempdir()?;
    let tracked = tree(directory.path(), &["a/img.png"], &[("b/img.png", "../a")])?;
    assert!(refused_with(
        sealed(directory.path(), &tracked),
        super::super::constants::CONTEXT_LINK_TARGET_UNSEALED
    ));
    let chain = tempfile::tempdir()?;
    let tracked = tree(
        chain.path(),
        &["a/img.png"],
        &[("b/img.png", "../c/img.png"), ("c/img.png", "../a/img.png")],
    )?;
    assert!(refused_with(
        sealed(chain.path(), &tracked),
        super::super::constants::CONTEXT_LINK_TARGET_UNSEALED
    ));
    Ok(())
}
pub(crate) fn seal_refuses_a_requested_source_that_is_a_link() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let root = scratch.path();
    let tracked = tree(root, &["pkg/real.py"], &[("pkg/mod.py", "real.py")])?;
    assert!(refused_with(
        seal_existing(
            root,
            &[ryo::linter::RelativePath("pkg/mod.py".into())],
            &tracked
        ),
        super::super::constants::SEALED_SOURCE_IS_A_LINK
    ));
    Ok(())
}
/// A link target that is not UTF-8 is refused: the seal holds exact text.
pub(crate) fn seal_refuses_a_non_utf8_link_target() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let root = scratch.path();
    let mut tracked = tree(root, &["a/img.png"], &[])?;
    std::fs::create_dir_all(root.join("b"))?;
    link(Path::new(&non_utf8_target()), &root.join("b/img.png"))?;
    tracked.push("b/img.png".into());
    assert!(refused_with(
        sealed(root, &tracked),
        super::super::constants::NON_UTF8_LINK_TARGET
    ));
    Ok(())
}
/// The target text `../a/<byte 0xff>img.png`, which is not UTF-8.
#[cfg(unix)]
fn non_utf8_target() -> std::ffi::OsString {
    use std::os::unix::ffi::OsStrExt as _;
    std::ffi::OsStr::from_bytes(b"../a/\xffimg.png").to_owned()
}
/// The target text `../a/<unpaired surrogate>img.png`, which is not UTF-8.
#[cfg(windows)]
fn non_utf8_target() -> std::ffi::OsString {
    use std::os::windows::ffi::OsStringExt as _;
    let mut wide: Vec<u16> = "../a/".encode_utf16().collect();
    wide.push(0xD800);
    wide.extend("img.png".encode_utf16());
    std::ffi::OsString::from_wide(&wide)
}
