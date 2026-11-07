//! What every `templates` adapter must pass: plain text renders as written and reads nothing, a value is put in and is a
//! name it reads, and a template that does not parse is refused.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contexts::types::{ContextKey, ContextText, RenderContext};
use crate::domain::definitions::types::TemplateText;
use crate::outbound::templates::interface::TemplatesPort;

/// The template text a row renders.
type Input = &'static str;
/// What it rendered.
type Value = String;
/// The text of the port's refusal.
type Failure = String;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("plain_text_renders_as_written"),
        word: Word::Succeeds,
        input: || "pub struct Greeting;\n",
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_value_is_put_in"),
        word: Word::Succeeds,
        input: || "{{ part }}",
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_template_that_does_not_parse_is_refused"),
        word: Word::Refuses,
        input: || "{% if %}",
        expect: Expect::Refusal(|reason| {
            if reason.is_empty() {
                crate::cases::Selection::Differs
            } else {
                crate::cases::Selection::Matches
            }
        }),
    },
];

/// What `adapter` rendered `text` as, empty when it rendered and read what the contract expects.
fn answer(adapter: &impl TemplatesPort, text: Input) -> Result<Value, Failure> {
    let mut context = RenderContext::default();
    context
        .values
        .extend([(ContextKey("part".into()), ContextText("greeting".into()))]);
    let rendered = adapter
        .render(&TemplateText(text.into()), &context)
        .map_err(|error| error.to_string())?;
    let reads = adapter
        .reads(&TemplateText(text.into()))
        .map_err(|error| error.to_string())?;
    let (expected, read) = if text.contains("{{") {
        ("greeting", reads.0.contains(&ContextKey("part".into())))
    } else {
        (text, reads.0.is_empty())
    };
    Ok(if &*rendered.0 == expected && read {
        String::new()
    } else {
        rendered.0.into_string()
    })
}

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl TemplatesPort) {
    run(CASES, Word::Succeeds, |content| answer(adapter, content));
    run(CASES, Word::Refuses, |content| answer(adapter, content));
}
