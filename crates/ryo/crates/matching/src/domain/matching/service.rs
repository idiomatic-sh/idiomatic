//! Matching algorithms; the caller owns every compiled value.

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use regex_automata::meta::Regex;

/// Compile a regex without acquiring or storing an engine cache entry.
pub(crate) fn compile_regex(pattern: &str) -> Result<Regex, String> {
    Regex::new(pattern).map_err(|error| {
        error.syntax_error().map_or_else(|| error.to_string(), ToString::to_string)
    })
}

/// Search using the caller's already acquired matcher.
pub(crate) fn matches_regex(regex: &Regex, text: &str) -> bool {
    regex.is_match(text)
}

/// Compile all globs into one set; one star never crosses a path separator.
pub(crate) fn compile_globs(globs: &[&str]) -> Result<GlobSet, (String, String)> {
    let mut builder = GlobSetBuilder::new();
    for glob in globs {
        let compiled = GlobBuilder::new(glob)
            .literal_separator(true)
            .build()
            .map_err(|error| ((*glob).to_owned(), error.to_string()))?;
        builder.add(compiled);
    }
    builder.build().map_err(|error| (String::new(), error.to_string()))
}

/// Validate a glob without constructing a request cache.
pub(crate) fn parse_glob(glob: &str) -> Result<(), String> {
    GlobBuilder::new(glob)
        .literal_separator(true)
        .build()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// Replace the caller's hit storage with the matching authored indices.
pub(crate) fn match_globs(set: &GlobSet, path: &std::path::Path, hits: &mut Vec<usize>) {
    set.matches_into(path, hits);
}

/// Visit nonoverlapping matches in source order, stopping at the first callback break.
pub(crate) fn visit_regex_matches(
    regex: &Regex,
    text: &str,
    each: &mut dyn FnMut(std::ops::Range<usize>) -> std::ops::ControlFlow<()>,
) -> std::ops::ControlFlow<()> {
    for found in regex.find_iter(text) {
        each(found.start()..found.end())?;
    }
    std::ops::ControlFlow::Continue(())
}

/// Validate the capture name before searching; a known nonparticipating group emits no row.
pub(crate) fn visit_regex_captures(
    regex: &Regex,
    text: &str,
    name: &str,
    each: &mut dyn FnMut(std::ops::Range<usize>, std::ops::Range<usize>) -> std::ops::ControlFlow<()>,
) -> Result<std::ops::ControlFlow<()>, String> {
    if regex.group_info().to_index(regex_automata::util::primitives::PatternID::ZERO, name).is_none() {
        return Err(format!("missing named capture `{name}`"));
    }
    for captures in regex.captures_iter(text) {
        let (Some(found), Some(group)) = (captures.get_match(), captures.get_group_by_name(name)) else { continue; };
        if let std::ops::ControlFlow::Break(()) = each(found.start()..found.end(), group.start..group.end) {
            return Ok(std::ops::ControlFlow::Break(()));
        }
    }
    Ok(std::ops::ControlFlow::Continue(()))
}
