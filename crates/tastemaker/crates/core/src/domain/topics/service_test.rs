//! What the `topics` service answers.

use super::service::{authoring, keys, names, text, topic, topics};
use super::types::{TopicBody, TopicName, TopicText};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contexts::constants::{
    CAPTURES, INSTANCE_PATH, INSTANCES, NAME, PRESENT, SCAFFOLD,
};

/// What a row asks of the topics.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// The topic of that name, said as Markdown.
    Topic(&'static str),
    /// The template topic names every map and value a template's content reads, by the names the context gives them.
    TemplateScope,
    /// Every `tm explain <topic>` the authoring steps name is a topic, or a kind followed by a name.
    AuthoringSteps,
    /// Every `tm create rule/…` and `tm create template/…` step names the extension `create` refuses to go without.
    CreateSteps,
    /// The scenarios topic shows the `tastemaker.toml` a scenario brings: a root, its dependency by path, an entry with no
    /// version.
    ScenarioWorkspace,
}

type Input = Ask;
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_topic_is_found_by_its_name"),
        word: Word::Succeeds,
        input: || Ask::Topic("selectors"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_form_topic_says_its_skeleton_then_a_line_per_key"),
        word: Word::Succeeds,
        input: || Ask::Topic("rule"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_template_topic_names_every_value_in_scope"),
        word: Word::Succeeds,
        input: || Ask::TemplateScope,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_authoring_steps_name_only_topics_that_exist"),
        word: Word::Succeeds,
        input: || Ask::AuthoringSteps,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_authoring_steps_create_a_rule_or_template_with_its_extension"),
        word: Word::Succeeds,
        input: || Ask::CreateSteps,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_scenarios_topic_shows_the_workspace_a_scenario_brings"),
        word: Word::Succeeds,
        input: || Ask::ScenarioWorkspace,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_name_no_topic_has_is_refused"),
        word: Word::Refuses,
        input: || Ask::Topic("registry"),
        expect: Expect::error(()),
    },
];

fn subject(ask: Input) -> Result<Value, Failure> {
    match ask {
        Ask::Topic(name) => said_as_markdown(name),
        Ask::TemplateScope => {
            let said = topic(TopicName("template"))
                .map(|found| text(&found).0)
                .unwrap_or_default();
            Ok(
                [CAPTURES, PRESENT, INSTANCES, NAME, INSTANCE_PATH, SCAFFOLD]
                    .into_iter()
                    .all(|name| said.contains(&format!("`{name}"))),
            )
        }
        Ask::AuthoringSteps => Ok(authoring()
            .into_iter()
            .flat_map(|line| {
                line.0
                    .split("tm explain ")
                    .skip(1)
                    .filter_map(|rest| rest.split([',', ' ']).next())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .filter(|asked| !asked.contains('/'))
            .all(|asked| names().iter().any(|name| name.0 == asked))),
        Ask::ScenarioWorkspace => {
            let said = topic(TopicName("scenarios"))
                .map(|found| text(&found).0)
                .unwrap_or_default();
            let brought = said
                .split("given.files.\"tastemaker.toml\" = '''\n")
                .nth(1)
                .and_then(|rest| rest.split("'''").next())
                .unwrap_or_default();
            Ok(brought.contains("workspace = { name = ")
                && brought.contains("dependencies = [{ package = ")
                && brought.contains("path = ")
                && brought.contains("rules = [{ rule = ")
                && !brought.contains('@'))
        }
        Ask::CreateSteps => {
            let creates: Vec<TopicText> = authoring()
                .into_iter()
                .filter(|line| {
                    line.0.contains("tm create rule/") || line.0.contains("tm create template/")
                })
                .collect();
            Ok(creates.len() == 2
                && creates
                    .iter()
                    .all(|line| line.0.ends_with(" --set extension=<ext>")))
        }
    }
}

/// Whether the topic called `name` is said as Markdown, each key a line.
fn said_as_markdown(name: &'static str) -> Result<Value, Failure> {
    let found = topic(TopicName(name)).ok_or(())?;
    let said = text(&found).0;
    let lines = match found.body {
        TopicBody::Text(_) => true,
        TopicBody::Form(form) => {
            said.contains(form.skeleton.0)
                && form.tables.iter().all(|table| {
                    keys(table)
                        .iter()
                        .all(|key| said.contains(&format!("- `{}` ", key.name.0)))
                })
        }
    };
    Ok(said.starts_with('#')
        && lines
        && topics().iter().any(|other| other.name == found.name)
        && names().iter().any(|other| other.0 == name))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
