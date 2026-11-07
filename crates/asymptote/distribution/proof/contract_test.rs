//! Independent mutations of target preparation: declared dependencies are isolated copies,
//! and no path or symlink escapes the target or the repository.
use super::preparation;

/// A pin whose one dependency binds `source`, in the repository, to `destination`, in the project.
fn pin(source: &str, destination: &str) -> toml::Table {
    toml::toml! { dependencies = [{ source = source, destination = destination }] }
}

#[test]
fn dependencies_are_isolated_external_packages_and_paths_cannot_escape() -> std::io::Result<()> {
    let scratch = tempfile::tempdir()?;
    let repository = scratch.path().join("repository");
    let project = scratch.path().join("target");
    let external = scratch.path().join("dependencies");
    std::fs::create_dir_all(repository.join("package"))?;
    std::fs::create_dir(&project)?;
    std::fs::write(repository.join("package/spec.toml"), "before")?;
    preparation::dependencies(
        &repository,
        &project,
        &external,
        &pin("package", ".benchmark/arch"),
    )?;
    let binding = project.join(".benchmark/arch");
    // Compared as canonical paths both sides: the temporary directory may be reached through a symbolic link (macOS's
    // /var is /private/var).
    assert!(!binding.canonicalize()?.starts_with(project.canonicalize()?));
    assert!(
        binding
            .canonicalize()?
            .starts_with(scratch.path().canonicalize()?)
    );
    std::fs::write(repository.join("package/spec.toml"), "after")?;
    assert_eq!(
        std::fs::read_to_string(binding.join("spec.toml"))?,
        "before"
    );
    std::fs::write(repository.join("configuration.lock"), "locked")?;
    preparation::dependencies(
        &repository,
        &project,
        &external,
        &pin("configuration.lock", "configuration.lock"),
    )?;
    assert_eq!(
        std::fs::read_to_string(project.join("configuration.lock"))?,
        "locked"
    );
    for bad in ["../outside", "/outside", "", "a/../../outside"] {
        assert!(
            preparation::dependencies(&repository, &project, &external, &pin(bad, "copy")).is_err()
        );
        assert!(
            preparation::dependencies(&repository, &project, &external, &pin("package", bad))
                .is_err()
        );
    }
    assert!(
        preparation::dependencies(
            &repository,
            &project,
            &external,
            &pin("package", ".benchmark/arch")
        )
        .is_err()
    );
    assert!(
        preparation::dependencies(
            &repository,
            &project,
            &external,
            &toml::toml! {dependencies = "wrong"}
        )
        .is_err()
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn dependency_source_and_destination_symlinks_cannot_escape() -> std::io::Result<()> {
    let scratch = tempfile::tempdir()?;
    let repository = scratch.path().join("repository");
    let project = scratch.path().join("target");
    let external = scratch.path().join("dependencies");
    std::fs::create_dir_all(repository.join("package"))?;
    std::fs::create_dir(&project)?;
    std::os::unix::fs::symlink(scratch.path(), repository.join("escape"))?;
    std::os::unix::fs::symlink(scratch.path(), project.join("escape"))?;
    assert!(
        preparation::dependencies(&repository, &project, &external, &pin("escape", "copy"))
            .is_err()
    );
    assert!(
        preparation::dependencies(
            &repository,
            &project,
            &external,
            &pin("package", "escape/copied")
        )
        .is_err()
    );
    std::os::unix::fs::symlink(scratch.path(), repository.join("package/escape"))?;
    assert!(
        preparation::dependencies(&repository, &project, &external, &pin("package", "copy"))
            .is_err()
    );
    Ok(())
}
