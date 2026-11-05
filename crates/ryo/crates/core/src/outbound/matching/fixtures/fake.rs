//! The fake of the `matching` port: what the core's tests stand in for it with. It reads `*` as
//! anything but `/`, `**` as anything, and refuses a glob that contains `[`.

use std::sync::Arc;

use ryo_execution::inbound::use_cases::execution::types::matching::MatchingError;
use ryo_execution::inbound::use_cases::execution::types::matching::MatchingPort;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::files::types::{GlobsMatcher, Hits, MatchCandidate};

/// Whether the glob `pattern` matches all of `text`.
fn matches(pattern: &[char], text: &[char]) -> bool {
    match pattern {
        [] => text.is_empty(),
        ['*', '*', after @ ..] => {
            (matches!(after, ['/', ..]) && matches(after.get(1..).unwrap_or_default(), text))
                || (0..=text.len()).any(|end| matches(after, text.get(end..).unwrap_or_default()))
        }
        ['*', after @ ..] => {
            let reach = text
                .iter()
                .position(|&letter| letter == '/')
                .unwrap_or(text.len());
            (0..=reach).any(|end| matches(after, text.get(end..).unwrap_or_default()))
        }
        [literal, after @ ..] => match text {
            [first, rest @ ..] => first == literal && matches(after, rest),
            [] => false,
        },
    }
}

/// What the core's tests stand in for `matching` with.
#[derive(Clone, Copy, Debug)]
pub struct MatchingFake;

impl MatchingPort for MatchingFake {
    fn parse(&self, glob: &Glob) -> Result<(), MatchingError> {
        if glob.0.contains('[') {
            return Err(MatchingError::BadGlob {
                glob: glob.0.to_string(),
                reason: String::new(),
            });
        }
        Ok(())
    }

    fn compile(&self, globs: &[Glob]) -> Result<GlobsMatcher, MatchingError> {
        if let Some(glob) = globs.iter().find(|glob| glob.0.contains('[')) {
            return Err(MatchingError::BadGlob {
                glob: glob.0.to_string(),
                reason: String::new(),
            });
        }
        let globs = globs.to_vec();
        Ok(GlobsMatcher {
            matches: Arc::new(move |path: MatchCandidate<'_>, hits: &mut Hits| {
                hits.0.clear();
                let characters: Vec<char> = path.0.chars().collect();
                hits.0.extend(
                    globs
                        .iter()
                        .enumerate()
                        .filter(|(_, glob)| {
                            matches(&glob.0.chars().collect::<Vec<_>>(), &characters)
                        })
                        .map(|(place, _)| place),
                );
            }),
        })
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
