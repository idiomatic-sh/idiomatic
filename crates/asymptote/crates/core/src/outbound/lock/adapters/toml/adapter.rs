//! The lock port, implemented with `toml` and `blake3`: one file per operation, named by the BLAKE3 digest of its LF
//! text, written once beside its place and renamed into it, never changed, and deleted only by the snapshot that
//! covers it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use super::constants::ORDINARY_MODE;
use super::constants::{CRLF, DIRECTORY, EXTENSION, ID_BYTES, LF, SUPPRESSED, TEMPORARY, UNNAMED};
use super::schemas::{is_hex, parse, render};
use super::settings::LockTomlSettings;
use crate::domain::manifest::types::{Pathspec, ProductRoot};
use crate::domain::operations::types::{Operation, OperationId, OperationText};
use crate::domain::violations::service::word;
use crate::domain::violations::types::{Id, Key, Standing};
use crate::outbound::lock::errors::LockError;
use crate::outbound::lock::interface::LockPort;
use crate::outbound::lock::types::{Listing, Operations};

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

/// The TOML lock adapter.
#[derive(Debug)]
pub(crate) struct LockTomlAdapter {
    /// The product whose lock it is.
    pub settings: LockTomlSettings,
}

/// The directory the product at `root` keeps its operation files in.
fn kept(root: &ProductRoot) -> PathBuf {
    Path::new(&*root.0).join(DIRECTORY)
}

/// The id a key's hash comes to: its first bytes, in lowercase hex.
fn id_of(hash: &blake3::Hash) -> Id {
    let hex = hash.to_hex();
    Id(hex.get(..ID_BYTES * 2).unwrap_or(hex.as_str()).into())
}

/// The content address of `text`.
fn address(text: &OperationText) -> OperationId {
    OperationId(blake3::hash(text.0.as_bytes()).to_hex().as_str().into())
}

/// Where the operation `id` of the product at `root` is kept.
fn file_of(root: &ProductRoot, id: &OperationId) -> PathBuf {
    kept(root).join(format!("{}.{EXTENSION}", id.0))
}

/// The operations whose files are in `directory`; none when it does not exist.
fn files(directory: &Path) -> Result<Vec<OperationId>, LockError> {
    let unreadable = |error: std::io::Error| LockError::Unreadable {
        path: directory.display().to_string(),
        reason: error.to_string(),
    };
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(unreadable(error)),
    };
    let mut found = Vec::new();
    for entry in entries {
        let path = entry.map_err(unreadable)?.path();
        // Every file of the directory is an operation: anything else is refused.
        let named = path.extension().and_then(|extension| extension.to_str()) == Some(EXTENSION);
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|_| named);
        let Some(stem) = stem else {
            return Err(LockError::Malformed {
                operation: path.display().to_string(),
                reason: UNNAMED.to_owned(),
            });
        };
        if !is_hex(stem, &[64]) {
            return Err(LockError::Malformed {
                operation: stem.to_owned(),
                reason: UNNAMED.to_owned(),
            });
        }
        found.push(OperationId(stem.into()));
    }
    found.sort();
    Ok(found)
}

impl LockTomlAdapter {
    /// The adapter over the lock of the product at `root`.
    pub(crate) const fn new(root: ProductRoot) -> Self {
        Self {
            settings: LockTomlSettings { root },
        }
    }

    /// The operation `text` holds, refused unless its LF bytes hash to the `id` it was found under.
    fn named(&self, id: &OperationId, text: &OperationText) -> Result<Operation, LockError> {
        // Git may check a text file out with CRLF; an operation is addressed by its LF bytes whatever the checkout.
        let canonical = OperationText(text.0.replace(CRLF, LF).into());
        let actual = address(&canonical);
        if &actual != id {
            return Err(LockError::Address {
                expected: id.0.to_string(),
                actual: actual.0.to_string(),
            });
        }
        parse(&canonical.0, &|key| self.id(key)).map_err(|error| LockError::Malformed {
            operation: id.0.to_string(),
            reason: error.to_string(),
        })
    }
}

impl LockPort for LockTomlAdapter {
    fn listing(&self) -> Result<Listing, LockError> {
        Ok(Listing {
            files: files(&kept(&self.settings.root))?,
        })
    }

    fn read(&self) -> Result<Operations, LockError> {
        let root = &self.settings.root;
        let operations = files(&kept(root))?
            .into_iter()
            .map(|id| {
                let path = file_of(root, &id);
                let text =
                    std::fs::read_to_string(&path).map_err(|error| LockError::Unreadable {
                        path: path.display().to_string(),
                        reason: error.to_string(),
                    })?;
                let operation = self.named(&id, &OperationText(text.into()))?;
                Ok((id, operation))
            })
            .collect::<Result<BTreeMap<_, _>, LockError>>()?;
        Ok(Operations(operations))
    }

    fn parse(&self, id: &OperationId, text: &OperationText) -> Result<Operation, LockError> {
        self.named(id, text)
    }

    fn append(&self, operation: &Operation) -> Result<(), LockError> {
        let text = render(operation);
        let id = address(&OperationText(text.as_str().into()));
        let directory = kept(&self.settings.root);
        let path = directory.join(format!("{}.{EXTENSION}", id.0));
        let unwritable = |error: std::io::Error| LockError::Unwritable {
            path: path.display().to_string(),
            reason: error.to_string(),
        };
        match std::fs::read_to_string(&path) {
            Ok(existing) if existing.replace(CRLF, LF) == text => return Ok(()),
            Ok(existing) => {
                return Err(LockError::Address {
                    expected: id.0.to_string(),
                    actual: address(&OperationText(existing.into())).0.to_string(),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(unwritable(error)),
        }
        std::fs::create_dir_all(&directory).map_err(unwritable)?;
        // An exclusive temporary file beside the operations, never among them, so
        // an interrupted append leaves nothing a reader takes for an operation, synced and then renamed into place. A
        // concurrent append of the same operation that renamed first leaves the same bytes, which is no failure.
        let parent = directory.parent().unwrap_or(&directory);
        // An operation is a file of the repository like any other, readable as `std::fs::write` would make it.
        let written = ordinary()
            .prefix(&format!(".{}.", id.0))
            .suffix(TEMPORARY)
            .tempfile_in(parent)
            .and_then(|mut temporary| {
                std::io::Write::write_all(&mut temporary, text.as_bytes())?;
                temporary.as_file().sync_all()?;
                temporary
                    .persist(&path)
                    .map(drop)
                    .map_err(|error| error.error)
            });
        match written {
            Ok(()) => Ok(()),
            Err(_) if std::fs::read_to_string(&path).is_ok_and(|existing| existing == text) => {
                Ok(())
            }
            Err(error) => Err(unwritable(error)),
        }
    }

    fn compact(&self, snapshot: &Operation) -> Result<(), LockError> {
        self.append(snapshot)?;
        for covered in &snapshot.covers {
            let path = file_of(&self.settings.root, covered);
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(LockError::Unwritable {
                        path: path.display().to_string(),
                        reason: error.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    fn directory(&self) -> Pathspec {
        Pathspec(DIRECTORY.into())
    }

    fn id(&self, key: &Key) -> Id {
        // Length-prefixed, so ("ab", "c") and ("a", "bc") cannot serialize alike.
        // Hashed in one call over the bytes a streaming hasher would be fed, which BLAKE3 digests alike.
        let mut bytes = Vec::new();
        for field in [&*key.tool.0, &*key.rule.0, &*key.path.0, word(key.level).0] {
            bytes.extend_from_slice(&u64::try_from(field.len()).unwrap_or(u64::MAX).to_le_bytes());
            bytes.extend_from_slice(field.as_bytes());
        }
        bytes.extend_from_slice(&key.fingerprint.0.to_le_bytes());
        if key.standing == Standing::Suppressed {
            bytes.extend_from_slice(SUPPRESSED);
        }
        id_of(&blake3::hash(&bytes))
    }
}
