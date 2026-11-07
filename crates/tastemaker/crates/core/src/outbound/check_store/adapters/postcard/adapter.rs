//! The `check_store` port, implemented with postcard: one file in the store's directory, read whole, and written whole
//! through a temporary file renamed into place. A file another build wrote, or one that does not read, holds nothing.

use std::io::Write as _;
use std::path::PathBuf;

#[cfg(unix)]
use super::constants::ORDINARY_MODE;
use super::constants::{FILE, GITIGNORE, GITIGNORE_TEXT, TAG, TAG_TEXT};
use super::schemas::{bytes, checked};
use crate::domain::settings::types::Store;
use crate::domain::writing::types::Writing;
use crate::outbound::check_store::errors::CheckStoreError;
use crate::outbound::check_store::interface::CheckStorePort;
use crate::outbound::check_store::types::Checked;

/// A temporary file that, renamed into place, is readable as any file `std::fs::write` makes: its mode is
/// `ORDINARY_MODE` less the umask, not the private `0o600` a temporary file starts with.
#[cfg(unix)]
fn ordinary() -> tempfile::Builder<'static, 'static> {
    let mut builder = tempfile::Builder::new();
    let _: &mut tempfile::Builder<'_, '_> = builder.permissions(
        <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(ORDINARY_MODE),
    );
    builder
}

/// A temporary file as any is made where files have no mode.
#[cfg(not(unix))]
fn ordinary() -> tempfile::Builder<'static, 'static> {
    tempfile::Builder::new()
}

/// The postcard `check_store` adapter: where its file is, the build that writes it, and whether it is written.
#[derive(Debug)]
pub(crate) struct CheckStorePostcardAdapter {
    /// Where the store is, the build that writes it, and whether it is written: injected, and kept as it came.
    store: Store,
    /// The file, in the store's directory.
    file: PathBuf,
}

impl CheckStorePostcardAdapter {
    /// The adapter of the store at `store`, under its build.
    pub(crate) fn at(store: &Store) -> Self {
        Self {
            file: PathBuf::from(&*store.directory.0).join(FILE),
            store: store.clone(),
        }
    }
}

impl CheckStorePort for CheckStorePostcardAdapter {
    fn checked(&self) -> Checked {
        std::fs::read(&self.file)
            .ok() // ignore: idiomatic.arch/no-ok-on-result - a store that cannot be read is no store: the check runs and writes it anew
            .and_then(|kept| checked(&kept, &self.store.build))
            .unwrap_or_default()
    }

    fn keep_checked(&self, checked: &Checked) -> Result<(), CheckStoreError> {
        if self.store.writing == Writing::DryRun {
            return Ok(());
        }
        let unwritable = |reason: &dyn std::fmt::Display| CheckStoreError::Unwritable {
            file: self.file.display().to_string(),
            reason: reason.to_string(),
        };
        let made = bytes(&self.store.build, checked).map_err(|error| unwritable(&error))?;
        // Through a temporary file in the directory renamed into place, so a reader sees the old store or the new, never
        // part of one.
        let write = || -> Result<(), std::io::Error> {
            std::fs::create_dir_all(std::path::Path::new(&*self.store.directory.0))?;
            for (name, said) in [(TAG, TAG_TEXT), (GITIGNORE, GITIGNORE_TEXT)] {
                // Written once: a file that is there already is kept as it is.
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(std::path::Path::new(&*self.store.directory.0).join(name))
                {
                    Ok(mut created) => created.write_all(said.as_bytes())?,
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error),
                }
            }
            let mut writing =
                ordinary().tempfile_in(std::path::Path::new(&*self.store.directory.0))?;
            writing.write_all(&made)?;
            drop(writing.persist(&self.file).map_err(|error| error.error)?);
            Ok(())
        };
        write().map_err(|error| unwritable(&error))
    }
}
