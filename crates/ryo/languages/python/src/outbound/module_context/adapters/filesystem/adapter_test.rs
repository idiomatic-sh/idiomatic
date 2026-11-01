//! Given context never uses a filesystem root; Disk lookup failures remain errors.
use super::adapter::ModuleContextFilesystem;
use crate::outbound::module_context::interface::ModuleContextPort as _;
use ryo::languages::{FilePath, ProjectError, ProjectRequest, Relation, Repository, RootPath};
fn request(repository: Repository, files: &[&str]) -> ProjectRequest {
    ProjectRequest {
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
