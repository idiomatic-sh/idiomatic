//! Builds the syntax adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

use super::types::OpaqueMacros;

/// The `ra_ap_syntax` syntax adapter: rust-analyzer's parser, reading again the arguments of every macro call but those `opaque` names.
#[must_use]
pub(crate) fn ra_ap_syntax(
    opaque: OpaqueMacros<'static>,
) -> super::adapters::ra_ap_syntax::adapter::SyntaxRaApSyntaxAdapter {
    super::adapters::ra_ap_syntax::adapter::SyntaxRaApSyntaxAdapter::new(opaque)
}
