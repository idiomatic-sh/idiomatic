//! The `settings` service: the build that runs, told by its executable's bytes.

use std::hash::BuildHasher as _;

use super::constants::CHUNK;
use super::types::{Build, Executable, ProcessStart};
use crate::domain::packages::types::VersionText;

/// The build that runs: one hash of its `version` and of the bytes `executable` reads, so a rebuild of the same size and
/// modification time never serves another build's proofs. Every command hashes its own executable as it starts, so the
/// bytes are read [`CHUNK`] at a time into one buffer and hashed with foldhash, hashbrown's own hasher, each chunk
/// seeded with the hash of those before it: a 20 MB `tm` costs about 2 ms, where reading it whole and hashing it with
/// the standard library's `SipHash` cost 11. The first seed is fixed, so one executable is one build in every process.
///
/// # Errors
///
/// What reading `executable` fails with.
pub fn build(version: &VersionText, executable: &mut Executable<'_>) -> std::io::Result<Build> {
    let mut chunk = vec![0_u8; CHUNK];
    let mut hash = foldhash::quality::FixedState::with_seed(0).hash_one(version.0.as_bytes());
    loop {
        let mut filled = 0;
        while filled < chunk.len() {
            match executable
                .0
                .read(chunk.get_mut(filled..).unwrap_or_default())
            {
                Ok(0) => break,
                Ok(read) => filled += read,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        hash = foldhash::quality::FixedState::with_seed(hash)
            .hash_one(chunk.get(..filled).unwrap_or_default());
        if filled < chunk.len() {
            return Ok(Build(format!("{hash:016x}").into()));
        }
    }
}

/// The build of a process whose executable could not be read: a build of its own for this process alone, so it never
/// serves, and is never served by, another process's proofs.
#[must_use]
pub fn unread(version: &VersionText, process: ProcessStart) -> Build {
    let hash = foldhash::quality::FixedState::with_seed(u64::from(process.id.0))
        .hash_one((version.0.as_bytes(), process.started.0));
    Build(format!("{hash:016x}").into())
}
