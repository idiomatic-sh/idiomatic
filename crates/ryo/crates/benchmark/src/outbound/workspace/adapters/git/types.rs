//! Git protocol inputs and outputs owned by this adapter boundary.

/// Arguments passed as separate OS command words.
pub(super) struct GitArguments<'a>(pub &'a [&'a str]);
/// Optional bytes supplied to the child standard input.
pub(super) struct GitInput<'a>(pub Option<&'a [u8]>);
/// Bytes captured from a successful command.
pub(super) struct GitOutput(pub Vec<u8>);
