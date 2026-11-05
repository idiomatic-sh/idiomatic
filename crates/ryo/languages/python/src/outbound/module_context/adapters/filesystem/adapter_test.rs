//! Given context never uses a filesystem root; Disk lookup failures remain errors.
use super::adapter::ModuleContextFilesystem;
use crate::outbound::module_context::interface::ModuleContextPort as _;
use ryo::languages::{FilePath, ProjectError, ProjectRequest, Relation, Repository, RootPath};
fn request(repository: Repository, files: &[&str]) -> ProjectRequest {
    ProjectRequest {
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Default::default(),
        root: RootPath("invalid\0disk-root".into()),
        repository,
        manifests: vec![],
        files: files.iter().map(|path| FilePath((*path).into())).collect(),
        notes: vec![],
        wanted: vec![Relation::ModuleIdentity],
    }
}
#[test]
fn given_is_zero_io_and_distinguishes_standalone_from_unknown_root_identity() {
    let adapter = ModuleContextFilesystem;
    let standalone = adapter
        .resolve(&request(Repository::Given, &["mod.py"]))
        .expect("the unusable disk root must never be touched");
    assert!(standalone.rows.is_empty() && !standalone.partial && standalone.consulted.is_empty());
    let unnamed = adapter
        .resolve(&request(Repository::Given, &["__init__.py", "mod.py"]))
        .expect("Given still performs no disk lookup");
    assert!(unnamed.rows.is_empty() && unnamed.partial && unnamed.consulted.is_empty());
    let nested = adapter
        .resolve(&request(
            Repository::Given,
            &["pkg/__init__.py", "pkg/mod.py"],
        ))
        .expect("known logical package");
    assert_eq!(nested.rows[1].path, "pkg.mod");
    assert!(!nested.partial && nested.consulted.is_empty());
}
#[test]
fn disk_stat_failures_are_not_package_absence() {
    assert!(matches!(
        ModuleContextFilesystem.resolve(&request(Repository::Disk, &["mod.py"])),
        Err(ProjectError::Context { .. })
    ));
}

#[test]
fn given_raw_names_preserve_dotted_packages_modules_and_stub_extension() {
    let contexts = ModuleContextFilesystem
        .resolve(&request(
            Repository::Given,
            &[
                "pkg.with.dot/__init__.py",
                "pkg.with.dot/mod.name.pyi",
                "pkg.with.dot/sub.package/__init__.py",
                "pkg.with.dot/sub.package/worker.py",
                "standalone.dot/script.py",
            ],
        ))
        .unwrap();
    let observed = contexts
        .rows
        .iter()
        .map(|row| {
            (
                row.file.0,
                row.path.as_str(),
                row.module_name.as_str(),
                row.package_name.as_str(),
                row.package_root_name.as_str(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        observed,
        vec![
            (
                0,
                "pkg.with.dot.__init__",
                "__init__",
                "pkg.with.dot",
                "pkg.with.dot"
            ),
            (
                1,
                "pkg.with.dot.mod.name",
                "mod.name",
                "pkg.with.dot",
                "pkg.with.dot"
            ),
            (
                2,
                "pkg.with.dot.sub.package.__init__",
                "__init__",
                "sub.package",
                "pkg.with.dot"
            ),
            (
                3,
                "pkg.with.dot.sub.package.worker",
                "worker",
                "sub.package",
                "pkg.with.dot"
            ),
        ]
    );
    assert!(!contexts.partial);
    assert!(
        contexts
            .rows
            .iter()
            .all(|row| !row.module_name.contains('/')
                && !row.package_name.contains('/')
                && !row.package_root_name.contains('/'))
    );
}

#[test]
fn given_namespace_prefix_does_not_become_the_package_root_name() {
    let contexts = ModuleContextFilesystem
        .resolve(&request(
            Repository::Given,
            &[
                "namespace/pkg.root/__init__.py",
                "namespace/pkg.root/child.name/__init__.py",
                "namespace/pkg.root/child.name/__main__.py",
                "namespace/pkg.root/worker.pyi",
            ],
        ))
        .unwrap();
    assert!(!contexts.partial);
    assert_eq!(contexts.rows.len(), 4);
    for row in &contexts.rows {
        assert_eq!(row.package_root_name, "pkg.root");
        assert!(row.path.starts_with("pkg.root."));
        assert!(!row.path.contains("namespace"));
    }
    assert_eq!(contexts.rows[2].package_name, "child.name");
    assert_eq!(contexts.rows[2].module_name, "__main__");
}
