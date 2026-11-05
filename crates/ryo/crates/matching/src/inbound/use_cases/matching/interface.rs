//! What matching promises callers without granting execution authority.

use super::compiled::{CompiledGlobs, CompiledRegex, CompiledTextGlobs};
use super::errors::MatchingError;

/// Pure regex and glob semantics over caller-owned values.
pub trait MatchingUseCase {
    /// Compile a regex. Invalid syntax is a refusal rather than an empty match.
    fn compile_regex(&self, pattern: &str) -> Result<CompiledRegex, MatchingError>;
    /// Search text with the matcher already acquired by the caller.
    fn matches_regex(&self, regex: &CompiledRegex, text: &str) -> bool;
    /// Visit source-order matches, stopping immediately when the caller breaks.
    fn visit_regex_matches(
        &self,
        regex: &CompiledRegex,
        text: &str,
        each: &mut dyn FnMut(std::ops::Range<usize>) -> std::ops::ControlFlow<()>,
    ) -> std::ops::ControlFlow<()>;
    /// Refuse an unknown name before scanning, skip nonparticipating known groups, and honor breaks.
    fn visit_regex_captures(
        &self,
        regex: &CompiledRegex,
        text: &str,
        name: &str,
        each: &mut dyn FnMut(
            std::ops::Range<usize>,
            std::ops::Range<usize>,
        ) -> std::ops::ControlFlow<()>,
    ) -> Result<std::ops::ControlFlow<()>, MatchingError>;
    /// Compile total text-glob semantics, retaining exact interpreter behavior when needed.
    fn compile_text_globs(&self, patterns: &[&str]) -> CompiledTextGlobs;
    /// Match any authored text glob; strategy selection stays inside the semantic value.
    fn matches_text_globs(&self, globs: &CompiledTextGlobs, text: &str) -> bool;
    /// Match a row-dependent text glob without acquiring or compiling cache state.
    fn matches_text_glob(&self, pattern: &str, text: &str) -> bool;
    /// Compile globs together, retaining authored index order.
    fn compile_globs(&self, globs: &[&str]) -> Result<CompiledGlobs, MatchingError>;
    /// Validate one glob before a run.
    fn parse_glob(&self, glob: &str) -> Result<(), MatchingError>;
    /// Replace hit storage with matching authored indices.
    fn match_globs(&self, globs: &CompiledGlobs, path: &std::path::Path, hits: &mut Vec<usize>);
}
