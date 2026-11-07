//! The types of the ignore corpus adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::files::types::{FileSize, Nanoseconds};

/// What the system says of an open file: its size, when it was last written and when its inode last changed. A rewrite during a read
/// moves the change time even when the writer restores the write time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stamp {
    /// Its size in bytes.
    pub size: FileSize,
    /// When it was last written.
    pub written: Nanoseconds,
    /// When its inode last changed.
    pub changed: Nanoseconds,
}

/// How many bytes a read brought in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BytesRead(pub u64);

/// Whether a read was stable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stability {
    /// The file said the same after the read as before it, and the bytes read were its size.
    Stable,
    /// The file changed during the read, or the bytes read were not its size.
    Unstable,
}
