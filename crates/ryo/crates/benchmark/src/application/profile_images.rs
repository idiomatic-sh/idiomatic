//! Cache an ordinary Cargo image and its allocator-observable companion.
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

pub(super) fn cargo_images(
    product: &Path,
    allocator_entry: &Path,
    binary: &str,
    stage: &Path,
    image: &Path,
) -> Result<(), Error> {
    let target = stage.join("target");
    let make = || {
        let mut command = Command::new("cargo");
        command
            .current_dir(product)
            .env("CARGO_PROFILE_BENCH_DEBUG", "line-tables-only")
            .args([
                "build",
                "--offline",
                "--profile",
                "bench",
                "--bin",
                binary,
                "--target-dir",
            ])
            .arg(&target);
        command
    };
    run(&mut make())?;
    let ordinary = stage.join("ordinary");
    fs::copy(target.join("release").join(binary), &ordinary)?;
    let entry = product.join(allocator_entry);
    let mut text = fs::read_to_string(&entry)?;
    for declaration in [
        "static GLOBAL: mimalloc::MiMalloc",
        "static GLOBAL: tikv_jemallocator::Jemalloc",
    ] {
        let (_, after) = text
            .split_once(declaration)
            .ok_or_else(|| Error::Invalid("pinned allocator declaration moved".into()))?;
        let start = text
            .find(declaration)
            .ok_or_else(|| Error::Invalid("allocator declaration".into()))?;
        let docs = text
            .get(..start)
            .and_then(|prefix| prefix.rfind("/// The allocator"))
            .ok_or_else(|| Error::Invalid("allocator declaration has unexpected shape".into()))?;
        let end = text.len() - after.len()
            + after
                .find('\n')
                .ok_or_else(|| Error::Invalid("allocator declaration is incomplete".into()))?;
        text.replace_range(docs..end, "");
    }
    fs::write(entry, text)?;
    run(&mut make())?;
    let heap = stage.join("heap");
    fs::copy(target.join("release").join(binary), &heap)?;
    fs::rename(ordinary, image.join("ordinary"))?;
    fs::rename(heap, image.join("heap"))?;
    Ok(())
}

pub(super) fn archive(repo: &Path, revision: &str, directory: &Path) -> Result<(), Error> {
    fs::create_dir_all(directory)?;
    let archive = directory.join("source.tar");
    run(Command::new("git")
        .current_dir(repo)
        .arg("archive")
        .arg("-o")
        .arg(&archive)
        .args([revision, "crates", "rust-toolchain.toml"]))?;
    run(Command::new("tar")
        .current_dir(directory)
        .arg("-xf")
        .arg(&archive))?;
    fs::remove_file(archive)?;
    Ok(())
}

pub(super) fn git_text(directory: &Path, arguments: &[&str]) -> Result<String, Error> {
    let output = Command::new("git")
        .current_dir(directory)
        .args(arguments)
        .output()?;
    if !output.status.success() {
        return Err(Error::Invalid(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| Error::Invalid(error.to_string()))
}

pub(super) fn run(command: &mut Command) -> Result<(), Error> {
    let status = command
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Invalid(format!("{command:?} exited {status}")))
    }
}
