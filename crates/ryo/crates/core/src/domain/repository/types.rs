//! The types of the `repository` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::keeping::types::DigestInput;
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::types::CellKind;

/// A column of a relation as the digest of the catalog sees it: its name and what it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DigestColumn<'a> {
    /// What the column is called in the query language.
    pub name: NameRef<'a>,
    /// What the column holds.
    pub kind: CellKind,
}

/// A relation as the digest of the catalog sees it: its name and its columns in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DigestRelation<'a> {
    /// What the relation is called in the query language.
    pub name: NameRef<'a>,
    /// Its columns, in the order the relation has them.
    pub columns: Vec<DigestColumn<'a>>,
}

/// A count or a length that the digest of the catalog writes as eight bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SchemaCount(pub usize);

/// What a value's derived `Hash` feeds, kept as bytes to digest: each number little endian at a fixed width (a length or an enum's
/// discriminant as eight bytes), so the bytes are the same on every platform and the digest port, not this, says what they come to.
#[derive(Debug)]
pub(crate) struct IdentityHasher(pub DigestInput);

impl std::hash::Hasher for IdentityHasher {
    /// A fast hash of the bytes fed so far; the identity is the bytes themselves, which the digest port digests.
    fn finish(&self) -> u64 {
        let mut fast = rustc_hash::FxHasher::default();
        fast.write(&self.0.0);
        fast.finish()
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.0.extend_from_slice(bytes);
    }

    fn write_u16(&mut self, value: u16) {
        self.write(&value.to_le_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.write(&value.to_le_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.write(&value.to_le_bytes());
    }

    fn write_u128(&mut self, value: u128) {
        self.write(&value.to_le_bytes());
    }

    fn write_usize(&mut self, value: usize) {
        self.write_u64(u64::try_from(value).unwrap_or(u64::MAX));
    }

    fn write_i16(&mut self, value: i16) {
        self.write(&value.to_le_bytes());
    }

    fn write_i32(&mut self, value: i32) {
        self.write(&value.to_le_bytes());
    }

    fn write_i64(&mut self, value: i64) {
        self.write(&value.to_le_bytes());
    }

    fn write_i128(&mut self, value: i128) {
        self.write(&value.to_le_bytes());
    }

    fn write_isize(&mut self, value: isize) {
        self.write_i64(i64::try_from(value).unwrap_or(i64::MAX));
    }
}
