//! Data owned by declared rule configuration.

/// An exact path escaped as a literal glob for the tool and SDK scope interfaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiteralGlob(pub String);
