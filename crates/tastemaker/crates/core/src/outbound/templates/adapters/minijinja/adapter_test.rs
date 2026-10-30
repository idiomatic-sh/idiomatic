//! The minijinja adapter of the templates port: the contract, then its filters and maps.

use super::adapter::TemplatesMinijinjaAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contexts::types::{ContextKey, ContextText, Present, RenderContext};
use crate::domain::definitions::types::TemplateText;
use crate::outbound::templates::errors::TemplatesError;
use crate::outbound::templates::fixtures::contract::check;
use crate::outbound::templates::interface::TemplatesPort as _;

/// What a row asks of the adapter.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// Renders this template.
    Render(&'static str),
    /// The template topic names every filter and function the adapter adds to Jinja's own, by the name it adds it under.
    TopicNames,
    /// The names this template reads, joined: a name only a test reads, and each part of a nested lookup.
    Reads(&'static str),
}

type Input = Asked;
type Value = String;
type Failure = TemplatesError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_case_filters_change_case"),
        word: Word::Succeeds,
        input: || Asked::Render("{{ part | pascal }} {{ part | snake }}"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("present_reads_the_map"),
        word: Word::Succeeds,
        input: || Asked::Render("{% if present.dtos %}dtos{% endif %}"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_template_topic_names_every_filter_and_function_the_adapter_adds"),
        word: Word::Succeeds,
        input: || Asked::TopicNames,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_name_only_a_test_reads_and_each_part_of_a_lookup_are_read"),
        word: Word::Succeeds,
        input: || {
            Asked::Reads("{% if kind is not defined %}x{% endif %}{{ captures.lib | first }}")
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_template_that_refuses_renders_nothing"),
        word: Word::Refuses,
        input: || Asked::Render("{{ refuse('a value is missing') }}"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(TemplatesError::Render {
                    reason: "invalid operation: a value is missing (in <string>:1)".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_name_the_context_does_not_hold_is_refused"),
        word: Word::Refuses,
        input: || Asked::Render("{{ missing }}"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(TemplatesError::Unset {
                    reason: "undefined value (in <string>:1)".into(),
                    names: "`missing`".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_unset_param_is_named_as_it_is_written_kebab_case"),
        word: Word::Refuses,
        input: || Asked::Render("{{ item_trait }}"),
        expect: Expect::Refusal(|actual| {
            if matches!(actual, TemplatesError::Unset { names, .. } if names == "`item-trait`") {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("only_the_names_neither_given_nor_global_are_named_as_unset"),
        word: Word::Refuses,
        input: || {
            Asked::Render(
                "{% for each in range(1) %}{{ part }}{{ products }}{{ each }}{% endfor %}",
            )
        },
        expect: Expect::Refusal(|actual| {
            if actual
                == &(TemplatesError::Unset {
                    reason: "undefined value (in <string>:1)".into(),
                    names: "`products`".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

fn expected(content: &str) -> &'static str {
    if content.contains("present") {
        "dtos"
    } else {
        "GreetUser greet_user"
    }
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let text = match asked {
        Asked::Render(text) => text,
        Asked::TopicNames => return Ok(unnamed_in_the_topic().join(", ")),
        Asked::Reads(text) => {
            let reads = TemplatesMinijinjaAdapter::new()
                .reads(&TemplateText(text.into()))?
                .0
                .into_iter()
                .map(|key| key.0.to_string())
                .collect::<Vec<_>>()
                .join(",");
            return Ok(if reads == "captures,kind,lib" {
                String::new()
            } else {
                reads
            });
        }
    };
    let mut context = RenderContext::default();
    context
        .values
        .extend([(ContextKey("part".into()), ContextText("greet-user".into()))]);
    context
        .present
        .extend([(ContextKey("dtos".into()), Present::Yes)]);
    let rendered = TemplatesMinijinjaAdapter::new().render(&TemplateText(text.into()), &context)?;
    Ok(if &*rendered.0 == expected(text) {
        String::new()
    } else {
        rendered.0.into_string()
    })
}

#[test]
fn succeeds() {
    check(&TemplatesMinijinjaAdapter::new());
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// The names `constants.txt` declares that the template topic does not name, so a filter added there and not to the topic fails.
fn unnamed_in_the_topic() -> Vec<&'static str> {
    use crate::domain::topics::service::{text, topic};
    use crate::domain::topics::types::TopicName;
    let said = topic(TopicName("template"))
        .map(|found| text(&found).0)
        .unwrap_or_default();
    include_str!("constants.rs")
        .lines()
        .filter_map(|line| line.strip_prefix("pub const "))
        .filter_map(|line| line.split('"').nth(1))
        .filter(|name| !said.contains(&format!("`{name}")))
        .collect()
}
