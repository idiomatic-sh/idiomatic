//! Builds the syntax adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

use super::types::Expanding;

/// The `ra_ap_syntax` syntax adapter: rust-analyzer's parser, reading again the arguments of the macros `expanding` names.
#[must_use]
pub(crate) fn ra_ap_syntax(
    expanding: Expanding<'static>,
) -> impl crate::outbound::syntax::interface::SyntaxPort {
    super::adapters::ra_ap_syntax::adapter::SyntaxRaApSyntaxAdapter::new(expanding)
}
