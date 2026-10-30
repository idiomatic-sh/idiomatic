//! The types of the `postcard` adapter: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether postcard left bytes after the value it read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Leftover {
    /// None: the bytes were exactly one value.
    Nothing,
    /// Some: the value was followed by more.
    More,
}
