//! The subject the contract rows run: its inputs, the values it answers, and the one function that answers them through the public trait.
use super::super::errors::MatchingError;
use super::super::interface::MatchingUseCase;
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

pub fn answer(
    matching: &impl MatchingUseCase,
    input: MatchingInput,
) -> Result<MatchingValue, MatchingError> {
    match input {
        MatchingInput::Regex(pattern, text) => {
            let regex = matching.compile_regex(pattern)?;
            let first = matching.matches_regex(&regex, text);
            assert_eq!(matching.matches_regex(&regex.clone(), text), first);
            Ok(MatchingValue::Match(first))
        }
        MatchingInput::Globs(patterns, path) => {
            let globs = matching.compile_globs(patterns)?;
            let mut hits = vec![usize::MAX];
            matching.match_globs(&globs, std::path::Path::new(path), &mut hits);
            let first = hits.clone();
            matching.match_globs(&globs.clone(), std::path::Path::new(path), &mut hits);
            assert_eq!(hits, first);
            Ok(MatchingValue::Hits(hits.into()))
        }
        MatchingInput::Ranges(pattern, text) => {
            let regex = matching.compile_regex(pattern)?;
            let mut ranges = Vec::new();
            let flow = matching.visit_regex_matches(&regex, text, &mut |range| {
                ranges.push(range);
                ControlFlow::Continue(())
            });
            assert_eq!(flow, ControlFlow::Continue(()));
            Ok(MatchingValue::Ranges(ranges.into()))
        }
        MatchingInput::Captures(pattern, text, name) => {
            let regex = matching.compile_regex(pattern)?;
            let mut ranges = Vec::new();
            let result = matching.visit_regex_captures(&regex, text, name, &mut |whole, group| {
                ranges.push((whole, group));
                ControlFlow::Continue(())
            });
            if result.is_err() {
                assert!(ranges.is_empty(), "invalid capture called the visitor");
            }
            assert_eq!(result?, ControlFlow::Continue(()));
            Ok(MatchingValue::Captures(ranges.into()))
        }
        MatchingInput::StopRanges(stop_at) => {
            let regex = matching.compile_regex("a")?;
            let mut visits = 0;
            let mut ranges = Vec::new();
            let mut failure = None;
            let flow = matching.visit_regex_matches(&regex, "a a a", &mut |range| {
                visits += 1;
                if visits == stop_at {
                    failure = Some("delivery failed");
                    return ControlFlow::Break(());
                }
                ranges.push(range);
                ControlFlow::Continue(())
            });
            assert_eq!(flow, ControlFlow::Break(()));
            assert_eq!(visits, stop_at, "visited after the downstream failure");
            assert_eq!(failure, Some("delivery failed"));
            Ok(MatchingValue::Ranges(ranges.into()))
        }
        MatchingInput::StopCaptures(stop_at) => {
            let regex = matching.compile_regex("(?P<letter>a)")?;
            let mut visits = 0;
            let mut ranges = Vec::new();
            let mut failure = None;
            let flow =
                matching.visit_regex_captures(&regex, "a a a", "letter", &mut |whole, group| {
                    visits += 1;
                    if visits == stop_at {
                        failure = Some("delivery failed");
                        return ControlFlow::Break(());
                    }
                    ranges.push((whole, group));
                    ControlFlow::Continue(())
                })?;
            assert_eq!(flow, ControlFlow::Break(()));
            assert_eq!(visits, stop_at, "visited after the downstream failure");
            assert_eq!(failure, Some("delivery failed"));
            Ok(MatchingValue::Captures(ranges.into()))
        }
        MatchingInput::TextGlob(pattern, text) => {
            let direct = matching.matches_text_glob(pattern, text);
            let compiled = matching.compile_text_globs(&[pattern]);
            assert_eq!(matching.matches_text_globs(&compiled, text), direct);
            Ok(MatchingValue::Match(direct))
        }
        MatchingInput::TextGlobSet(patterns, text) => {
            let compiled = matching.compile_text_globs(patterns);
            Ok(MatchingValue::Match(
                matching.matches_text_globs(&compiled, text),
            ))
        }
        MatchingInput::DeepTextGlob => {
            let pattern = format!("{}a{}", "{".repeat(300), "}".repeat(300));
            let compiled = matching.compile_text_globs(&[&pattern]);
            Ok(MatchingValue::Match(
                matching.matches_text_globs(&compiled, "a"),
            ))
        }
        MatchingInput::OwnedTextGlobs => {
            let compiled = {
                let pattern = String::from("a?c");
                matching.compile_text_globs(&[&pattern])
            };
            Ok(MatchingValue::Match(
                matching.matches_text_globs(&compiled, "aéc"),
            ))
        }
        MatchingInput::Parse(pattern) => {
            matching.parse_glob(pattern).map(|()| MatchingValue::Parsed)
        }
    }
}
