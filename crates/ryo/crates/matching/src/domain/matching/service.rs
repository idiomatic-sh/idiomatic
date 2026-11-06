//! Matching algorithms; the caller owns every compiled value.

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use regex_automata::meta::Regex;

use super::types::{
    AuthoredIndices, AuthoredRegex, CaptureName, CompilerReason, GlobRefusal, Haystack, MatchSpan,
    PathGlob, RefusedGlob, RegexMatch,
};

/// Compile a regex without acquiring or storing an engine cache entry.
pub(crate) fn compile_regex(pattern: AuthoredRegex<'_>) -> Result<Regex, CompilerReason> {
    Regex::new(pattern.0).map_err(|error| {
        CompilerReason(
            error
                .syntax_error()
                .map_or_else(|| error.to_string(), ToString::to_string),
        )
    })
}

/// Search using the caller's already acquired matcher.
pub(crate) fn matches_regex(regex: &Regex, text: Haystack<'_>) -> RegexMatch {
    if regex.is_match(text.0) {
        RegexMatch::Matches
    } else {
        RegexMatch::Differs
    }
}

/// Compile all globs into one set; one star never crosses a path separator.
pub(crate) fn compile_globs(globs: &[PathGlob<'_>]) -> Result<GlobSet, GlobRefusal> {
    let mut builder = GlobSetBuilder::new();
    for glob in globs {
        let compiled = GlobBuilder::new(glob.0)
            .literal_separator(true)
            .build()
            .map_err(|error| GlobRefusal {
                glob: RefusedGlob(glob.0.to_owned()),
                reason: CompilerReason(error.to_string()),
            })?;
        builder.add(compiled);
    }
    builder.build().map_err(|error| GlobRefusal {
        glob: RefusedGlob(String::new()),
        reason: CompilerReason(error.to_string()),
    })
}

/// Validate a glob without constructing a request cache.
pub(crate) fn parse_glob(glob: PathGlob<'_>) -> Result<(), CompilerReason> {
    GlobBuilder::new(glob.0)
        .literal_separator(true)
        .build()
        .map(|_| ())
        .map_err(|error| CompilerReason(error.to_string()))
}

/// Replace the caller's hit storage with the matching authored indices.
pub(crate) fn match_globs(set: &GlobSet, path: &std::path::Path, hits: &mut AuthoredIndices) {
    set.matches_into(path, &mut hits.0);
}

/// Visit nonoverlapping matches in source order, stopping at the first callback break.
pub(crate) fn visit_regex_matches(
    regex: &Regex,
    text: Haystack<'_>,
    each: &mut dyn FnMut(MatchSpan) -> std::ops::ControlFlow<()>,
) -> std::ops::ControlFlow<()> {
    for found in regex.find_iter(text.0) {
        each(MatchSpan(found.start()..found.end()))?;
    }
    std::ops::ControlFlow::Continue(())
}

/// Validate the capture name before searching; a known nonparticipating group emits no row.
pub(crate) fn visit_regex_captures(
    regex: &Regex,
    text: Haystack<'_>,
    name: CaptureName<'_>,
    each: &mut dyn FnMut(MatchSpan, MatchSpan) -> std::ops::ControlFlow<()>,
) -> Result<std::ops::ControlFlow<()>, CompilerReason> {
    if regex
        .group_info()
        .to_index(regex_automata::util::primitives::PatternID::ZERO, name.0)
        .is_none()
    {
        return Err(CompilerReason(format!(
            "missing named capture `{}`",
            name.0
        )));
    }
    for captures in regex.captures_iter(text.0) {
        let (Some(found), Some(group)) = (captures.get_match(), captures.get_group_by_name(name.0))
        else {
            continue;
        };
        if let std::ops::ControlFlow::Break(()) = each(
            MatchSpan(found.start()..found.end()),
            MatchSpan(group.start..group.end),
        ) {
            return Ok(std::ops::ControlFlow::Break(()));
        }
    }
    Ok(std::ops::ControlFlow::Continue(()))
}
