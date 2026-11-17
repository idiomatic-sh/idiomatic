//! The types of the `entries` concept: plain data, public fields and derives, with no constructor and no inherent impl.
//! An entry is one line of a `tastemaker.toml` or a spec: a rule, spec or template, where it applies and with what.

use std::collections::BTreeMap;

use crate::domain::packages::types::{DefinitionName, Ownership, PackageName, VersionText};
use crate::domain::scalars::types::{FlagValue, FloatText, TextValue, ValueKey, WholeValue};
use crate::domain::selectors::types::{PathPattern, Selectors, VariableName};

/// A count: how many of what an entry places.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Amount(pub u32);

/// The most a range admits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Bound {
    /// No most.
    Unbounded,
    /// At most this many.
    AtMost(Amount),
}

/// The name of a condition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConditionName(pub Box<str>);

/// How many an entry requires.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Count {
    /// Exactly this many.
    Exactly(Amount),
    /// At least `min`, at most `max`.
    Range {
        /// The fewest.
        min: Amount,
        /// The most.
        max: Bound,
    },
    /// `then` when the condition holds, `otherwise` when it does not.
    When {
        /// The condition.
        condition: ConditionName,
        /// When it holds.
        then: Amount,
        /// When it does not.
        otherwise: Amount,
    },
}

/// A value as a definition writes it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    /// Text.
    Text(TextValue),
    /// A whole number.
    Whole(WholeValue),
    /// A float, as written.
    Float(FloatText),
    /// A yes or no.
    Flag(FlagValue),
    /// A list.
    List(Vec<Value>),
    /// A table.
    Table(BTreeMap<ValueKey, Value>),
}

/// The name of a param.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParamName(pub Box<str>);

/// What an entry passes for a param.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParamValue {
    /// A value as written.
    Literal(Value),
    /// `{ param = "x" }`: the enclosing spec's param of that name.
    Param(ParamName),
    /// `{ variable = "x" }`: the variable of that name, resolved before placement.
    Variable(VariableName),
}

/// The values an entry passes, by name.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParamValues(pub BTreeMap<ParamName, ParamValue>);

/// A param's type, as written: a RyoQL parameter type for a rule, a template type otherwise.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParamType(pub Box<str>);

/// Where a param's value comes from when nothing passes one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParamSource {
    /// Only what an entry passes, or a capture of the same name.
    Given,
    /// The name of the directory the spec instance is applied to.
    DirectoryName,
    /// The name of the workspace, its `tastemaker.toml`'s `workspace.name`.
    WorkspaceName,
}

/// One line of text that says what something is.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Description(pub Box<str>);

/// Whether a description is written.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Described {
    /// It is.
    Said(Description),
    /// It is not: optional only in a `tastemaker.toml` of format 2.
    Unsaid,
}

/// One declared param.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParamDeclaration {
    /// Its type.
    pub kind: ParamType,
    /// What it means.
    pub description: Described,
    /// Where its value comes from when nothing passes one.
    pub source: ParamSource,
}

/// The params a definition declares, by name.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParamDeclarations(pub BTreeMap<ParamName, ParamDeclaration>);

/// An entry's local name, when two entries share one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Alias {
    /// It has this alias.
    Named(DefinitionName),
    /// It is named by its definition.
    Unnamed,
}

/// A module pattern an `imports` allow-list names: `std::*`, or an entry's reference.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImportPattern(pub Box<str>);

/// Whether imports are allowed unless listed, or denied unless listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImportDefault {
    /// Allowed unless listed.
    Allow,
    /// Denied unless listed in `allow`.
    Deny,
}

/// An entry's `imports`, when it has one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Imports {
    /// It has none.
    Unrestricted,
    /// Its default and its allow-list.
    Restricted {
        /// The default.
        default: ImportDefault,
        /// What it allows.
        allow: Vec<ImportPattern>,
    },
}

/// What every entry carries.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryHeader {
    /// The package its definition comes from: the spec's own for an owned definition.
    pub package: PackageName,
    /// The version it follows: the spec's own for an owned definition.
    pub version: VersionText,
    /// Shared, or owned by the spec that writes the entry.
    pub ownership: Ownership,
    /// Its local name.
    pub alias: Alias,
    /// What it is here.
    pub description: Described,
    /// Where: a rule's or spec's `include`; empty for a template entry, which places its `file`.
    pub include: Selectors,
    /// What is taken from `include` or from the template's matches.
    pub exclude: Selectors,
    /// What it passes.
    pub params: ParamValues,
}

/// A rule entry.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleEntry {
    /// What every entry carries.
    pub header: EntryHeader,
    /// The rule.
    pub rule: DefinitionName,
}

/// A spec entry.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpecEntry {
    /// What every entry carries.
    pub header: EntryHeader,
    /// The spec.
    pub spec: DefinitionName,
    /// How many directories it applies at.
    pub count: Count,
    /// What its instances' code may import.
    pub imports: Imports,
}

/// A template entry.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplateEntry {
    /// What every entry carries.
    pub header: EntryHeader,
    /// The template.
    pub template: DefinitionName,
    /// The file it places: a path, or a pattern when many files share the shape.
    pub file: PathPattern,
    /// How many files, or captured values.
    pub count: Count,
    /// What its files may import.
    pub imports: Imports,
}

/// A named yes or no, answered by a rule entry: yes when the rule selects a row within the entry's `include`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Condition {
    /// What it answers.
    pub description: Described,
    /// The rule entry that answers it.
    pub entry: RuleEntry,
}

/// What a v2 file writes of the import order that Tastemaker folds into `import-directions` placements: kept so
/// a check can say each one is to be converted, never lost.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ordering {
    /// How many groups `layers` has.
    pub layers: Amount,
    /// How many entries `independent` names.
    pub independent: Amount,
    /// Whether `acyclic = true` is written.
    pub acyclic: Acyclicity,
}

/// Whether a file writes `acyclic = true`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Acyclicity {
    /// It does not.
    #[default]
    Unwritten,
    /// It does.
    Written,
}

/// A named value of a `tastemaker.toml` or spec, shared by the entries that refer to it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variables(pub BTreeMap<VariableName, Value>);

/// What a `tastemaker.toml` and a spec compose: their entries, conditions and variables.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Composition {
    /// Whether only placed files may exist.
    pub closed: ClosedChoice,
    /// The rule entries.
    pub rules: Vec<RuleEntry>,
    /// The spec entries.
    pub specs: Vec<SpecEntry>,
    /// The template entries.
    pub templates: Vec<TemplateEntry>,
    /// The conditions, by name.
    pub conditions: BTreeMap<ConditionName, Condition>,
    /// The variables, by name.
    pub variables: Variables,
    /// Module patterns this directory adds to the `allow` of every instance placed at it.
    pub imports_allow: Vec<ImportPattern>,
    /// The variables whose lists `imports-allow` names, put in place by the variables pass.
    pub imports_allow_variables: Vec<VariableName>,
    /// The import order a v2 file writes, folded into `import-directions` placements.
    pub ordering: Ordering,
}

/// `closed` as written: left out is closed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClosedChoice {
    /// Left out: the default.
    #[default]
    Closed,
    /// `closed = false`.
    Open,
}

/// How output names an entry: `package/name`, `package/spec/name` for an owned definition, with `(alias)` when it has one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryIdentity(pub String);

/// Which branch of a conditional count its condition picks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Branch {
    /// The condition holds: the `then` count.
    Then,
    /// It does not: the `else` count.
    Otherwise,
}

/// A count as a message writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountText(pub String);

/// A value as it reads in a path or a template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueText(pub String);

/// What writes a value as a path or a template reads it, as the definitions' format writes it: the manifest port's.
pub trait ValueWriter: Sync {
    /// `value`, written.
    fn write(&self, value: &Value) -> ValueText;
}

impl std::fmt::Debug for dyn ValueWriter + '_ {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::any::type_name::<Self>())
    }
}

impl<F: Fn(&Value) -> ValueText + Sync> ValueWriter for F {
    fn write(&self, value: &Value) -> ValueText {
        self(value)
    }
}

impl std::fmt::Display for EntryIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Display for CountText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Display for ValueText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<EntryIdentity> for Box<str> {
    fn from(identity: EntryIdentity) -> Self {
        identity.0.into()
    }
}

/// A value given on the command line, `--set name=value` or `--param name=value`: a name or a value of it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Setting(pub Box<str>);

/// An entry's public reference and the definition release it pins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedRelease {
    /// Its alias, or the definition's name when no alias is given.
    pub entry: DefinitionName,
    /// The release the entry names.
    pub release: crate::domain::packages::types::Release,
}
