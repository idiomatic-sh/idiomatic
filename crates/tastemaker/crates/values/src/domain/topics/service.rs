//! The `topics` service: free functions named by verb over the topics `explain` answers, with no IO and no ports.

use super::constants::{AUTHORING, CONTENT_HEADING, OPTIONAL, PART, REQUIRED, RYOQL, TOPICS};
use super::types::{
    Form, Key, KeyLine, KeyTable, Necessity, Topic, TopicBody, TopicName, TopicPage, TopicText,
};

/// The topic of that name, when there is one.
#[must_use]
pub fn topic(name: TopicName<'_>) -> Option<Topic> {
    TOPICS.iter().find(|topic| topic.name.0 == name.0).copied()
}

/// Every topic.
#[must_use]
pub fn topics() -> &'static [Topic] {
    TOPICS
}

/// The keys of a table, as its lines declare them; a line that is not four parts is no key.
#[must_use]
pub fn keys(table: &KeyTable) -> Vec<Key> {
    table
        .keys
        .0
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, PART).map(str::trim);
            let name = parts.next()?;
            let shape = parts.next()?;
            let presence = match parts.next()? {
                REQUIRED => Necessity::Required,
                _ => Necessity::Optional,
            };
            let says = parts.next()?;
            Some(Key {
                name: TopicText(name),
                shape: TopicText(shape),
                presence,
                says: TopicText(says),
            })
        })
        .collect()
}

/// A key as one line: `- `name` shape, required: what it is`.
fn key_line(key: &Key) -> KeyLine {
    let presence = match key.presence {
        Necessity::Required => REQUIRED,
        Necessity::Optional => OPTIONAL,
    };
    KeyLine(format!(
        "- `{}` {}, {presence}: {}",
        key.name.0, key.shape.0, key.says.0
    ))
}

/// A definition file's keys as Markdown: its smallest correct file, then each table's keys, a line each.
fn form_text(form: &Form) -> TopicPage {
    let tables = form
        .tables
        .iter()
        .map(|table| {
            let lines = keys(table)
                .iter()
                .map(|key| format!("{}\n", key_line(key).0))
                .collect::<Vec<_>>()
                .concat();
            format!("\n## {}\n\n{lines}", table.title.0)
        })
        .collect::<Vec<_>>()
        .concat();
    let notes = if form.notes.is_empty() {
        String::new()
    } else {
        let lines = form
            .notes
            .iter()
            .map(|note| format!("{}\n", note.0))
            .collect::<Vec<_>>()
            .concat();
        format!("{CONTENT_HEADING}{lines}")
    };
    TopicPage(format!(
        "# {}\n\n```toml\n{}```\n{tables}{notes}",
        form.file.0, form.skeleton.0
    ))
}

/// What a topic says, as Markdown.
#[must_use]
pub fn text(topic: &Topic) -> TopicPage {
    match &topic.body {
        TopicBody::Text(text) => TopicPage(text.0.to_owned()),
        TopicBody::Form(form) => form_text(form),
    }
}

/// Every topic's name, `ryoql` with them, as `explain` names them.
#[must_use]
pub fn names() -> Vec<TopicName<'static>> {
    TOPICS
        .iter()
        .map(|topic| topic.name)
        .chain(std::iter::once(TopicName(RYOQL)))
        .collect()
}

/// How each kind of asset is written, a step a line.
#[must_use]
pub fn authoring() -> Vec<TopicText> {
    AUTHORING.to_vec()
}
