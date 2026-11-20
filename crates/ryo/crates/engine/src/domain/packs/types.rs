//! The types of the `packs` concept: a pack's bytes, lent or encoded, and the digest it is written for.

/// The digest of the rule sources, their fragments, the kind vocabulary, the anchor statistics and the engine version.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::packs::types::PackDigest;
///
/// assert_ne!(PackDigest([0; 32]), PackDigest([1; 32]));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackDigest(pub [u8; 32]);

/// Bytes of a compiled pack, lent by the host's store.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::packs::types::PackBytes;
///
/// assert_eq!(PackBytes(b"pack").0.len(), 4);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct PackBytes<'a>(pub &'a [u8]);

/// The bytes of an encoded rule set, for the host's store.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::packs::service::encoded;
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_engine::domain::packs::types::PackDigest;
///
/// assert!(!encoded(&Programs::default(), PackDigest([0; 32])).0.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedPack(pub Vec<u8>);
