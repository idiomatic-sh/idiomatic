//! Pure module decisions preserve marker boundaries and Given isolation.
use super::super::super::{
    service::{disk, given, relative_to},
    types::{Basenames, Markers, ModuleDecision, ModuleDottedPath, PackageModule, PackageRootName},
};
use ryo::languages::NameText;
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
        ModuleDecision::Need(PathBuf::from("/boundary/pkg/sub/__init__.py"))
    );
    let _previous_1 = markers
        .0
        .insert("/boundary/pkg/sub/__init__.py".into(), true);
    let _previous_2 = markers.0.insert("/boundary/pkg/__init__.py".into(), true);
    assert_eq!(
        disk(source, &markers),
        ModuleDecision::Need("/boundary/__init__.py".into())
    );
    let _previous_3 = markers.0.insert("/boundary/__init__.py".into(), false);
    assert_eq!(
        disk(source, &markers),
        ModuleDecision::Known(Some(PackageModule {
            path: ModuleDottedPath("pkg.sub.__init__".into()),
            package_root_name: PackageRootName("pkg".into())
        }))
    );
}

#[test]
fn standalone_is_complete_empty_and_pyi_is_not_a_marker() {
    let markers = Markers(BTreeMap::from([("/root/__init__.py".into(), false)]));
    assert_eq!(
        disk(Path::new("/root/mod.py"), &markers),
        ModuleDecision::Known(None)
    );
    let files = BTreeSet::from([PathBuf::from("__init__.pyi"), PathBuf::from("mod.py")]);
    assert_eq!(
        given(Path::new("mod.py"), &files),
        ModuleDecision::Known(None)
    );
}

#[test]
fn given_knows_nested_packages_but_never_an_unnamed_root_package() {
    let mut files = BTreeSet::from([
        PathBuf::from("pkg/__init__.py"),
        PathBuf::from("pkg/mod.py"),
    ]);
    assert_eq!(
        given(Path::new("pkg/mod.py"), &files),
        ModuleDecision::Known(Some(PackageModule {
            path: ModuleDottedPath("pkg.mod".into()),
            package_root_name: PackageRootName("pkg".into())
        }))
    );
    let _inserted = files.insert("__init__.py".into());
    assert_eq!(
        given(Path::new("pkg/mod.py"), &files),
        ModuleDecision::Unknown
    );
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

#[test]
fn raw_basenames_and_directory_components_are_not_filename_stems() {
    use super::super::super::service::basenames;
    let source = Path::new("/boundary/pkg.with.dot/nested.name/mod.with.dot.pyi");
    assert_eq!(
        basenames(source),
        Some(Basenames {
            module: NameText("mod.with.dot"),
            package: NameText("nested.name"),
        })
    );
    let markers = Markers(BTreeMap::from([
        (
            "/boundary/pkg.with.dot/nested.name/__init__.py".into(),
            true,
        ),
        ("/boundary/pkg.with.dot/__init__.py".into(), true),
        ("/boundary/__init__.py".into(), false),
    ]));
    assert_eq!(
        disk(source, &markers),
        ModuleDecision::Known(Some(PackageModule {
            path: ModuleDottedPath("pkg.with.dot.nested.name.mod.with.dot".into()),
            package_root_name: PackageRootName("pkg.with.dot".into())
        }))
    );
    assert_eq!(
        basenames(Path::new("/boundary/pkg.with.dot/__init__.py")),
        Some(Basenames {
            module: NameText("__init__"),
            package: NameText("pkg.with.dot"),
        })
    );
    assert_eq!(basenames(Path::new("/boundary/pkg/mod.txt")), None);
}

#[test]
fn dotted_nonpackage_directory_does_not_create_module_context() {
    let source = Path::new("/boundary/no.package/mod.py");
    let markers = Markers(BTreeMap::from([(
        "/boundary/no.package/__init__.py".into(),
        false,
    )]));
    assert_eq!(disk(source, &markers), ModuleDecision::Known(None));
    let files = BTreeSet::from([PathBuf::from("no.package/mod.py")]);
    assert_eq!(
        given(Path::new("no.package/mod.py"), &files),
        ModuleDecision::Known(None)
    );
}

#[test]
fn root_basename_preserves_contiguous_boundary_and_namespace_prefix() {
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
        let ModuleDecision::Known(Some(module)) = disk(Path::new(source), &markers) else {
            panic!("the complete contiguous ancestry has a module identity");
        };
        assert_eq!(module.package_root_name.0, "pkg.root");
        assert!(module.path.0.starts_with("pkg.root.sub.name."));
        assert!(!module.path.0.contains("namespace"));
    }
}
