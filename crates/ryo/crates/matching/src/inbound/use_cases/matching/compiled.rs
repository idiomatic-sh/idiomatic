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

/// Compiled text-glob semantics with private exact-compiler or interpreter strategy.
/// Engine owners retain this value; it contains no cache key, permit or scheduling authority.
#[derive(Debug)]
pub struct CompiledTextGlobs {
    pub(crate) program: crate::domain::globbing::types::TextGlobProgram,
}

impl Default for CompiledTextGlobs {
    fn default() -> Self {
        Self {
            program: crate::domain::globbing::types::TextGlobProgram::Interpreted(Vec::new()),
        }
    }
}
