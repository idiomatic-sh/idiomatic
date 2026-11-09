//! The subject the contract rows run: its inputs, the values it answers, and the one function that answers them through the public trait.
use super::super::errors::MatchingError;
use super::super::interface::MatchingUseCase;
use super::super::types::{
    GlobHits, GlobPattern, GroupName, MatchOutcome, RegexSource, SearchText,
};
use std::borrow::Cow;
use std::ops::ControlFlow;

#[derive(Debug)]
pub enum MatchingInput {
    Regex(&'static str, &'static str),
    Globs(&'static [&'static str], &'static str),
    Parse(&'static str),
    Ranges(&'static str, &'static str),
    Captures(&'static str, &'static str, &'static str),
    StopRanges(usize),
    StopCaptures(usize),
    TextGlob(&'static str, &'static str),
    TextGlobSet(&'static [&'static str], &'static str),
    DeepTextGlob,
    OwnedTextGlobs,
}

#[derive(Debug, PartialEq)]
pub enum MatchingValue {
    Match(bool),
    Hits(Cow<'static, [usize]>),
    Parsed,
    Ranges(Cow<'static, [std::ops::Range<usize>]>),
    Captures(Cow<'static, [(std::ops::Range<usize>, std::ops::Range<usize>)]>),
}

/// The value `input` answers through `matching`'s public trait, each call checked against its repeat.
///
/// # Errors
///
/// The [`MatchingError`] the trait refuses `input` with.
///
/// # Panics
///
/// When a repeated call answers differently, or a visitor runs after the caller broke or on a refused capture name.
pub fn answer(
    matching: &impl MatchingUseCase,
    input: MatchingInput,
) -> Result<MatchingValue, MatchingError> {
    match input {
        MatchingInput::Regex(pattern, text) => {
            let regex = matching.compile_regex(RegexSource(pattern))?;
            let first = matching.matches_regex(&regex, SearchText(text));
            assert_eq!(
                matching.matches_regex(&regex.clone(), SearchText(text)),
                first
            );
            Ok(MatchingValue::Match(first == MatchOutcome::Matches))
        }
        MatchingInput::Globs(patterns, path) => {
            let globs = matching.compile_globs(
                &patterns
                    .iter()
                    .map(|pattern| GlobPattern(pattern))
                    .collect::<Vec<_>>(),
            )?;
            let mut hits = GlobHits(vec![usize::MAX]);
            matching.match_globs(&globs, std::path::Path::new(path), &mut hits);
            let first = hits.clone();
            matching.match_globs(&globs.clone(), std::path::Path::new(path), &mut hits);
            assert_eq!(hits, first);
            Ok(MatchingValue::Hits(hits.0.into()))
        }
        MatchingInput::Ranges(pattern, text) => {
            let regex = matching.compile_regex(RegexSource(pattern))?;
            let mut ranges = Vec::new();
            let flow = matching.visit_regex_matches(&regex, SearchText(text), &mut |range| {
                ranges.push(range.0);
                ControlFlow::Continue(())
            });
            assert_eq!(flow, ControlFlow::Continue(()));
            Ok(MatchingValue::Ranges(ranges.into()))
        }
        MatchingInput::Captures(pattern, text, name) => {
            let regex = matching.compile_regex(RegexSource(pattern))?;
            let mut ranges = Vec::new();
            let result = matching.visit_regex_captures(
                &regex,
                SearchText(text),
                GroupName(name),
                &mut |whole, group| {
                    ranges.push((whole.0, group.0));
                    ControlFlow::Continue(())
                },
            );
            if result.is_err() {
                assert!(ranges.is_empty(), "invalid capture called the visitor");
            }
            assert_eq!(result?, ControlFlow::Continue(()));
            Ok(MatchingValue::Captures(ranges.into()))
        }
        MatchingInput::StopRanges(stop_at) => {
            let regex = matching.compile_regex(RegexSource("a"))?;
            let mut visits = 0;
            let mut ranges = Vec::new();
            let mut failure = None;
            let flow = matching.visit_regex_matches(&regex, SearchText("a a a"), &mut |range| {
                visits += 1;
                if visits == stop_at {
                    failure = Some("delivery failed");
                    return ControlFlow::Break(());
                }
                ranges.push(range.0);
                ControlFlow::Continue(())
            });
            assert_eq!(flow, ControlFlow::Break(()));
            assert_eq!(visits, stop_at, "visited after the downstream failure");
            assert_eq!(failure, Some("delivery failed"));
            Ok(MatchingValue::Ranges(ranges.into()))
        }
        MatchingInput::StopCaptures(stop_at) => {
            let regex = matching.compile_regex(RegexSource("(?P<letter>a)"))?;
            let mut visits = 0;
            let mut ranges = Vec::new();
            let mut failure = None;
            let flow = matching.visit_regex_captures(
                &regex,
                SearchText("a a a"),
                GroupName("letter"),
                &mut |whole, group| {
                    visits += 1;
                    if visits == stop_at {
                        failure = Some("delivery failed");
                        return ControlFlow::Break(());
                    }
                    ranges.push((whole.0, group.0));
                    ControlFlow::Continue(())
                },
            )?;
            assert_eq!(flow, ControlFlow::Break(()));
            assert_eq!(visits, stop_at, "visited after the downstream failure");
            assert_eq!(failure, Some("delivery failed"));
            Ok(MatchingValue::Captures(ranges.into()))
        }
        MatchingInput::TextGlob(pattern, text) => {
            let direct = matching.matches_text_glob(GlobPattern(pattern), SearchText(text));
            let compiled = matching.compile_text_globs(&[GlobPattern(pattern)]);
            assert_eq!(
                matching.matches_text_globs(&compiled, SearchText(text)),
                direct
            );
            Ok(MatchingValue::Match(direct == MatchOutcome::Matches))
        }
        MatchingInput::TextGlobSet(patterns, text) => {
            let compiled = matching.compile_text_globs(
                &patterns
                    .iter()
                    .map(|pattern| GlobPattern(pattern))
                    .collect::<Vec<_>>(),
            );
            Ok(MatchingValue::Match(
                matching.matches_text_globs(&compiled, SearchText(text)) == MatchOutcome::Matches,
            ))
        }
        MatchingInput::DeepTextGlob => {
            let pattern = format!("{}a{}", "{".repeat(300), "}".repeat(300));
            let compiled = matching.compile_text_globs(&[GlobPattern(&pattern)]);
            Ok(MatchingValue::Match(
                matching.matches_text_globs(&compiled, SearchText("a")) == MatchOutcome::Matches,
            ))
        }
        MatchingInput::OwnedTextGlobs => {
            let compiled = {
                let pattern = String::from("a?c");
                matching.compile_text_globs(&[GlobPattern(&pattern)])
            };
            Ok(MatchingValue::Match(
                matching.matches_text_globs(&compiled, SearchText("aéc")) == MatchOutcome::Matches,
            ))
        }
        MatchingInput::Parse(pattern) => matching
            .parse_glob(GlobPattern(pattern))
            .map(|()| MatchingValue::Parsed),
    }
}
