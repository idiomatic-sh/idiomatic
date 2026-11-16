//! The limit on one name's events and reads, and whether a file's names stay within it.
/// The most events and reads of one name a file may hold.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NameChainLimit(pub usize);

/// Whether one name holds more events and reads than a limit allows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NameLoad {
    Within,
    Over,
}
