//! Pure module decisions preserve marker boundaries and Given isolation.
use super::{
    service::{disk, given, relative_to},
    types::{Decision, Markers},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
#[test]
fn disk_requests_every_marker_then_retains_initializer_component() {
    let source = Path::new("/boundary/pkg/sub/__init__.py");
    let mut markers = Markers::default();
    assert_eq!(
        disk(source, &markers),
        Decision::Need(PathBuf::from("/boundary/pkg/sub/__init__.py"))
    );
    let _previous = markers
        .0
        .insert("/boundary/pkg/sub/__init__.py".into(), true);
    let _previous = markers.0.insert("/boundary/pkg/__init__.py".into(), true);
    assert_eq!(
        disk(source, &markers),
        Decision::Need("/boundary/__init__.py".into())
    );
    let _previous = markers.0.insert("/boundary/__init__.py".into(), false);
    assert_eq!(
        disk(source, &markers),
        Decision::Known(Some("pkg.sub.__init__".into()))
    );
}
#[test]
fn standalone_is_complete_empty_and_pyi_is_not_a_marker() {
    let markers = Markers(BTreeMap::from([("/root/__init__.py".into(), false)]));
    assert_eq!(
        disk(Path::new("/root/mod.py"), &markers),
        Decision::Known(None)
    );
    let files = BTreeSet::from([PathBuf::from("__init__.pyi"), PathBuf::from("mod.py")]);
    assert_eq!(given(Path::new("mod.py"), &files), Decision::Known(None));
}
#[test]
fn given_knows_nested_packages_but_never_an_unnamed_root_package() {
    let mut files = BTreeSet::from([
        PathBuf::from("pkg/__init__.py"),
        PathBuf::from("pkg/mod.py"),
    ]);
    assert_eq!(
        given(Path::new("pkg/mod.py"), &files),
        Decision::Known(Some("pkg.mod".into()))
    );
    let _inserted = files.insert("__init__.py".into());
    assert_eq!(given(Path::new("pkg/mod.py"), &files), Decision::Unknown);
}
#[test]
fn paths_above_root_remain_real_relative_dependencies() {
    assert_eq!(
        relative_to(
            Path::new("/boundary/pkg/sub"),
            Path::new("/boundary/pkg/__init__.py")
        ),
        Some(PathBuf::from("../__init__.py"))
    );
}
