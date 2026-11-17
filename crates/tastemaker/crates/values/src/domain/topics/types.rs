//! The types of the `topics` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// A topic `explain` answers that is no definition and no path: how a part of Tastemaker is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Topic {
    /// Its name, as `explain <topic>` names it.
    pub name: TopicName<'static>,
    /// What it says.
    pub body: TopicBody,
}

/// What a topic says: prose, or the keys of a definition file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopicBody {
    /// Markdown, as written.
    Text(TopicText),
    /// A definition file's keys, table by table, after its smallest correct file.
    Form(Form),
}

/// The name of a topic, as `explain <topic>` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TopicName<'a>(pub &'a str);

/// What a topic says, as Markdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TopicText(pub &'static str);

/// A definition file's keys: the smallest file that reads, then every table it may hold, then what else it says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Form {
    /// The file, as a package names it: `rule.toml`.
    pub file: TopicText,
    /// The smallest file the format-3 reader accepts: every required key and no other.
    pub skeleton: TopicText,
    /// Every table of the file, the file's own first.
    pub tables: &'static [KeyTable],
    /// Lines said after the tables, one fact each.
    pub notes: &'static [TopicText],
}

/// One table of a definition file and its keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyTable {
    /// The table as written: `rule.toml`, `[diagnostic]`, `[params.<name>]`.
    pub title: TopicText,
    /// Its keys, in the order the reader declares them, one line each: `name | shape | required or optional | what it is`.
    pub keys: TopicText,
}

/// One key of a table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    /// The key as written.
    pub name: TopicText,
    /// What its value is written as: `integer`, `string`, `selector list`.
    pub shape: TopicText,
    /// Whether the reader refuses a table without it.
    pub presence: Necessity,
    /// What it is, in a phrase.
    pub says: TopicText,
}

/// Whether the reader refuses a table without the key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Necessity {
    /// The table is refused without it.
    Required,
    /// The table reads without it.
    Optional,
}

/// A topic as Markdown: its skeleton and a line per key, or its prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopicPage(pub String);

/// A key as one Markdown line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyLine(pub String);
