//! Acceptance steps call the production inbound interface directly.
use cucumber::{given, then, when};
use crate::Behavior;
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;

#[given(expr = "the matching input {string}")]
fn input(world: &mut Behavior, input: String) { world.input = input; }

#[when(expr = "matching runs")]
fn runs(world: &mut Behavior) {
    let matching = MatchingUseCaseImpl::new();
    world.answer = match world.input.as_str() {
        "regex-positive" | "regex-negative" => {
            let regex = matching.compile_regex("^ok$").expect("valid regex");
            let text = if world.input == "regex-positive" { "ok" } else { "prefix ok" };
            if matching.matches_regex(&regex, text) { "match" } else { "no match" }.to_owned()
        }
        "glob-separator" => {
            let globs = matching.compile_globs(&["*.rs", "**/*.rs"]).expect("valid globs");
            let mut hits = vec![999];
            matching.match_globs(&globs, std::path::Path::new("src/lib.rs"), &mut hits);
            assert_eq!(hits, [1]);
            hits.len().to_string()
        }
        "invalid-regex" => {
            assert!(matches!(matching.compile_regex("["), Err(MatchingError::InvalidRegex { reason }) if !reason.is_empty()));
            "invalid regex".to_owned()
        }
        "invalid-glob" => {
            assert!(matches!(matching.compile_globs(&["[ definition"]), Err(MatchingError::BadGlob { glob, reason }) if glob == "[ definition" && !reason.is_empty()));
            "invalid glob".to_owned()
        }
        _ => panic!("unknown matching input"),
    };
}

#[then(expr = "matching answers {string}")]
fn answers(world: &mut Behavior, expected: String) { assert_eq!(world.answer, expected); }
