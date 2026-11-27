//! The contracts of the language over its own ports, for the tests and benches outside the crate.

use std::sync::Arc;

use ryo::languages;

use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::syntax::factory::tree_sitter_go;

/// The `parse` contract, over the language's syntax port.
#[must_use]
pub(crate) fn parse() -> impl languages::Parse {
    ParseContract::new(Arc::new(tree_sitter_go()))
}
