//! Context validation and receipts reject unsealed filesystem state.
use super::adapter::*;
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use crate::domain::suite::types::{CaseContext, ContextManifest};
use std::{collections::BTreeMap, path::Path};
fn fixture(root: &Path) -> Result<ContextManifest, Error> {
    materialize(
        &root.join("boundary"),
        &CaseContext {
            logical_root: "pkg".into(),
            auxiliary_files: BTreeMap::from([("pkg/__init__.py".into(), b"".to_vec())]),
            missing: vec!["__init__.py".into()],
        },
        &BTreeMap::from([(
            ryo::linter::RelativePath("mod.py".into()),
            ryo::linter::SourceText("import pkg.mod\n".into()),
        )]),
    )
}
#[test]
fn seal_binds_bytes_absences_logical_root_and_observed_dependencies() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let manifest = fixture(scratch.path())?;
    let root = manifest.boundary.join("pkg");
    let dependencies = vec![
        ryo::linter::ProjectDependency {
            path: ryo::linter::RelativePath("__init__.py".into()),
            state: ryo::linter::ProjectDependencyState::Present(
                ryo::linter::dependency_fingerprint(b""),
            ),
        },
        ryo::linter::ProjectDependency {
            path: ryo::linter::RelativePath("../__init__.py".into()),
            state: ryo::linter::ProjectDependencyState::Missing,
        },
    ];
    let observed = receipt(&manifest, &root, dependencies)?;
    verify_receipt(&manifest, &root, &observed)?;
    let _verified_1 = receipt(
        &manifest,
        &root,
        vec![ryo::linter::ProjectDependency {
            path: ryo::linter::RelativePath("../undeclared".into()),
            state: ryo::linter::ProjectDependencyState::Missing,
        }],
    )
    .expect_err("expected refusal");
    let _verified_2 = receipt(
        &manifest,
        &root,
        vec![ryo::linter::ProjectDependency {
            path: ryo::linter::RelativePath("../../escape".into()),
            state: ryo::linter::ProjectDependencyState::Missing,
        }],
    )
    .expect_err("expected refusal");
    std::fs::write(root.join("__init__.py"), b"changed")?;
    let _verified_3 = validate(&manifest, &root).expect_err("expected refusal");
    std::fs::write(root.join("__init__.py"), b"")?;
    std::fs::write(manifest.boundary.join("__init__.py"), b"")?;
    let _verified_4 = validate(&manifest, &root).expect_err("expected refusal");
    Ok(())
}
#[test]
fn path_roles_and_symlinked_context_are_not_accepted() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let mut manifest = fixture(scratch.path())?;
    let root = manifest.boundary.join("pkg");
    manifest.missing.push("../escape".into());
    let _verified_5 = validate(&manifest, &root).expect_err("expected refusal");
    let _removed_missing = manifest.missing.pop();
    let _previous = manifest
        .auxiliary_files
        .insert("pkg/mod.py".into(), artifact(b"import pkg.mod\n"));
    let _verified_6 = validate(&manifest, &root).expect_err("expected refusal");
    let _removed_auxiliary = manifest.auxiliary_files.remove(Path::new("pkg/mod.py"));
    #[cfg(unix)]
    {
        std::fs::remove_file(root.join("__init__.py"))?;
        std::os::unix::fs::symlink(root.join("mod.py"), root.join("__init__.py"))?;
        let _verified_7 = validate(&manifest, &root).expect_err("expected refusal");
    }
    Ok(())
}
