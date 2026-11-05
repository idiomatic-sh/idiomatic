//! The unread check of the conformance: a kind the parser produces that no relation reads is a finding, unless the language says
//! why no relation models it. Which kinds the readers look at is the readers' own declaration (`kinds:`), never a list written beside.

use ryo_relations::domain::relations::types::KindBits;
use crate::inbound::use_cases::parse::types::{KindSample, Sample};

/// What a language says it shows with `sample`, as the finding writes it.
fn shown(sample: Sample) -> &'static str {
    match sample {
        Sample::Node(source) | Sample::Token(source) => source.0,
        Sample::Unread(why) => why.0,
    }
}

/// The findings about the kinds of `language`: each kind with a sample that no reader of `read` looks at, and each kind marked unread
/// with no reason.
#[must_use]
pub fn findings(language: &str, kinds: &[KindSample], read: &[KindBits]) -> Vec<String> {
    let mut found = Vec::new();
    for each in kinds {
        match each.sample {
            Sample::Unread(reason) if reason.is_empty() => found.push(format!(
                "{language}: NodeKind::{kind} is marked Sample::Unread(\"\") with no reason: say why no relation models it",
                kind = each.kind
            )),
            Sample::Node(_) | Sample::Token(_) if !read.contains(&each.bits) => {
                found.push(format!(
                    "{language}'s parser produces `{kind}` (sample: `{sample}`), and no relation reads it. Either read it: add NodeKind::{kind} to a reader's `kinds:`. Or mark it: Sample::Unread(\"<why no relation models it>\") in sample().",
                    kind = each.kind,
                    sample = shown(each.sample).escape_debug()
                ));
            }
            Sample::Unread(_) | Sample::Node(_) | Sample::Token(_) => {}
        }
    }
    found
}

#[cfg(test)]
#[path = "unread_test.rs"]
mod tests;
