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
    let additional = receipt(
        &manifest,
        &root,
        vec![ryo::linter::ProjectDependency {
            path: ryo::linter::RelativePath("../undeclared".into()),
            state: ryo::linter::ProjectDependencyState::Missing,
        }],
    )?;
    assert_ne!(additional.manifest_hash, fingerprint(&manifest));
    verify_receipt(&manifest, &root, &additional)?;
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
fn observed_missing_paths_are_sealed_once_per_corpus_state() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let root = scratch.path();
    std::fs::write(root.join("source.py"), b"import pkg\n")?;
    let selected = vec![ryo::linter::RelativePath("source.py".into())];
    let tracked = vec!["source.py".into()];
    let initial = seal_existing(root, &selected, &tracked)?;
    assert!(initial.missing.is_empty());
    let absent = || {
        vec![ryo::linter::ProjectDependency {
            path: ryo::linter::RelativePath("pkg/__init__.py".into()),
            state: ryo::linter::ProjectDependencyState::Missing,
        }]
    };
    let cold = receipt(&initial, root, absent())?;
    let warm = receipt(&initial, root, absent())?;
    assert_eq!(cold, warm);
    assert_ne!(cold.manifest_hash, fingerprint(&initial));
    let mut effective = initial.clone();
    effective.missing.push("pkg/__init__.py".into());
    assert_eq!(cold.manifest_hash, fingerprint(&effective));
    verify_receipt(&initial, root, &cold)?;
    let another = ryo::linter::ProjectDependency {
        path: ryo::linter::RelativePath("pkg/marker.py".into()),
        state: ryo::linter::ProjectDependencyState::Missing,
    };
    let ordered = receipt(
        &initial,
        root,
        [absent()[0].clone(), another.clone()].into(),
    )?;
    let reversed = receipt(&initial, root, [another, absent()[0].clone()].into())?;
    assert_eq!(ordered, reversed, "dependency order cannot change the seal");
    let relocated = tempfile::tempdir()?;
    std::fs::write(relocated.path().join("source.py"), b"import pkg\n")?;
    let relocation_seal = seal_existing(relocated.path(), &selected, &tracked)?;
    let relocated_receipt = receipt(&relocation_seal, relocated.path(), absent())?;
    assert_eq!(relocated_receipt.manifest_hash, cold.manifest_hash);
    assert_ne!(relocated_receipt.actual_root, cold.actual_root);

    std::fs::create_dir(root.join("pkg"))?;
    std::fs::write(root.join("pkg/__init__.py"), b"now present")?;
    assert!(verify_receipt(&initial, root, &cold).is_err());
    std::fs::remove_file(root.join("pkg/__init__.py"))?;
    assert!(verify_receipt(&initial, root, &cold).is_ok());

    std::fs::write(root.join("source.py"), b"import pkg.more\n")?;
    let edited = seal_existing(root, &selected, &tracked)?;
    let edited_receipt = receipt(&edited, root, absent())?;
    assert_ne!(edited_receipt.manifest_hash, cold.manifest_hash);
    verify_receipt(&edited, root, &edited_receipt)?;
    assert!(verify_receipt(&initial, root, &edited_receipt).is_err());
    Ok(())
}
#[test]
fn observed_missing_cannot_hide_present_untracked_or_symlinked_paths() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let root = scratch.path();
    std::fs::write(root.join("source.py"), b"import pkg\n")?;
    let manifest = seal_existing(
        root,
        &[ryo::linter::RelativePath("source.py".into())],
        &["source.py".into()],
    )?;
    let missing = |path: &str| {
        vec![ryo::linter::ProjectDependency {
            path: ryo::linter::RelativePath(path.to_owned().into_boxed_str()),
            state: ryo::linter::ProjectDependencyState::Missing,
        }]
    };
    std::fs::write(root.join("untracked.py"), b"present")?;
    assert!(receipt(&manifest, root, missing("untracked.py")).is_err());
    assert!(receipt(&manifest, root, missing("source.py")).is_err());
    assert!(receipt(&manifest, root, missing("pkg/../source.py")).is_err());
    assert!(receipt(&manifest, root, missing("../escape.py")).is_err());
    assert!(receipt(&manifest, root, missing("untracked.py/child")).is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root, root.join("linked"))?;
        assert!(receipt(&manifest, root, missing("linked/absent.py")).is_err());
        std::os::unix::fs::symlink(root.join("absent-target"), root.join("dangling"))?;
        assert!(receipt(&manifest, root, missing("dangling")).is_err());
    }
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
