//! Linear links over the completed native token prefix, never a second lexer.
use super::constants;
use super::types::{LinkKind};

use ryo::languages::{NameText};

/// Public vocabulary belongs to the structural fact, not a linter.
pub(crate) const fn link_kind(kind: LinkKind) -> NameText<'static> {
    match kind {
        LinkKind::Next => NameText(constants::NEXT_TOKEN),
        LinkKind::Matching => NameText(constants::MATCHING_DELIMITER),
        LinkKind::Enclosing => NameText(constants::ENCLOSING_DELIMITER),
    }
}
