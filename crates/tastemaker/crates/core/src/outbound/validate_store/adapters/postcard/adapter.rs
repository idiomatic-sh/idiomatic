//! The `validate_store` port, implemented with postcard: one file in the store's directory, read whole, and written whole
//! through a temporary file renamed into place. A file another build wrote, or one that does not read, holds nothing.

use std::io::Write as _;
use std::path::PathBuf;

#[cfg(unix)]
use super::constants::ORDINARY_MODE;
use super::constants::{FILE, GITIGNORE, GITIGNORE_TEXT, TAG, TAG_TEXT};
use super::schemas::{bytes, definitions};
use super::types::Unwritten;
use crate::domain::proofs::types::{ProvenVersion, ProvenVersions};
use crate::domain::settings::types::Store;
use crate::domain::writing::types::Writing;
use crate::outbound::validate_store::errors::ValidateStoreError;
use crate::outbound::validate_store::interface::ValidateStorePort;

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

/// The postcard `validate_store` adapter: where its file is, the build that writes it, and whether it is written.
#[derive(Debug)]
pub(crate) struct ValidateStorePostcardAdapter {
    /// Where the store is, the build that writes it, and whether it is written: injected, and kept as it came.
    store: Store,
    /// The file, in the store's directory.
    file: PathBuf,
    /// What a `-n/--dry-run` kept and did not write: the run that goes on to consult the proofs it just made (`check`
    /// proves first) reads them here, and nothing reaches the disk.
    unwritten: Unwritten,
}

impl ValidateStorePostcardAdapter {
    /// What a dry run kept and did not write, for the test of its bound.
    #[cfg(test)]
    pub(super) const fn unwritten(&self) -> &Unwritten {
        &self.unwritten
    }

    /// The adapter of the store at `store`, under its build.
    pub(crate) fn at(store: &Store) -> Self {
        Self {
            file: PathBuf::from(&*store.directory.0).join(FILE),
            store: store.clone(),
            unwritten: Unwritten::default(),
        }
    }
}

impl ValidateStorePort for ValidateStorePostcardAdapter {
    fn definitions(&self) -> ProvenVersions {
        if let Some(kept) = self
            .unwritten
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
        {
            return ProvenVersions(kept);
        }
        ProvenVersions(
            std::fs::read(&self.file)
                .ok() // ignore: idiomatic.arch/no-ok-on-result - a store that cannot be read is no store: validation proves again and writes it anew
                .and_then(|kept| definitions(&kept, &self.store.build))
                .unwrap_or_default(),
        )
    }

    fn keep_definitions(&self, rows: &[ProvenVersion]) -> Result<(), ValidateStoreError> {
        if self.store.writing == Writing::DryRun {
            *self
                .unwritten
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(rows.to_vec());
            return Ok(());
        }
        let unwritable = |reason: &dyn std::fmt::Display| ValidateStoreError::Unwritable {
            file: self.file.display().to_string(),
            reason: reason.to_string(),
        };
        let made = bytes(&self.store.build, rows).map_err(|error| unwritable(&error))?;
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
