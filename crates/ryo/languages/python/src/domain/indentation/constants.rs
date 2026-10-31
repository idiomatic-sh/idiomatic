//! A partial lexical stream cannot justify the absence of an indentation region.
pub(crate) const INCOMPLETE: &str =
    "indentation regions require a complete balanced lexical stream";
