//! A partial lexical stream cannot justify the absence of an indentation region.
/// Why a partial token stream supplies no complete indentation-region facts.
pub(crate) const INCOMPLETE: &str =
    "indentation regions require a complete balanced lexical stream";
