//! The `packs` service: a rule set's programs as bytes and back. A pack is the engine version, the digest it was written for, then
//! the programs (the rule count, then each rule's programs, the shared sub-programs, the made tables,
//! the constant pool and the regular expressions: the sections of `writing`). A pack read back holds no words: the load,
//! which has the vocabulary, makes them.

use super::constants::{COUNT_WIDTH, ENGINE_VERSION, VERSION_WIDTH};
use super::errors::PacksError;
use super::reading::service::read;
use super::types::{EncodedPack, PackBytes, PackDigest};
use super::writing::service::written;
use crate::domain::programs::types::{Programs, ProgramsEngine};
use ryo_relations::domain::relations::types::Count;

/// The bytes of `programs`, written for `digest` by this engine.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::packs::service::{decoded, encoded};
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_engine::domain::packs::types::{PackBytes, PackDigest};
///
/// let digest = PackDigest([7; 32]);
/// let bytes = encoded(&Programs::default(), digest);
/// assert!(decoded(PackBytes(&bytes.0), digest).is_ok());
/// ```
#[must_use]
pub fn encoded(programs: &Programs, digest: PackDigest) -> EncodedPack {
    let mut bytes = Vec::new();
    let version = ENGINE_VERSION.as_bytes();
    bytes.extend_from_slice(
        &u16::try_from(version.len())
            .unwrap_or(u16::MAX)
            .to_le_bytes(),
    );
    bytes.extend_from_slice(version);
    bytes.extend_from_slice(&digest.0);
    bytes.extend_from_slice(
        &u32::try_from(programs.0.code.rules.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    // A rule set of no rule is its header alone.
    if !programs.0.code.rules.is_empty() {
        written(&mut bytes, &programs.0);
    }
    EncodedPack(bytes)
}

/// Reads `width` bytes of `bytes` from `at` on, moving `at` past them.
fn take<'b>(
    bytes: PackBytes<'b>,
    at: &mut Count,
    width: Count,
) -> Result<PackBytes<'b>, PacksError> {
    let end = at.0.checked_add(width.0).ok_or(PacksError::Corrupt)?;
    let taken = bytes.0.get(at.0..end).ok_or(PacksError::Corrupt)?;
    *at = Count(end);
    Ok(PackBytes(taken))
}

/// The programs a pack holds, when it was written by this engine for `digest`.
///
/// # Errors
///
/// `Version` for a pack of another engine version, `Digest` for one written for other rules, `Corrupt` for
/// bytes that do not decode.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::packs::service::decoded;
/// use ryo_engine::domain::packs::errors::PacksError;
/// use ryo_engine::domain::packs::types::{PackBytes, PackDigest};
///
/// assert_eq!(decoded(PackBytes(b"no"), PackDigest([0; 32])).err(), Some(PacksError::Corrupt));
/// ```
pub fn decoded(bytes: PackBytes<'_>, digest: PackDigest) -> Result<Programs, PacksError> {
    let mut at = Count(0);
    let width = take(bytes, &mut at, VERSION_WIDTH)?.0;
    let width = Count(usize::from(u16::from_le_bytes(
        width.try_into().map_err(|_| PacksError::Corrupt)?,
    )));
    if take(bytes, &mut at, width)?.0 != ENGINE_VERSION.as_bytes() {
        return Err(PacksError::Version);
    }
    if take(bytes, &mut at, Count(digest.0.len()))?.0 != digest.0 {
        return Err(PacksError::Digest);
    }
    let count = take(bytes, &mut at, COUNT_WIDTH)?.0;
    let rest = bytes.0.get(at.0..).unwrap_or_default();
    // A rule set of no rule is its header alone; any other is its sections, read whole.
    if count == 0_u32.to_le_bytes() {
        return if rest.is_empty() {
            Ok(Programs(ProgramsEngine::default()))
        } else {
            Err(PacksError::Corrupt)
        };
    }
    let programs = read(rest).ok_or(PacksError::Corrupt)?;
    if u32::try_from(programs.code.rules.len()).unwrap_or(u32::MAX).to_le_bytes() != *count {
        return Err(PacksError::Corrupt);
    }
    Ok(Programs(programs))
}
