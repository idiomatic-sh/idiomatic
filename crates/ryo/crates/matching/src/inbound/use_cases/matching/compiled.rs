//! Opaque semantic values. No scheduling, preparation or cache authority is carried here.

/// A compiled regex whose storage and lifetime belong to its caller.
#[derive(Clone, Debug)]
pub struct CompiledRegex {
    pub(crate) regex: regex_automata::meta::Regex,
}

/// A compiled set of globs in authored order.
#[derive(Clone, Debug)]
pub struct CompiledGlobs {
    pub(crate) set: globset::GlobSet,
}
