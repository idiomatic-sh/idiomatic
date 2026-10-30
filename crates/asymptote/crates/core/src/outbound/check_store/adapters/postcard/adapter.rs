//! `check`'s store: one postcard file of flat tables, read whole and replaced in one atomic write.
#[cfg(unix)]
use super::constants::ORDINARY_MODE;
use super::constants::{FILE, GITIGNORE, GITIGNORE_TEXT, TAG, TAG_TEXT};
use super::schemas::{read, write};
use super::settings::CheckStorePostcardSettings;
use crate::outbound::check_store::errors::CheckStoreError;
use crate::outbound::check_store::interface::CheckStorePort;
use crate::outbound::check_store::types::{Store, Stored};
use std::io::Write as _;

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

/// The store adapter: it owns its file.
#[derive(Debug)]
pub(crate) struct CheckStorePostcardAdapter {
    /// Its file and the build that runs.
    pub settings: CheckStorePostcardSettings,
}

impl CheckStorePostcardAdapter {
    /// The adapter over `store`: its file is named here, in the store's directory.
    pub(crate) fn new(store: Store) -> Self {
        Self {
            settings: CheckStorePostcardSettings {
                file: store.directory.join(FILE),
                build: store.build,
            },
        }
    }
}

impl CheckStorePort for CheckStorePostcardAdapter {
    fn load(&self) -> Stored {
        std::fs::read(&self.settings.file)
            .ok() // ignore: idiomatic.arch/no-ok-on-result - a store that cannot be read is no store: the run finds nothing kept and writes it anew
            .and_then(|bytes| read(&bytes, &self.settings.build))
            .unwrap_or_default()
    }

    fn keep(&self, stored: &Stored) -> Result<(), CheckStoreError> {
        let file = &self.settings.file;
        let directory = file
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default();
        let failure = |error: &dyn std::fmt::Display| CheckStoreError::Unwritable {
            directory: directory.display().to_string(),
            reason: error.to_string(),
        };
        std::fs::create_dir_all(&directory).map_err(|error| failure(&error))?;
        for (name, text) in [(TAG, TAG_TEXT), (GITIGNORE, GITIGNORE_TEXT)] {
            // Written once: a file that is there already is kept as it is.
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join(name))
            {
                Ok(mut created) => created
                    .write_all(text.as_bytes())
                    .map_err(|error| failure(&error))?,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(failure(&error)),
            }
        }
        let bytes = write(stored, &self.settings.build).map_err(|error| failure(&error))?;
        let mut writing = ordinary()
            .tempfile_in(&directory)
            .map_err(|error| failure(&error))?;
        writing.write_all(&bytes).map_err(|error| failure(&error))?;
        // The open handle is not needed: the store is read back by its name.
        drop(writing.persist(file).map_err(|error| failure(&error))?);
        Ok(())
    }
}
