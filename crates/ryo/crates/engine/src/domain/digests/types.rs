//! The types of the `digests` concept: what a step's digest is written into, and the walk's own count. Crate-private.

use ryo_values::domain::query_plans::types::{NodeDigest, NodeDigests};

/// How many times a step was digested: the walk's own count, which its case holds to one a step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StepsDigested(pub(crate) usize);

/// The digests of a plan's steps, and how many steps were digested for them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Digested {
    /// The digest of each step, in step order.
    pub(crate) digests: NodeDigests,
    /// How many steps were digested.
    pub(crate) steps: StepsDigested,
}

/// What a step's digest is written into: blake3 over the bytes written, every number at a fixed width, least byte first, so a
/// digest depends on neither the width of the machine that made it nor the build that made it.
#[derive(Clone, Debug, Default)]
pub(crate) struct DigestWriter(pub(crate) blake3::Hasher);

/// The digest of what was written: the first sixteen bytes of the hash.
impl From<&DigestWriter> for NodeDigest {
    fn from(writer: &DigestWriter) -> Self {
        let hash = writer.0.finalize();
        let mut digest = Self::default();
        for (byte, hashed) in digest.0.iter_mut().zip(hash.as_bytes()) {
            *byte = *hashed;
        }
        digest
    }
}

impl std::hash::Hasher for DigestWriter {
    fn finish(&self) -> u64 {
        let mut low = [0_u8; 8];
        for (byte, hashed) in low.iter_mut().zip(NodeDigest::from(self).0) {
            *byte = hashed;
        }
        u64::from_le_bytes(low)
    }

    fn write(&mut self, bytes: &[u8]) {
        let _hasher = self.0.update(bytes);
    }

    fn write_u16(&mut self, number: u16) {
        self.write(&number.to_le_bytes());
    }

    fn write_u32(&mut self, number: u32) {
        self.write(&number.to_le_bytes());
    }

    fn write_u64(&mut self, number: u64) {
        self.write(&number.to_le_bytes());
    }

    fn write_u128(&mut self, number: u128) {
        self.write(&number.to_le_bytes());
    }

    fn write_usize(&mut self, number: usize) {
        self.write_u64(u64::try_from(number).unwrap_or(u64::MAX));
    }

    fn write_i16(&mut self, number: i16) {
        self.write(&number.to_le_bytes());
    }

    fn write_i32(&mut self, number: i32) {
        self.write(&number.to_le_bytes());
    }

    fn write_i64(&mut self, number: i64) {
        self.write(&number.to_le_bytes());
    }

    fn write_i128(&mut self, number: i128) {
        self.write(&number.to_le_bytes());
    }

    fn write_isize(&mut self, number: isize) {
        self.write_i64(i64::try_from(number).unwrap_or(i64::MAX));
    }
}
