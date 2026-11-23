//! The names the `join` operation's lowering writes.

/// The operation's name, as a query calls it.
pub(crate) const JOIN: &str = "join";
/// The alias the left source is read under.
pub(crate) const LEFT_ALIAS: &str = "l";
/// The alias the right source is read under.
pub(crate) const RIGHT_ALIAS: &str = "r";
