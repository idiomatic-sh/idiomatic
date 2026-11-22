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

/// The most alternatives a set of required texts holds, and the most chars a class may stand for to be one: past either,
/// the set says too little to be worth a search, and none is given.
const REQUIRED_WIDTH: usize = 16;

/// The texts of which every match of the regex holds one, read off the parse the compiler reads (`util::syntax::parse`,
/// the syntax `meta::Regex::new` compiles): a literal is itself; a concatenation needs what its most specific part
/// needs; an alternation, one of each branch's; a repetition at least once, its body's; a small class, one of its chars.
/// None for an invalid regex, or one that can match without any (an empty match, a look-around, an optional part).
pub(crate) fn required_texts(pattern: AuthoredRegex<'_>) -> Option<Vec<Vec<u8>>> {
    let hir = regex_automata::util::syntax::parse(pattern.0).ok()?;
    let mut texts = required(&hir)?;
    texts.sort_unstable();
    texts.dedup();
    Some(texts)
}

/// What a part of a parse needs, as `required_texts` reads it.
fn required(hir: &regex_syntax::hir::Hir) -> Option<Vec<Vec<u8>>> {
    use regex_syntax::hir::{Class, HirKind};
    match hir.kind() {
        HirKind::Literal(literal) if !literal.0.is_empty() => Some(vec![literal.0.to_vec()]),
        HirKind::Capture(capture) => required(&capture.sub),
        HirKind::Repetition(repetition) if repetition.min >= 1 => required(&repetition.sub),
        HirKind::Class(Class::Unicode(class)) => {
            let mut texts = Vec::new();
            for range in class.ranges() {
                for each in range.start()..=range.end() {
                    if texts.len() >= REQUIRED_WIDTH {
                        return None;
                    }
                    let mut buffer = [0_u8; 4];
                    texts.push(each.encode_utf8(&mut buffer).as_bytes().to_vec());
                }
            }
            (!texts.is_empty()).then_some(texts)
        }
        HirKind::Class(Class::Bytes(class)) => {
            let mut texts = Vec::new();
            for range in class.ranges() {
                for each in range.start()..=range.end() {
                    if texts.len() >= REQUIRED_WIDTH {
                        return None;
                    }
                    texts.push(vec![each]);
                }
            }
            (!texts.is_empty()).then_some(texts)
        }
        // The part whose least specific text is longest; of those, the one with the fewest texts.
        HirKind::Concat(parts) => parts.iter().filter_map(required).max_by_key(|texts| {
            let shortest = texts.iter().map(Vec::len).min().unwrap_or(0);
            (shortest, std::cmp::Reverse(texts.len()))
        }),
        HirKind::Alternation(branches) => {
            let mut texts = Vec::new();
            for branch in branches {
                texts.extend(required(branch)?);
                if texts.len() > REQUIRED_WIDTH {
                    return None;
                }
            }
            Some(texts)
        }
        HirKind::Literal(_) | HirKind::Empty | HirKind::Look(_) | HirKind::Repetition(_) => None,
    }
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
        let _: &mut GlobSetBuilder = builder.add(compiled);
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
