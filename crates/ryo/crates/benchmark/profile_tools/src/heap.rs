//! Prepare a symbol-only sysroot for Heaptrack's offline decoder.
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, BufRead, BufReader},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};

pub(super) fn decode(directory: &Path) -> io::Result<()> {
    let trace = ["heaptrack.raw.gz", "heaptrack.raw.zst"]
        .into_iter()
        .map(|name| directory.join(name))
        .find(|path| path.is_file())
        .ok_or_else(|| io::Error::other("missing raw Heaptrack capture"))?;
    let sysroot = directory.join("function-symbols");
    fs::create_dir_all(&sysroot)?;
    for path in loaded_images(&trace)? {
        if !path.is_file() {
            continue;
        }
        let relative = path.strip_prefix("/").map_err(io::Error::other)?;
        if relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(io::Error::other("invalid Heaptrack image path"));
        }
        let copy = sysroot.join(relative);
        if let Some(parent) = copy.parent() {
            fs::create_dir_all(parent)?;
        }
        completed(
            Command::new("objcopy")
                .args([
                    "--strip-debug",
                    "--remove-section=.gnu_debuglink",
                    "--remove-section=.note.gnu.build-id",
                    "--wildcard",
                    "--strip-symbol=$x*",
                    "--strip-symbol=$d*",
                ])
                .arg(path)
                .arg(copy),
        )?;
    }
    completed(
        Command::new("time")
            .args(["-f", "%e %U %S %M", "-o"])
            .arg(directory.join("interpret-resources"))
            .args([
                "prlimit",
                "--as=2147483648",
                "--",
                "heaptrack",
                "--interpret",
            ])
            .arg(trace)
            .arg("--sysroot")
            .arg(sysroot)
            .env_remove("DEBUGINFOD_URLS")
            .stdout(Stdio::null())
            .stderr(File::create(directory.join("interpret-stderr"))?),
    )
}

/// Only module headers are read here. Allocation events and stack attribution
/// belong to the standard Heaptrack programs.
fn loaded_images(trace: &Path) -> io::Result<BTreeSet<PathBuf>> {
    let decoder = if trace
        .extension()
        .is_some_and(|extension| extension == "zst")
    {
        "zstd"
    } else {
        "gzip"
    };
    let mut child = Command::new(decoder)
        .arg("-dc")
        .arg(trace)
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing trace pipe"))?;
    let mut reader = BufReader::new(stdout);
    let mut bytes = Vec::new();
    let mut images = BTreeSet::new();
    let scan = (|| -> io::Result<()> {
        while reader.read_until(b'\n', &mut bytes)? != 0 {
            if let Some(path) = image_name(&bytes) {
                let path = PathBuf::from(path);
                if path.is_absolute() {
                    let _added = images.insert(path);
                }
            }
            bytes.clear();
        }
        Ok(())
    })();
    drop(reader);
    let status = child.wait()?;
    scan?;
    if !status.success() {
        return Err(io::Error::other(format!("{decoder} exited {status}")));
    }
    Ok(images)
}

fn image_name(record: &[u8]) -> Option<&str> {
    let rest = record
        .strip_prefix(b"m ")
        .or_else(|| record.strip_prefix(b"x "))?;
    let space = rest.iter().position(|byte| *byte == b' ')?;
    let length = usize::from_str_radix(std::str::from_utf8(&rest[..space]).ok()?, 16).ok()?;
    std::str::from_utf8(rest.get(space + 1..)?.get(..length)?).ok()
}

fn completed(command: &mut Command) -> io::Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{:?} exited {status}",
            command.get_program()
        )))
    }
}
