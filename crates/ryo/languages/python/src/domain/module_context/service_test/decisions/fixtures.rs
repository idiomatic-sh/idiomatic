//! The scenarios of the group: each runs its subject and records what it observed.
use super::super::super::{
    service::{disk, given, relative_to},
    types::{Markers, ModuleDecision},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

pub(super) fn disk_requests_every_marker_then_retains_initializer_component() -> String {
    let mut observed: Vec<String> = Vec::new();
    let source = Path::new("/boundary/pkg/sub/__init__.py");
    let mut markers = Markers::default();
    observed.push(format!("{:?}", disk(source, &markers)));
    let _previous_1 = markers
        .0
        .insert("/boundary/pkg/sub/__init__.py".into(), true);
    let _previous_2 = markers.0.insert("/boundary/pkg/__init__.py".into(), true);
    observed.push(format!("{:?}", disk(source, &markers)));
    let _previous_3 = markers.0.insert("/boundary/__init__.py".into(), false);
    observed.push(format!("{:?}", disk(source, &markers)));
    observed.join("|")
}

pub(super) fn standalone_is_complete_empty_and_pyi_is_not_a_marker() -> String {
    let mut observed: Vec<String> = Vec::new();
    let markers = Markers(BTreeMap::from([("/root/__init__.py".into(), false)]));
    observed.push(format!("{:?}", disk(Path::new("/root/mod.py"), &markers)));
    let files = BTreeSet::from([PathBuf::from("__init__.pyi"), PathBuf::from("mod.py")]);
    observed.push(format!("{:?}", given(Path::new("mod.py"), &files)));
    observed.join("|")
}

pub(super) fn given_knows_nested_packages_but_never_an_unnamed_root_package() -> String {
    let mut observed: Vec<String> = Vec::new();
    let mut files = BTreeSet::from([
        PathBuf::from("pkg/__init__.py"),
        PathBuf::from("pkg/mod.py"),
    ]);
    observed.push(format!("{:?}", given(Path::new("pkg/mod.py"), &files)));
    let _inserted = files.insert("__init__.py".into());
    observed.push(format!("{:?}", given(Path::new("pkg/mod.py"), &files)));
    observed.join("|")
}

pub(super) fn paths_above_root_remain_real_relative_dependencies() -> String {
    let mut observed: Vec<String> = Vec::new();
    observed.push(format!(
        "{:?}",
        relative_to(
            Path::new("/boundary/pkg/sub"),
            Path::new("/boundary/pkg/__init__.py")
        )
    ));
    observed.join("|")
}

pub(super) fn raw_basenames_and_directory_components_are_not_filename_stems() -> String {
    let mut observed: Vec<String> = Vec::new();
    use super::super::super::service::basenames;
    let source = Path::new("/boundary/pkg.with.dot/nested.name/mod.with.dot.pyi");
    observed.push(format!("{:?}", basenames(source)));
    let markers = Markers(BTreeMap::from([
        (
            "/boundary/pkg.with.dot/nested.name/__init__.py".into(),
            true,
        ),
        ("/boundary/pkg.with.dot/__init__.py".into(), true),
        ("/boundary/__init__.py".into(), false),
    ]));
    observed.push(format!("{:?}", disk(source, &markers)));
    observed.push(format!(
        "{:?}",
        basenames(Path::new("/boundary/pkg.with.dot/__init__.py"))
    ));
    observed.push(format!(
        "{:?}",
        basenames(Path::new("/boundary/pkg/mod.txt"))
    ));
    observed.join("|")
}

pub(super) fn dotted_nonpackage_directory_does_not_create_module_context() -> String {
    let mut observed: Vec<String> = Vec::new();
    let source = Path::new("/boundary/no.package/mod.py");
    let markers = Markers(BTreeMap::from([(
        "/boundary/no.package/__init__.py".into(),
        false,
    )]));
    observed.push(format!("{:?}", disk(source, &markers)));
    let files = BTreeSet::from([PathBuf::from("no.package/mod.py")]);
    observed.push(format!(
        "{:?}",
        given(Path::new("no.package/mod.py"), &files)
    ));
    observed.join("|")
}

pub(super) fn root_basename_preserves_contiguous_boundary_and_namespace_prefix() -> String {
    let mut observed: Vec<String> = Vec::new();
    for source in [
        "/boundary/namespace/pkg.root/sub.name/__init__.py",
        "/boundary/namespace/pkg.root/sub.name/__main__.py",
        "/boundary/namespace/pkg.root/sub.name/worker.pyi",
        "/boundary/namespace/pkg.root/sub.name/migrations/0001_initial.py",
    ] {
        let markers = Markers(BTreeMap::from([
            (
                "/boundary/namespace/pkg.root/sub.name/migrations/__init__.py".into(),
                true,
            ),
            (
                "/boundary/namespace/pkg.root/sub.name/__init__.py".into(),
                true,
            ),
            ("/boundary/namespace/pkg.root/__init__.py".into(), true),
            ("/boundary/namespace/__init__.py".into(), false),
        ]));
        match disk(Path::new(source), &markers) {
            ModuleDecision::Known(Some(module)) => {
                observed.push(format!("{:?}", module.package_root_name.0));
                observed.push(format!(
                    "{}",
                    module.path.0.starts_with("pkg.root.sub.name.")
                ));
                observed.push(format!("{}", !module.path.0.contains("namespace")));
            }
            other => observed.push(format!("{other:?}")),
        }
    }
    observed.join("|")
}
