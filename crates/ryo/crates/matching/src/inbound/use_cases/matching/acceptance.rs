//! Acceptance steps call the production inbound interface directly.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase;
use ryo_matching::inbound::use_cases::matching::types::{
    GlobHits, GlobPattern, MatchOutcome, RegexSource, SearchText,
};
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;

#[given(expr = "the matching input {string}")]
fn given_input(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "matching runs")]
fn runs(world: &mut Behavior) {
    let matching = MatchingUseCaseImpl::new();
    world.answer = match world.input.as_str() {
        "regex-positive" | "regex-negative" => match matching.compile_regex(RegexSource("^ok$")) {
            Ok(regex) => {
                let text = if world.input == "regex-positive" {
                    "ok"
                } else {
                    "prefix ok"
                };
                if matching.matches_regex(&regex, SearchText(text)) == MatchOutcome::Matches {
                    "match"
                } else {
                    "no match"
                }
                .to_owned()
            }
            Err(refusal) => refusal.to_string(),
        },
        "glob-separator" => {
            match matching.compile_globs(&[GlobPattern("*.rs"), GlobPattern("**/*.rs")]) {
                Ok(globs) => {
                    let mut hits = GlobHits(vec![999]);
                    matching.match_globs(&globs, std::path::Path::new("src/lib.rs"), &mut hits);
                    assert_eq!(hits.0, [1], "only the separator-crossing glob matches");
                    hits.0.len().to_string()
                }
                Err(refusal) => refusal.to_string(),
            }
        }
        "text-star-crosses-separator"
        | "text-question-negative"
        | "text-malformed-brace-literal" => {
            let (pattern, text) = match world.input.as_str() {
                "text-star-crosses-separator" => ("src/*.rs", "src/a/b.rs"),
                "text-question-negative" => ("a?c", "aééc"),
                _ => ("{a", "{a"),
            };
            let globs = matching.compile_text_globs(&[GlobPattern(pattern)]);
            if matching.matches_text_globs(&globs, SearchText(text)) == MatchOutcome::Matches {
                "match"
            } else {
                "no match"
            }
            .to_owned()
        }
        "invalid-regex" => {
            assert!(
                matches!(matching.compile_regex(RegexSource("[")), Err(MatchingError::InvalidRegex { reason }) if !reason.is_empty())
            );
            "invalid regex".to_owned()
        }
        "invalid-glob" => {
            assert!(
                matches!(matching.compile_globs(&[GlobPattern("[ definition")]), Err(MatchingError::BadGlob { glob, reason }) if glob == "[ definition" && !reason.is_empty())
            );
            "invalid glob".to_owned()
        }
        _ => panic!("unknown matching input"),
    };
}

#[then(expr = "matching answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
