// ignore: idiomatic.arch/file-length - the adapter's one wire format: every definition file Tastemaker reads (`tastemaker.toml`, `spec.toml`, `rule.toml`, `template.toml`, `package.toml`, `tastemaker.lock`) is one family of TOML schemas, read through serde and written through toml_edit with the same keys; the adapter spec places one schemas.rs for them.
//! The shape of the data this adapter does not control: each wire shape is a `<Name>Schema`, read by serde, and its
//! checked conversion into the core's type is an `impl TryFrom`. Every definition is one file, its `[[scenarios]]` and its
//! inline `content` in it.

use std::collections::{BTreeMap, BTreeSet};

use super::constants::{
    DENY, DIRECTORY_NAME, LEVEL_KEY, LOCK_HEAD, LOCK_VERSION, PLACED_ALONE, SCENARIO_KEY,
    SCENARIO_WORKSPACE, SPEC_DROPPED, UNKNOWN_FIELD, VERSION_KEY, WORKSPACE_NAME,
};
use crate::domain::definitions::constants::WORKSPACE_FILE;
use crate::domain::definitions::types::{
    ColumnName, Diagnostic, Exactness, Exception, QueryText, Role, RuleFile, SpecFile,
    TemplateFile, TemplateText, Why, WorkspaceFile, WorkspaceSection,
};
use crate::domain::entries::service::derived;
use crate::domain::entries::types::{
    Acyclicity, Alias, Amount, Bound, ClosedChoice, Composition, Condition, ConditionName, Count,
    Described, Description, EntryHeader, ImportDefault, ImportPattern, Imports, Ordering,
    ParamDeclaration, ParamDeclarations, ParamName, ParamSource, ParamType, ParamValue,
    ParamValues, RuleEntry, SpecEntry, TemplateEntry, Value, Variables,
};
use crate::domain::files::types::FileText;
use crate::domain::findings::types::{MessageTemplate, WorkspaceName};
use crate::domain::packages::constants::{
    PACKAGE_KEY, RULE_WORD, RULES, SPEC_WORD, SPECS, TASTEMAKER_VERSION, TEMPLATE_WORD, TEMPLATES,
};
use crate::domain::packages::types::{
    DefinitionKind, DefinitionName, Dependency, GitSource, Lock, LockedPackage, LockedRelease,
    Ownership, PackageManifest, PackageName, PackageSource, RepositoryUrl, Revision, SourcePath,
    VersionText,
};
use crate::domain::scalars::types::{FlagValue, FloatText, TextValue, ValueKey, WholeValue};
use crate::domain::scenario_files::types::{
    CountKey, FileKey, ScenarioKeys, ScenarioRoot, Wrapper,
};
use crate::domain::scenarios::types::{
    Expectation, ExpectedFinding, ExpectedLine, ExpectedRule, ExpectedRuleChoice, RuleScenario,
    ScenarioFileText, ScenarioFiles, ScenarioId, ScenarioLine, SpecScenario, TemplateFileChoice,
    TemplateScenario,
};
use crate::domain::selectors::types::{
    PartSelector, PathPattern, PlacedBySelector, Recursion, RegexText, Selector, Selectors,
    VariableName,
};
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::manifest::types::{DefinitionText, EntryDraft};

/// Wraps the reason a text was refused as the manifest's shape error.
fn shape(reason: impl std::fmt::Display) -> ManifestError {
    ManifestError::Shape {
        reason: reason.to_string(),
    }
}

/// A selector as written: a glob string, or a table of one of the other kinds.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
pub(super) enum SelectorSchema {
    /// A glob over paths.
    Glob(String),
    /// A regular expression over paths.
    Regex {
        /// The expression.
        regex: String,
    },
    /// A named part of the definition's layout.
    Part {
        /// The part's name.
        part: String,
        #[serde(default)]
        /// Whether the part is searched through its descendants.
        recursive: bool,
    },
    /// The files placed by another entry.
    PlacedBy {
        #[serde(rename = "placed-by")]
        /// The name of the placing entry.
        placed_by: String,
    },
    /// A variable holding the selectors.
    Variable {
        /// The variable's name.
        variable: String,
    },
}

impl From<SelectorSchema> for Selector {
    fn from(schema: SelectorSchema) -> Self {
        match schema {
            SelectorSchema::Glob(text) => Self::Glob(PathPattern(text.into())),
            SelectorSchema::Regex { regex } => Self::Regex(RegexText(regex.into())),
            SelectorSchema::Part { part, recursive } => Self::Part(PartSelector {
                part: DefinitionName(part.into()),
                recursion: if recursive {
                    Recursion::Recursive
                } else {
                    Recursion::Direct
                },
            }),
            SelectorSchema::PlacedBy { placed_by } => Self::PlacedBy(PlacedBySelector {
                entry: DefinitionName(placed_by.into()),
            }),
            SelectorSchema::Variable { variable } => Self::Variable(VariableName(variable.into())),
        }
    }
}

/// Reads an optional list of selector tables as the core's selectors, none when absent.
fn selectors(list: Option<Vec<SelectorSchema>>) -> Selectors {
    Selectors(
        list.unwrap_or_default()
            .into_iter()
            .map(Selector::from)
            .collect(),
    )
}

/// A value as TOML gives it, read into the core's.
pub(super) fn value(written: toml::Value) -> Value {
    match written {
        toml::Value::String(text) => Value::Text(TextValue(text.into())),
        toml::Value::Integer(number) => Value::Whole(WholeValue(number)),
        toml::Value::Float(number) => Value::Float(FloatText(number.to_string().into())),
        toml::Value::Boolean(flag) => {
            Value::Flag(if flag { FlagValue::Yes } else { FlagValue::No })
        }
        toml::Value::Datetime(time) => Value::Text(TextValue(time.to_string().into())),
        toml::Value::Array(items) => Value::List(items.into_iter().map(value).collect()),
        toml::Value::Table(table) => Value::Table(
            table
                .into_iter()
                .map(|(key, item)| (ValueKey(key.into()), value(item)))
                .collect(),
        ),
    }
}

/// A param value as it reads in a path or a template: text and a float as written, anything else as `toml_edit` writes
/// the value inline (`[\"a\", \"b\"]`, `{ key = 1 }`).
pub(super) fn value_text(written: &Value) -> crate::domain::entries::types::ValueText {
    crate::domain::entries::types::ValueText(match written {
        Value::Text(text) => text.0.to_string(),
        Value::Float(text) => text.0.to_string(),
        Value::Whole(_) | Value::Flag(_) | Value::List(_) | Value::Table(_) => {
            inline(written).to_string()
        }
    })
}

/// A param value as a TOML value: a float keeps the text it was read as.
fn inline(written: &Value) -> toml_edit::Value {
    match written {
        Value::Text(text) => toml_edit::Value::from(&*text.0),
        Value::Whole(number) => toml_edit::Value::from(number.0),
        Value::Float(text) => text
            .0
            .parse::<toml_edit::Value>()
            .unwrap_or_else(|_unparsed| toml_edit::Value::from(&*text.0)),
        Value::Flag(FlagValue::Yes) => toml_edit::Value::from(true),
        Value::Flag(FlagValue::No) => toml_edit::Value::from(false),
        Value::List(items) => {
            toml_edit::Value::Array(items.iter().map(inline).collect::<toml_edit::Array>())
        }
        Value::Table(table) => toml_edit::Value::InlineTable(
            table
                .iter()
                .map(|(key, item)| (&*key.0, inline(item)))
                .collect::<toml_edit::InlineTable>(),
        ),
    }
}

/// What an entry passes for one param: `{ param = … }`, `{ variable = … }` or a value.
#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub(super) enum ParamValueSchema {
    /// A param that takes the value of another param.
    Param(#[serde(deserialize_with = "reference")] ReferenceSchema),
    /// A param that takes the value of a variable.
    Variable(#[serde(deserialize_with = "variable")] VariableSchema),
    /// A value written in place.
    Literal(toml::Value),
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
/// A `{ param = ... }` reference.
pub(super) struct ReferenceSchema {
    /// The param referred to.
    param: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
/// A `{ variable = ... }` reference.
pub(super) struct VariableSchema {
    /// The variable referred to.
    variable: String,
}

/// Deserializes a `{ param = ... }` table, refusing any other key.
fn reference<'de, D>(deserializer: D) -> Result<ReferenceSchema, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let table = <toml::Table as serde::Deserialize>::deserialize(deserializer)?;
    table.try_into().map_err(serde::de::Error::custom)
}

/// Deserializes a `{ variable = ... }` table, refusing any other key.
fn variable<'de, D>(deserializer: D) -> Result<VariableSchema, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let table = <toml::Table as serde::Deserialize>::deserialize(deserializer)?;
    table.try_into().map_err(serde::de::Error::custom)
}

/// Refuses a key that is not kebab-case, `what` saying what the key names.
fn kebab_key(key: &str, what: &str) -> Result<(), ManifestError> {
    if kebab(key) {
        Ok(())
    } else {
        Err(shape(format!(
            "{what} `{key}` is not kebab-case: write `{}`; every key of a Tastemaker file is lowercase words joined by `-`",
            key.to_lowercase().replace('_', "-")
        )))
    }
}

/// Reads an entry's `params` table, refusing a param whose name is not kebab-case.
fn params(map: Option<BTreeMap<String, ParamValueSchema>>) -> Result<ParamValues, ManifestError> {
    let map = map.unwrap_or_default();
    for name in map.keys() {
        kebab_key(name, "param")?;
    }
    Ok(ParamValues(
        map.into_iter()
            .map(|(name, written)| {
                let written = match written {
                    ParamValueSchema::Param(reference) => {
                        ParamValue::Param(ParamName(reference.param.into()))
                    }
                    ParamValueSchema::Variable(found) => {
                        ParamValue::Variable(VariableName(found.variable.into()))
                    }
                    ParamValueSchema::Literal(literal) => ParamValue::Literal(value(literal)),
                };
                (ParamName(name.into()), written)
            })
            .collect(),
    ))
}

/// A count as written.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
pub(super) enum CountSchema {
    /// A count written as one number.
    Exactly(u32),
    /// A count that depends on a condition.
    When(CountWhenSchema),
    /// A count written as a range.
    Range(CountRangeSchema),
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
/// A `{ min, max }` count.
pub(super) struct CountRangeSchema {
    /// The least count; zero when absent.
    min: Option<u32>,
    /// The greatest count; unbounded when absent.
    max: Option<u32>,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
/// A `{ when, then, else }` count.
pub(super) struct CountWhenSchema {
    /// The condition that picks the count.
    when: ConditionReferenceSchema,
    /// The count when the condition holds.
    then: u32,
    #[serde(rename = "else")]
    /// The count when the condition does not hold.
    otherwise: u32,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
/// A `{ condition = ... }` reference.
pub(super) struct ConditionReferenceSchema {
    /// The condition referred to.
    condition: String,
}

impl From<CountSchema> for Count {
    fn from(schema: CountSchema) -> Self {
        match schema {
            CountSchema::Exactly(count) => Self::Exactly(Amount(count)),
            CountSchema::Range(range) => Self::Range {
                min: Amount(range.min.unwrap_or(0)),
                max: range
                    .max
                    .map_or(Bound::Unbounded, |max| Bound::AtMost(Amount(max))),
            },
            CountSchema::When(when) => Self::When {
                condition: ConditionName(when.when.condition.into()),
                then: Amount(when.then),
                otherwise: Amount(when.otherwise),
            },
        }
    }
}

/// An entry's `imports` as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImportsSchema {
    #[serde(default)]
    /// Whether imports are allowed or denied unless listed; allowed when absent.
    default: Option<String>,
    #[serde(default)]
    /// The import patterns the entry admits.
    allow: Vec<String>,
}

/// Reads an entry's optional `imports` table as the core's imports: a `default` other than `deny` is refused, never read as
/// allowing everything.
fn imports(schema: Option<ImportsSchema>) -> Result<Imports, ManifestError> {
    Ok(match schema {
        None => Imports::Unrestricted,
        Some(schema) => Imports::Restricted {
            default: match schema.default.as_deref() {
                None => ImportDefault::Allow,
                Some(DENY) => ImportDefault::Deny,
                Some(other) => {
                    return Err(shape(format!(
                        "`imports.default = \"{other}\"` is not a default: write `deny`, or leave it out to allow every import the list does not name"
                    )));
                }
            },
            allow: schema
                .allow
                .into_iter()
                .map(|item| ImportPattern(item.into()))
                .collect(),
        },
    })
}

/// Every key any entry may carry; which of them each list allows is checked after.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
// ignore: idiomatic.arch/no-deep-stored-types - the shape is the tables of the definition file a user writes (`tastemaker.toml`, `rule.toml`, `spec.toml`, `template.toml`), read table for table: a flatter shape reads a different file
pub(super) struct EntrySchema {
    /// The name the entry is known by in its list.
    alias: Option<String>,
    /// What the entry is for.
    description: Option<String>,
    /// The rule an entry of `rules` places.
    rule: Option<String>,
    /// The spec an entry of `specs` places.
    spec: Option<String>,
    /// The template an entry of `templates` places.
    template: Option<String>,
    /// The selectors of the paths the entry covers.
    include: Option<Vec<SelectorSchema>>,
    /// The selectors of the paths the entry leaves out.
    exclude: Option<Vec<SelectorSchema>>,
    /// The path a template entry writes its file at.
    file: Option<String>,
    /// The values the entry gives its definition's params.
    params: Option<BTreeMap<String, ParamValueSchema>>,
    /// How many times the entry applies, when it differs from what its paths say.
    count: Option<CountSchema>,
    /// The imports the entry restricts its files to.
    imports: Option<ImportsSchema>,
}

/// Reads an optional description as the core's, unsaid when absent.
fn described(text: Option<String>) -> Described {
    text.map_or(Described::Unsaid, |text| {
        Described::Said(Description(text.into()))
    })
}

/// What an entry's `rule`, `spec` or `template` names: `package/name` in a `tastemaker.toml`, whose lock pins the
/// version; `package/name@version` in a spec; or a bare `name`, a definition of the spec that writes it.
struct IdentitySchema {
    /// The package the definition belongs to, empty for a bare name.
    package: String,
    /// The version a spec pins, empty when none is written.
    version: String,
    /// Whether the definition is the writing spec's own or shared from a package.
    ownership: Ownership,
    /// The definition's name within its package.
    name: String,
}

/// Splits a written definition name into its package, version and name, refusing a malformed one.
fn identity(written: &str) -> Result<IdentitySchema, ManifestError> {
    let Some((package, rest)) = written.split_once('/') else {
        return Ok(IdentitySchema {
            package: String::new(),
            version: String::new(),
            ownership: Ownership::Owned(DefinitionName(Box::default())),
            name: written.to_owned(),
        });
    };
    let (name, version) = rest.split_once('@').unwrap_or((rest, ""));
    if package.is_empty()
        || name.is_empty()
        || name.contains('/')
        || (rest.contains('@') && version.is_empty())
    {
        return Err(shape(format!(
            "`{written}` is no definition: write `package/name`, as `idiomatic.arch/crate`"
        )));
    }
    Ok(IdentitySchema {
        package: package.to_owned(),
        version: version.to_owned(),
        ownership: Ownership::Shared,
        name: name.to_owned(),
    })
}

/// The header of an entry naming `written`, and the name of its definition.
fn header(
    schema: &mut EntrySchema,
    written: &str,
) -> Result<(EntryHeader, DefinitionName), ManifestError> {
    let IdentitySchema {
        package,
        version,
        ownership,
        name,
    } = identity(written)?;
    let header = EntryHeader {
        package: PackageName(package.into()),
        version: VersionText(version.into()),
        ownership,
        alias: schema.alias.take().map_or(Alias::Unnamed, |alias| {
            Alias::Named(DefinitionName(alias.into()))
        }),
        description: described(schema.description.take()),
        include: selectors(schema.include.take()),
        exclude: selectors(schema.exclude.take()),
        params: params(schema.params.take())?,
    };
    Ok((header, DefinitionName(name.into())))
}

/// Refuses an entry of `rules` or `specs` that writes no `include`: it would cover no file, where it once covered every
/// one, so the entry says which it covers.
fn included(
    include: Option<&Vec<SelectorSchema>>,
    kind: &str,
    name: &str,
) -> Result<(), ManifestError> {
    if include.is_some() {
        Ok(())
    } else {
        Err(shape(format!(
            "{kind} entry `{name}`: names no `include`; write `include = [\"**\"]` to cover every file, or the paths it covers"
        )))
    }
}

/// Reads an entry of `rules` as the core's rule entry.
fn rule_entry(mut schema: EntrySchema) -> Result<RuleEntry, ManifestError> {
    let rule = schema
        .rule
        .take()
        .ok_or_else(|| shape("a rule entry names no `rule`"))?;
    if schema.count.is_some() {
        return Err(shape(format!(
            "rule entry `{rule}`: `count` is refused on a rule entry"
        )));
    }
    included(schema.include.as_ref(), "rule", &rule)?;
    let (header, rule) = header(&mut schema, &rule)?;
    Ok(RuleEntry { header, rule })
}

/// Reads an entry of `specs` as the core's spec entry, `interpolated` being the params its file declares.
fn spec_entry(
    mut schema: EntrySchema,
    interpolated: &BTreeSet<String>,
) -> Result<SpecEntry, ManifestError> {
    let spec = schema
        .spec
        .take()
        .ok_or_else(|| shape("a spec entry names no `spec`"))?;
    included(schema.include.as_ref(), "spec", &spec)?;
    let patterns = schema.include.as_ref().map_or_else(Vec::new, |include| {
        include
            .iter()
            .map(|selector| match selector {
                SelectorSchema::Glob(pattern) => Some(pattern.as_str()),
                SelectorSchema::Regex { .. }
                | SelectorSchema::Part { .. }
                | SelectorSchema::PlacedBy { .. }
                | SelectorSchema::Variable { .. } => None,
            })
            .collect()
    });
    let count = counted(&spec, schema.count.take(), &patterns, interpolated)?;
    let imports = imports(schema.imports.take())?;
    let (header, spec) = header(&mut schema, &spec)?;
    Ok(SpecEntry {
        header,
        spec,
        count,
        imports,
    })
}

/// An entry's count: the one written when it differs from what its paths say, else what they say; writing what they
/// say is refused, so a count written is always one that matters.
fn counted(
    name: &str,
    written: Option<CountSchema>,
    patterns: &[Option<&str>],
    interpolated: &BTreeSet<String>,
) -> Result<Count, ManifestError> {
    let patterns = patterns
        .iter()
        .map(|pattern| pattern.map(|pattern| PathPattern(pattern.into())))
        .collect::<Vec<_>>();
    let derived = derived(
        &patterns.iter().map(Option::as_ref).collect::<Vec<_>>(),
        &interpolated
            .iter()
            .map(|param| ParamName(param.as_str().into()))
            .collect(),
    );
    match written.map(Count::from) {
        None => Ok(derived),
        Some(count) if count == derived => Err(shape(format!(
            "entry `{name}`: its `count` is what its path says already ({}): leave `count` out",
            match &derived {
                Count::Exactly(amount) => amount.0.to_string(),
                Count::Range { .. } | Count::When { .. } => "at least 1".to_owned(),
            }
        ))),
        Some(count) => Ok(count),
    }
}

/// Reads an entry of `templates` as the core's template entry, `interpolated` being the params its file declares.
fn template_entry(
    mut schema: EntrySchema,
    interpolated: &BTreeSet<String>,
) -> Result<TemplateEntry, ManifestError> {
    let template = schema
        .template
        .take()
        .ok_or_else(|| shape("a template entry names no `template`"))?;
    let file = schema
        .file
        .take()
        .ok_or_else(|| shape(format!("template entry `{template}` has no `file`")))?;
    let count = counted(
        &template,
        schema.count.take(),
        &[Some(file.as_str())],
        interpolated,
    )?;
    let imports = imports(schema.imports.take())?;
    let (header, template) = header(&mut schema, &template)?;
    Ok(TemplateEntry {
        header,
        template,
        file: PathPattern(file.into()),
        count,
        imports,
    })
}

/// A condition as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
// ignore: idiomatic.arch/no-deep-stored-types - the shape is the tables of the definition file a user writes (`tastemaker.toml`, `rule.toml`, `spec.toml`, `template.toml`), read table for table: a flatter shape reads a different file
pub(super) struct ConditionSchema {
    /// What the condition is for.
    description: Option<String>,
    /// The rule entry the condition checks, written as `rule`.
    rule: Option<EntrySchema>,
    /// The rule entry the condition checks, written as `entry`.
    entry: Option<EntrySchema>,
}

/// One item of `imports-allow`: a module pattern, or `{ variable = "…" }`, a variable whose list of patterns is put in place.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
pub(super) enum ImportAllowSchema {
    /// A module pattern.
    Pattern(String),
    /// A variable.
    Variable {
        /// Its name.
        variable: String,
    },
}

#[derive(Debug, Default)]
/// What a `tastemaker.toml` and a spec compose, gathered from their own shapes.
pub(super) struct CompositionSchema {
    /// The params the file declares: a `{name}` of one in a path is put in place, so it captures nothing.
    interpolated: BTreeSet<String>,
    /// Whether paths outside the entries are refused.
    closed: Option<bool>,
    /// The entries of `rules`.
    rules: Vec<EntrySchema>,
    /// The entries of `specs`.
    specs: Vec<EntrySchema>,
    /// The entries of `templates`.
    templates: Vec<EntrySchema>,
    /// The conditions that entries and counts refer to.
    conditions: BTreeMap<String, ConditionSchema>,
    /// The variables that selectors and params refer to.
    variables: BTreeMap<String, toml::Value>,
    /// The items of `imports-allow`.
    imports_allow: Vec<ImportAllowSchema>,
    /// The layers of the ordering, each a list of names.
    layers: Vec<Vec<String>>,
    /// The names that may not depend on one another.
    independent: Vec<String>,
    /// Whether the dependencies may not form a cycle.
    acyclic: bool,
}

impl TryFrom<CompositionSchema> for Composition {
    type Error = ManifestError;

    fn try_from(schema: CompositionSchema) -> Result<Self, Self::Error> {
        let mut conditions = BTreeMap::new();
        for name in schema.conditions.keys() {
            kebab_key(name, "condition")?;
        }
        for name in schema.variables.keys() {
            kebab_key(name, "variable")?;
        }
        let interpolated = &schema.interpolated;
        for (name, condition) in schema.conditions {
            let entry = condition
                .entry
                .or(condition.rule)
                .ok_or_else(|| shape(format!("condition `{name}` names no rule entry")))?;
            drop(conditions.insert(
                ConditionName(name.into()),
                Condition {
                    description: described(condition.description),
                    entry: rule_entry(entry)?,
                },
            ));
        }
        let count = |value: usize| Amount(u32::try_from(value).unwrap_or(u32::MAX));
        Ok(Self {
            closed: match schema.closed {
                Some(false) => ClosedChoice::Open,
                // A file writes only what differs from the default.
                Some(true) => {
                    return Err(shape(
                        "`closed = true` is the default: remove the line; `closed = false` opens the directory",
                    ));
                }
                None => ClosedChoice::Closed,
            },
            rules: schema
                .rules
                .into_iter()
                .map(rule_entry)
                .collect::<Result<_, _>>()?,
            specs: schema
                .specs
                .into_iter()
                .map(|entry| spec_entry(entry, interpolated))
                .collect::<Result<_, _>>()?,
            templates: schema
                .templates
                .into_iter()
                .map(|entry| template_entry(entry, interpolated))
                .collect::<Result<_, _>>()?,
            conditions,
            variables: Variables(
                schema
                    .variables
                    .into_iter()
                    .map(|(name, written)| (VariableName(name.into()), value(written)))
                    .collect(),
            ),
            imports_allow: schema
                .imports_allow
                .iter()
                .filter_map(|item| match item {
                    ImportAllowSchema::Pattern(text) => Some(ImportPattern(text.as_str().into())),
                    ImportAllowSchema::Variable { .. } => None,
                })
                .collect(),
            imports_allow_variables: schema
                .imports_allow
                .iter()
                .filter_map(|item| match item {
                    ImportAllowSchema::Variable { variable } => {
                        Some(VariableName(variable.as_str().into()))
                    }
                    ImportAllowSchema::Pattern(_) => None,
                })
                .collect(),
            ordering: Ordering {
                layers: count(schema.layers.len()),
                independent: count(schema.independent.len()),
                acyclic: if schema.acyclic {
                    Acyclicity::Written
                } else {
                    Acyclicity::Unwritten
                },
            },
        })
    }
}

/// A dependency as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DependencySchema {
    /// The name of the dependency's package.
    package: String,
    /// The directory of a dependency read from a path.
    path: Option<String>,
    /// The URL of a dependency read from git.
    git: Option<String>,
    /// The revision of a dependency read from git.
    rev: Option<String>,
}

impl TryFrom<DependencySchema> for Dependency {
    type Error = ManifestError;

    fn try_from(schema: DependencySchema) -> Result<Self, Self::Error> {
        let source = match (schema.path, schema.git, schema.rev) {
            (Some(path), None, None) => PackageSource::Path(SourcePath(path.into())),
            (None, Some(url), Some(revision)) => PackageSource::Git(GitSource {
                url: RepositoryUrl(url.into()),
                revision: Revision(revision.into()),
            }),
            (None, None, None) => PackageSource::Registry,
            (Some(_), Some(_), _) | (_, None, Some(_)) | (None, Some(_), None) => {
                return Err(shape(format!(
                    "dependency `{}` names one source: `path`, `git` with `rev`, or neither",
                    schema.package
                )));
            }
        };
        Ok(Self {
            package: PackageName(schema.package.into()),
            source,
        })
    }
}

/// The `workspace` table as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkspaceSchema {
    /// The workspace's name.
    name: String,
    #[serde(default, rename = "rules-exclude")]
    /// The selectors of the paths the workspace's rules leave out.
    rules_exclude: Vec<SelectorSchema>,
    #[serde(default)]
    /// The selectors of the directories that are members.
    members: Vec<SelectorSchema>,
    #[serde(default)]
    /// The packages the workspace depends on.
    dependencies: Vec<DependencySchema>,
}

/// A `tastemaker.toml` as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
// ignore: idiomatic.arch/no-deep-stored-types - the shape is the tables of the definition file a user writes (`tastemaker.toml`, `rule.toml`, `spec.toml`, `template.toml`), read table for table: a flatter shape reads a different file
pub(super) struct WorkspaceFileSchema {
    /// The version of Tastemaker the workspace needs, in a root file.
    tastemaker: Option<String>,
    /// The `workspace` table, present in a root file.
    workspace: Option<WorkspaceSchema>,
    /// Whether paths outside the entries are refused.
    closed: Option<bool>,
    #[serde(default)]
    /// The entries of `rules`.
    rules: Vec<EntrySchema>,
    #[serde(default)]
    /// The entries of `specs`.
    specs: Vec<EntrySchema>,
    #[serde(default)]
    /// The entries of `templates`.
    templates: Vec<EntrySchema>,
    #[serde(default)]
    /// The conditions that entries and counts refer to.
    conditions: BTreeMap<String, ConditionSchema>,
    #[serde(default)]
    /// The variables that selectors and params refer to.
    variables: BTreeMap<String, toml::Value>,
    #[serde(default, rename = "imports-allow")]
    /// The items of `imports-allow`.
    imports_allow: Vec<ImportAllowSchema>,
    #[serde(default)]
    /// The layers of the ordering, each a list of names.
    layers: Vec<Vec<String>>,
    #[serde(default)]
    /// The names that may not depend on one another.
    independent: Vec<String>,
    #[serde(default)]
    /// Whether the dependencies may not form a cycle.
    acyclic: bool,
}

impl TryFrom<WorkspaceFileSchema> for WorkspaceFile {
    type Error = ManifestError;

    fn try_from(schema: WorkspaceFileSchema) -> Result<Self, Self::Error> {
        match (&schema.tastemaker, &schema.workspace) {
            (Some(_), None) => {
                return Err(shape(
                    "`tastemaker` names the version of Tastemaker a workspace needs: write it in the root file, beside `workspace`",
                ));
            }
            (Some(needs), Some(_)) if needs != TASTEMAKER_VERSION => {
                return Err(ManifestError::Needs {
                    needs: needs.clone(),
                    installed: TASTEMAKER_VERSION.to_owned(),
                });
            }
            (Some(_) | None, _) => {}
        }
        let role = match schema.workspace {
            Some(workspace) => Role::Root(WorkspaceSection {
                name: WorkspaceName(workspace.name.into()),
                rules_exclude: selectors(Some(workspace.rules_exclude)),
                members: selectors(Some(workspace.members)),
                dependencies: workspace
                    .dependencies
                    .into_iter()
                    .map(Dependency::try_from)
                    .collect::<Result<_, _>>()?,
            }),
            None => Role::Local,
        };
        let composition = Composition::try_from(CompositionSchema {
            interpolated: BTreeSet::new(),
            closed: schema.closed,
            rules: schema.rules,
            specs: schema.specs,
            templates: schema.templates,
            conditions: schema.conditions,
            variables: schema.variables,
            imports_allow: schema.imports_allow,
            layers: schema.layers,
            independent: schema.independent,
            acyclic: schema.acyclic,
        })?;
        let headers = || {
            composition
                .rules
                .iter()
                .map(|entry| &entry.header)
                .chain(composition.specs.iter().map(|entry| &entry.header))
                .chain(composition.templates.iter().map(|entry| &entry.header))
                .chain(
                    composition
                        .conditions
                        .values()
                        .map(|condition| &condition.entry.header),
                )
        };
        if headers().any(|header| header.ownership != Ownership::Shared) {
            return Err(shape(
                "an entry of a tastemaker.toml names its package: write `package/name`, as `idiomatic.arch/crate`",
            ));
        }
        if let Some(pinned) = headers().find(|header| !header.version.0.is_empty()) {
            return Err(shape(format!(
                "an entry of `{}` writes its version: tastemaker.lock pins it; write `package/name`, and `tm update` moves the version",
                pinned.package.0
            )));
        }
        Ok(Self { role, composition })
    }
}

/// A param's declaration as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ParamDeclarationSchema {
    #[serde(rename = "type")]
    /// The param's type, written as a string.
    kind: toml::Value,
    /// What the param is for.
    description: Option<String>,
    /// Where the param takes its value from when it is not given.
    from: Option<String>,
}

/// Reads a `params` table of declarations as the core's, refusing a param whose name is not kebab-case.
fn declarations(
    map: BTreeMap<String, ParamDeclarationSchema>,
) -> Result<ParamDeclarations, ManifestError> {
    let mut declared = BTreeMap::new();
    for (name, schema) in map {
        kebab_key(&name, "param")?;
        let source = match schema.from.as_deref() {
            None => ParamSource::Given,
            Some(DIRECTORY_NAME) => ParamSource::DirectoryName,
            Some(WORKSPACE_NAME) => ParamSource::WorkspaceName,
            Some(other) => {
                return Err(shape(format!(
                    "param `{name}`: `from = \"{other}\"` is neither `directory-name` nor `workspace-name`"
                )));
            }
        };
        let kind = match schema.kind {
            toml::Value::String(text) => text,
            other @ (toml::Value::Integer(_)
            | toml::Value::Float(_)
            | toml::Value::Boolean(_)
            | toml::Value::Datetime(_)
            | toml::Value::Array(_)
            | toml::Value::Table(_)) => other.to_string(),
        };
        drop(declared.insert(
            ParamName(name.into()),
            ParamDeclaration {
                kind: ParamType(kind.into()),
                description: described(schema.description),
                source,
            },
        ));
    }
    Ok(ParamDeclarations(declared))
}

/// An expected finding as a entry writes it.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExpectedSchema {
    /// The rule the finding is of, the scenario's own when absent.
    rule: Option<String>,
    /// The path the finding is in.
    path: String,
    /// The line of the finding, the whole file when absent.
    line: Option<u32>,
}

impl From<ExpectedSchema> for ExpectedFinding {
    fn from(schema: ExpectedSchema) -> Self {
        Self {
            rule: schema.rule.map_or(ExpectedRuleChoice::Own, |rule| {
                ExpectedRuleChoice::Named(ExpectedRule(rule.into()))
            }),
            path: crate::domain::paths::types::RepoPath(schema.path.into()),
            line: schema.line.map_or(ExpectedLine::Whole, |line| {
                ExpectedLine::At(crate::domain::findings::types::Line(line))
            }),
        }
    }
}

/// A scenario as any definition writes it, an entry of `[[scenarios]]`; which keys each kind allows is checked after.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
// ignore: idiomatic.arch/no-deep-stored-types - the shape is the tables of the definition file a user writes (`tastemaker.toml`, `rule.toml`, `spec.toml`, `template.toml`), read table for table: a flatter shape reads a different file
pub(super) struct ScenarioSchema {
    /// The scenario's id.
    id: String,
    /// The sentence saying what the scenario shows.
    scenario: String,
    #[serde(default)]
    /// What the scenario starts from.
    given: GivenSchema,
    #[serde(default)]
    /// What the scenario expects.
    then: ThenSchema,
}

/// What a scenario starts from: the files of its project, and where and how the definition is placed in them. A
/// definition's own `[given]` is what every one of its scenarios starts from.
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
// ignore: idiomatic.arch/no-deep-stored-types - the shape is the tables of the definition file a user writes (`tastemaker.toml`, `rule.toml`, `spec.toml`, `template.toml`), read table for table: a flatter shape reads a different file
pub(super) struct GivenSchema {
    /// The path the definition is placed at.
    at: Option<String>,
    /// The file a template is placed in.
    file: Option<String>,
    /// How many times the definition applies.
    count: Option<CountSchema>,
    /// The selectors of the paths the definition covers.
    include: Option<Vec<SelectorSchema>>,
    /// The selectors of the paths the definition leaves out.
    exclude: Option<Vec<SelectorSchema>>,
    #[serde(default)]
    /// The values given to the definition's params.
    params: BTreeMap<String, toml::Value>,
    #[serde(default)]
    /// The files of the scenario's project, by path.
    files: BTreeMap<String, String>,
}

/// A scenario over its definition's shared `[given]`: each key the scenario's own `given` writes wins, and its files and
/// params are added to the shared ones, a path or a name it writes again taking its own text or value.
fn with_shared(shared: &GivenSchema, mut scenario: ScenarioSchema) -> ScenarioSchema {
    let given = &mut scenario.given;
    given.at = given.at.take().or_else(|| shared.at.clone());
    given.file = given.file.take().or_else(|| shared.file.clone());
    given.count = given.count.take().or_else(|| shared.count.clone());
    given.include = given.include.take().or_else(|| shared.include.clone());
    given.exclude = given.exclude.take().or_else(|| shared.exclude.clone());
    for (name, value) in &shared.params {
        let _: &mut toml::Value = given
            .params
            .entry(name.clone())
            .or_insert_with(|| value.clone());
    }
    for (path, text) in &shared.files {
        let _: &mut String = given
            .files
            .entry(path.clone())
            .or_insert_with(|| text.clone());
    }
    scenario
}

/// What a scenario expects: the findings, none when it passes.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ThenSchema {
    /// The rules whose findings are compared, all when absent.
    only: Option<Vec<String>>,
    /// The findings the scenario expects, none when absent.
    findings: Option<Vec<ExpectedSchema>>,
}

/// Reads a scenario's `params` table as the core's, refusing a param whose name is not kebab-case.
fn scenario_params(
    map: BTreeMap<String, toml::Value>,
) -> Result<BTreeMap<ParamName, Value>, ManifestError> {
    for name in map.keys() {
        kebab_key(name, "param")?;
    }
    Ok(map
        .into_iter()
        .map(|(name, written)| (ParamName(name.into()), value(written)))
        .collect())
}

/// Reads a scenario's files as the core's.
fn scenario_files(map: BTreeMap<String, String>) -> ScenarioFiles {
    ScenarioFiles(
        map.into_iter()
            .map(|(path, text)| {
                (
                    crate::domain::paths::types::RepoPath(path.into()),
                    ScenarioFileText(text.into()),
                )
            })
            .collect(),
    )
}

/// The rules a scenario compares, as it names them.
fn only(written: Option<Vec<String>>) -> Vec<ExpectedRule> {
    written
        .into_iter()
        .flatten()
        .map(|rule| ExpectedRule(rule.into()))
        .collect()
}

/// What every scenario says, whatever the definition: its id and sentence, its verdict, and what it expects.
struct SaidSchema {
    /// The scenario's id.
    id: ScenarioId,
    /// The sentence saying what the scenario shows.
    scenario: ScenarioLine,
    /// Whether the scenario passes or fails.
    verdict: Expectation,
    /// The rules whose findings are compared.
    only: Vec<ExpectedRule>,
    /// The findings the scenario expects.
    findings: Vec<ExpectedFinding>,
}

/// The ids no scenario may have: they say nothing about what the scenario covers.
const BARE_IDS: [&str; 4] = ["pass", "fail", "passes", "fails"];

/// Whether `id` is kebab-case: lowercase words and digits joined by single hyphens.
fn kebab(id: &str) -> bool {
    !id.is_empty()
        && id.split('-').all(|word| {
            !word.is_empty()
                && word
                    .chars()
                    .all(|letter| letter.is_ascii_lowercase() || letter.is_ascii_digit())
        })
}

/// A key a `kind`'s scenario writes that only another kind's scenarios have.
fn foreign_keys(
    kind: &str,
    id: &str,
    present: &[(&str, bool)],
    refused: &[&str],
) -> Result<(), ManifestError> {
    match present
        .iter()
        .find(|(key, written)| *written && refused.contains(key))
    {
        Some((key, _)) => Err(shape(format!(
            "scenario `{id}`: a {kind}'s scenario has no `{key}`: remove it"
        ))),
        None => Ok(()),
    }
}

/// A scenario read: what every kind says, and what is placed, checked against the kind's keys.
fn read_scenario(
    kind: &str,
    refused: &[&str],
    written: ScenarioSchema,
) -> Result<(SaidSchema, GivenSchema), ManifestError> {
    let ScenarioSchema {
        id,
        scenario,
        given,
        then,
    } = written;
    if !kebab(&id) || BARE_IDS.contains(&id.as_str()) {
        return Err(shape(format!(
            "scenario `{id}`: the `id` is not a descriptive kebab-case name: say what the scenario covers, as `todo-in-line-comment` does (not `pass` or `fail`)"
        )));
    }
    if scenario.trim().is_empty() {
        return Err(shape(format!(
            "scenario `{id}`: its `scenario` sentence is empty: write one sentence saying what the scenario shows, as a Gherkin Scenario line"
        )));
    }
    let line = scenario;
    foreign_keys(
        kind,
        &id,
        &[
            ("given.at", given.at.is_some()),
            ("given.file", given.file.is_some()),
            ("given.count", given.count.is_some()),
            ("given.include", given.include.is_some()),
            ("given.exclude", given.exclude.is_some()),
            ("given.params", !given.params.is_empty()),
            ("then.only", then.only.is_some()),
        ],
        refused,
    )?;
    let verdict = if then
        .findings
        .as_ref()
        .is_some_and(|found| !found.is_empty())
    {
        Expectation::Fails
    } else {
        Expectation::Passes
    };
    if kind != "rule"
        && then
            .findings
            .iter()
            .flatten()
            .any(|finding| finding.rule.is_none())
    {
        return Err(shape(format!(
            "scenario `{id}`: a {kind}'s finding names its `rule`: add `rule = \"...\"` to each finding"
        )));
    }
    let said = SaidSchema {
        id: ScenarioId(id.into()),
        scenario: ScenarioLine(line.into()),
        verdict,
        only: only(then.only),
        findings: then
            .findings
            .into_iter()
            .flatten()
            .map(ExpectedFinding::from)
            .collect(),
    };
    Ok((said, given))
}

/// A `rule.toml` as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
// ignore: idiomatic.arch/no-deep-stored-types - the shape is the tables of the definition file a user writes (`tastemaker.toml`, `rule.toml`, `spec.toml`, `template.toml`), read table for table: a flatter shape reads a different file
pub(super) struct RuleFileSchema {
    /// What the rule is for.
    description: String,
    /// Why the rule exists.
    why: String,
    #[serde(default)]
    /// The params the rule declares.
    params: BTreeMap<String, ParamDeclarationSchema>,
    #[serde(default)]
    /// The values the rule admits for its params, with the reason for each.
    exceptions: BTreeMap<String, ExceptionSchema>,
    /// The query that finds the rule's violations.
    query: String,
    /// How a finding of the rule is reported.
    diagnostic: DiagnosticSchema,
    #[serde(default)]
    /// The scenarios that show the rule at work.
    scenarios: Vec<ScenarioSchema>,
    #[serde(default)]
    /// What every scenario starts from, its own `given` adding to it.
    given: GivenSchema,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
/// The values a rule admits for a param, with the reason.
pub(super) struct ExceptionSchema {
    /// The values admitted.
    admits: Vec<toml::Value>,
    /// Why the values are admitted.
    why: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
/// How a finding of a rule is reported.
pub(super) struct DiagnosticSchema {
    /// The query column the finding is reported at.
    primary: String,
    /// The message of the finding.
    message: String,
    /// The help that says how to fix the finding.
    help: String,
}

impl TryFrom<RuleFileSchema> for RuleFile {
    type Error = ManifestError;

    fn try_from(mut schema: RuleFileSchema) -> Result<Self, Self::Error> {
        let shared = std::mem::take(&mut schema.given);
        Ok(Self {
            description: Description(schema.description.into()),
            why: Why(schema.why.into()),
            params: declarations(schema.params)?,
            exceptions: schema
                .exceptions
                .into_iter()
                .map(|(name, exception)| {
                    (
                        ParamName(name.into()),
                        Exception {
                            admits: exception.admits.into_iter().map(value).collect(),
                            why: Why(exception.why.into()),
                        },
                    )
                })
                .collect(),
            query: QueryText(schema.query.into()),
            diagnostic: Diagnostic {
                primary: ColumnName(schema.diagnostic.primary.into()),
                message: MessageTemplate(schema.diagnostic.message.into()),
                help: MessageTemplate(schema.diagnostic.help.into()),
            },
            scenarios: schema
                .scenarios
                .into_iter()
                .map(|written| {
                    let (said, given) = read_scenario(
                        "rule",
                        &["given.at", "given.file", "given.count"],
                        with_shared(&shared, written),
                    )?;
                    Ok(RuleScenario {
                        id: said.id,
                        scenario: said.scenario,
                        verdict: said.verdict,
                        include: selectors(given.include),
                        exclude: selectors(given.exclude),
                        params: scenario_params(given.params)?,
                        files: scenario_files(given.files),
                        only: said.only,
                        findings: said.findings,
                    })
                })
                .collect::<Result<_, ManifestError>>()?,
        })
    }
}

/// A `spec.toml` as written.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
// ignore: idiomatic.arch/no-deep-stored-types - the shape is the tables of the definition file a user writes (`tastemaker.toml`, `rule.toml`, `spec.toml`, `template.toml`), read table for table: a flatter shape reads a different file
pub(super) struct SpecFileSchema {
    /// What the spec is for.
    description: String,
    /// Why the spec exists.
    why: String,
    #[serde(default)]
    /// The params the spec declares.
    params: BTreeMap<String, ParamDeclarationSchema>,
    #[serde(default, rename = "rules-exclude")]
    /// The selectors of the paths the spec's rules leave out.
    rules_exclude: Vec<SelectorSchema>,
    #[serde(default)]
    /// The scenarios that show the spec at work.
    scenarios: Vec<ScenarioSchema>,
    #[serde(default)]
    /// What every scenario starts from, its own `given` adding to it.
    given: GivenSchema,
    /// Whether paths outside the entries are refused.
    closed: Option<bool>,
    #[serde(default)]
    /// The entries of `rules`.
    rules: Vec<EntrySchema>,
    #[serde(default)]
    /// The entries of `specs`.
    specs: Vec<EntrySchema>,
    #[serde(default)]
    /// The entries of `templates`.
    templates: Vec<EntrySchema>,
    #[serde(default)]
    /// The conditions that entries and counts refer to.
    conditions: BTreeMap<String, ConditionSchema>,
    #[serde(default)]
    /// The variables that selectors and params refer to.
    variables: BTreeMap<String, toml::Value>,
    #[serde(default)]
    /// The layers of the ordering, each a list of names.
    layers: Vec<Vec<String>>,
    #[serde(default)]
    /// The names that may not depend on one another.
    independent: Vec<String>,
    #[serde(default)]
    /// Whether the dependencies may not form a cycle.
    acyclic: bool,
}

/// A spec's composition: an entry naming another package's definition pins its version, `package/name@version`, since a
/// spec has no lock.
fn spec_composition(schema: CompositionSchema) -> Result<Composition, ManifestError> {
    let composition = Composition::try_from(schema)?;
    let unpinned = composition
        .rules
        .iter()
        .map(|entry| (&entry.header, &entry.rule))
        .chain(
            composition
                .specs
                .iter()
                .map(|entry| (&entry.header, &entry.spec)),
        )
        .chain(
            composition
                .templates
                .iter()
                .map(|entry| (&entry.header, &entry.template)),
        )
        .chain(
            composition
                .conditions
                .values()
                .map(|condition| (&condition.entry.header, &condition.entry.rule)),
        )
        .find(|(header, _)| header.ownership == Ownership::Shared && header.version.0.is_empty());
    if let Some((header, name)) = unpinned {
        return Err(shape(format!(
            "`{}/{}` names no version: a spec pins what it places, `{}/{}@<version>`",
            header.package.0, name.0, header.package.0, name.0
        )));
    }
    Ok(composition)
}

impl TryFrom<SpecFileSchema> for SpecFile {
    type Error = ManifestError;

    fn try_from(mut schema: SpecFileSchema) -> Result<Self, Self::Error> {
        let shared = std::mem::take(&mut schema.given);
        Ok(Self {
            description: Description(schema.description.into()),
            why: Why(schema.why.into()),
            rules_exclude: selectors(Some(schema.rules_exclude)),
            composition: spec_composition(CompositionSchema {
                interpolated: schema.params.keys().cloned().collect(),
                closed: schema.closed,
                rules: schema.rules,
                specs: schema.specs,
                templates: schema.templates,
                conditions: schema.conditions,
                variables: schema.variables,
                imports_allow: Vec::new(),
                layers: schema.layers,
                independent: schema.independent,
                acyclic: schema.acyclic,
            })?,
            params: declarations(schema.params)?,
            scenarios: schema
                .scenarios
                .into_iter()
                .map(|written| {
                    let written = with_shared(&shared, written);
                    let (said, given) = read_scenario(
                        "spec",
                        &[
                            "given.file",
                            "given.count",
                            "given.include",
                            "given.exclude",
                        ],
                        written,
                    )?;
                    Ok(SpecScenario {
                        id: said.id,
                        scenario: said.scenario,
                        verdict: said.verdict,
                        at: crate::domain::paths::types::RepoPath(
                            given.at.unwrap_or_default().into(),
                        ),
                        params: scenario_params(given.params)?,
                        files: scenario_files(given.files),
                        only: said.only,
                        findings: said.findings,
                    })
                })
                .collect::<Result<_, ManifestError>>()?,
        })
    }
}

/// How much of a template a placed file must hold: `match = "declarations"` or `match = "text"`, `contains` when it is not written.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum MatchSchema {
    #[default]
    /// A placed file holds the template's declarations and more.
    Contains,
    /// A placed file holds the template's declarations and nothing else.
    Declarations,
    /// A placed file holds the template's text and nothing else.
    Text,
}

/// A `template.toml` as written: its `content` is the text and `match` says how much of it a placed file holds.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
// ignore: idiomatic.arch/no-deep-stored-types - the shape is the tables of the definition file a user writes (`tastemaker.toml`, `rule.toml`, `spec.toml`, `template.toml`), read table for table: a flatter shape reads a different file
pub(super) struct TemplateFileSchema {
    /// What the template is for.
    description: String,
    /// Why the template exists.
    why: String,
    #[serde(default)]
    /// The params the template declares.
    params: BTreeMap<String, ParamDeclarationSchema>,
    #[serde(default, rename = "match")]
    /// How much of the template a placed file holds.
    matching: MatchSchema,
    /// The text a placed file is written from.
    content: String,
    #[serde(default)]
    /// The scenarios that show the template at work.
    scenarios: Vec<ScenarioSchema>,
    #[serde(default)]
    /// What every scenario starts from, its own `given` adding to it.
    given: GivenSchema,
}

impl TryFrom<TemplateFileSchema> for TemplateFile {
    type Error = ManifestError;

    fn try_from(mut schema: TemplateFileSchema) -> Result<Self, Self::Error> {
        let shared = std::mem::take(&mut schema.given);
        Ok(Self {
            description: Description(schema.description.into()),
            why: Why(schema.why.into()),
            params: declarations(schema.params)?,
            exact: match schema.matching {
                MatchSchema::Contains => Exactness::Contains,
                MatchSchema::Declarations => Exactness::Exact,
                MatchSchema::Text => Exactness::Verbatim,
            },
            content: TemplateText(schema.content.into()),
            scenarios: schema
                .scenarios
                .into_iter()
                .map(|written| {
                    let written = with_shared(&shared, written);
                    let (said, given) = read_scenario(
                        TEMPLATE_WORD,
                        &[
                            "given.at",
                            "given.include",
                            "given.exclude",
                        ],
                        written,
                    )?;
                    if given.file.is_none() && !given.files.contains_key(WORKSPACE_FILE) {
                        return Err(shape(format!(
                            "scenario `{}`: a template's scenario names the `given.file` it places the template at, or brings the `tastemaker.toml` that places it in `given.files`",
                            said.id.0
                        )));
                    }
                    Ok(TemplateScenario {
                        id: said.id,
                        scenario: said.scenario,
                        verdict: said.verdict,
                        file: given
                            .file
                            .map_or(TemplateFileChoice::BroughtWorkspace, |file| {
                                TemplateFileChoice::Named(PathPattern(file.into()))
                            }),
                        count: given.count.map_or(Count::Exactly(Amount(1)), Count::from),
                        params: scenario_params(given.params)?,
                        files: scenario_files(given.files),
                        only: said.only,
                        findings: said.findings,
                    })
                })
                .collect::<Result<_, ManifestError>>()?,
        })
    }
}

/// A `package.toml`: its `name`, what it is for, and its dependencies.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PackageFileSchema {
    /// The package's name.
    name: Option<String>,
    /// What the package is for.
    #[expect(
        dead_code,
        reason = "the package's description is for its readers; the workspace reads only its name and dependencies"
    )]
    description: Option<String>,
    #[serde(default)]
    /// The packages the package depends on.
    dependencies: Vec<DependencySchema>,
}

impl TryFrom<PackageFileSchema> for PackageManifest {
    type Error = ManifestError;

    fn try_from(schema: PackageFileSchema) -> Result<Self, Self::Error> {
        let name = schema
            .name
            .ok_or_else(|| shape("a package.toml names its package: add `name = \"...\"`"))?;
        Ok(Self {
            name: PackageName(name.into()),
            dependencies: schema
                .dependencies
                .into_iter()
                .map(Dependency::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}

/// A lock as written: the packages the workspace names, and each package resolved by name.
#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LockSchema {
    #[serde(default)]
    /// The lock's version, written first: 1 until the first release.
    pub version: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// The names of the packages the workspace depends on directly.
    pub dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    /// Every package the workspace resolved, by name.
    pub packages: BTreeMap<String, LockedPackageSchema>,
}

/// A package of a lock as written: where it was resolved from, the packages it names, and the version of each of its
/// definitions the workspace uses, by kind and then by name.
#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LockedPackageSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The package's directory, for a package read from a path.
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The repository, for a package read from git.
    pub git: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The revision, for a package read from git.
    pub rev: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// The names of the packages it depends on.
    pub dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    /// The version of each rule used, by name.
    pub rules: BTreeMap<String, LockedVersionsSchema>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    /// The version of each spec used, by name.
    pub specs: BTreeMap<String, LockedVersionsSchema>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    /// The version of each template used, by name.
    pub templates: BTreeMap<String, LockedVersionsSchema>,
}

/// The versions of one definition a lock pins: one, written as itself, or several, written as a list.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub(super) enum LockedVersionsSchema {
    /// The one version used.
    One(String),
    /// Every version used, when there is more than one.
    Several(Vec<String>),
}

impl TryFrom<LockSchema> for Lock {
    type Error = ManifestError;

    fn try_from(schema: LockSchema) -> Result<Self, Self::Error> {
        if schema.version != Some(LOCK_VERSION) {
            return Err(ManifestError::LockVersion {
                reason: format!(
                    "a lock begins `version = {LOCK_VERSION}`: run `tm check --fix` to write it again"
                ),
            });
        }
        let mut packages = Vec::new();
        for (name, package) in schema.packages {
            let source = match (package.path, package.git, package.rev) {
                (Some(path), None, None) => PackageSource::Path(SourcePath(path.into())),
                (None, Some(url), Some(revision)) => PackageSource::Git(GitSource {
                    url: RepositoryUrl(url.into()),
                    revision: Revision(revision.into()),
                }),
                (None, None, None) => PackageSource::Registry,
                (Some(_), Some(_), _) | (_, None, Some(_)) | (None, Some(_), None) => {
                    return Err(shape(format!(
                        "package `{name}` of the lock names one source: `path`, `git` with `rev`, or neither"
                    )));
                }
            };
            let mut releases = Vec::new();
            for (kind, used) in [
                (DefinitionKind::Rule, package.rules),
                (DefinitionKind::Spec, package.specs),
                (DefinitionKind::Template, package.templates),
            ] {
                for (definition, versions) in used {
                    let versions = match versions {
                        LockedVersionsSchema::One(version) => vec![version],
                        LockedVersionsSchema::Several(versions) => versions,
                    };
                    for version in versions {
                        releases.push(LockedRelease {
                            kind,
                            name: DefinitionName(definition.clone().into()),
                            version: VersionText(version.into()),
                        });
                    }
                }
            }
            releases.sort();
            packages.push(LockedPackage {
                name: PackageName(name.into()),
                source,
                dependencies: package
                    .dependencies
                    .into_iter()
                    .map(|dependency| PackageName(dependency.into()))
                    .collect(),
                releases,
            });
        }
        Ok(Self {
            dependencies: schema
                .dependencies
                .into_iter()
                .map(|name| PackageName(name.into()))
                .collect(),
            packages,
        })
    }
}

impl From<&Lock> for LockSchema {
    fn from(lock: &Lock) -> Self {
        Self {
            version: Some(LOCK_VERSION),
            dependencies: lock
                .dependencies
                .iter()
                .map(|name| name.0.to_string())
                .collect(),
            packages: lock
                .packages
                .iter()
                .map(|package| {
                    let (path, git, rev) = match &package.source {
                        PackageSource::Path(path) => (Some(path.0.to_string()), None, None),
                        PackageSource::Git(git) => (
                            None,
                            Some(git.url.0.to_string()),
                            Some(git.revision.0.to_string()),
                        ),
                        PackageSource::Registry => (None, None, None),
                    };
                    let used = |kind: DefinitionKind| {
                        let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
                        for release in package
                            .releases
                            .iter()
                            .filter(|release| release.kind == kind)
                        {
                            by_name
                                .entry(release.name.0.to_string())
                                .or_default()
                                .push(release.version.0.to_string());
                        }
                        by_name
                            .into_iter()
                            .map(|(name, mut versions)| {
                                let written = if versions.len() == 1 {
                                    LockedVersionsSchema::One(versions.remove(0))
                                } else {
                                    LockedVersionsSchema::Several(versions)
                                };
                                (name, written)
                            })
                            .collect()
                    };
                    (
                        package.name.0.to_string(),
                        LockedPackageSchema {
                            path,
                            git,
                            rev,
                            dependencies: package
                                .dependencies
                                .iter()
                                .map(|name| name.0.to_string())
                                .collect(),
                            rules: used(DefinitionKind::Rule),
                            specs: used(DefinitionKind::Spec),
                            templates: used(DefinitionKind::Template),
                        },
                    )
                })
                .collect(),
        }
    }
}

// The port's methods, in TOML: each text read through its schema, and each text written with its keys.

/// Reads `text` as `Schema`. Every file it reads denies unknown keys and needs each scenario's `id` and `scenario`, so a
/// text that reads is parsed once; only a refused one is parsed again, as a table, to name its `version` key or the
/// scenario key it lacks.
pub(super) fn read<Schema>(
    text: &DefinitionText,
    topic: Option<&'static str>,
    dropped: &'static [&'static str],
) -> Result<Schema, ManifestError>
where
    Schema: serde::de::DeserializeOwned,
{
    toml::from_str(&text.0).map_err(|error| {
        // Text that is no TOML at all is refused as serde says it, the error above.
        let Ok(table) = text.0.parse::<toml::Table>() else {
            return teach(&error, topic, dropped);
        };
        if table.contains_key(VERSION_KEY) {
            return shape(
                "`version` is no key of a Tastemaker file: only a lock has a version; remove the line",
            );
        }
        if topic.is_some()
            && let Err(refused) = named_scenario_keys(&table)
        {
            return refused;
        }
        teach(&error, topic, dropped)
    })
}

/// Reads `text` as `Schema`, a `version` key read as any other: a lock's.
fn decode<Schema>(
    text: &DefinitionText,
    topic: Option<&'static str>,
    dropped: &'static [&'static str],
) -> Result<Schema, ManifestError>
where
    Schema: serde::de::DeserializeOwned,
{
    toml::from_str(&text.0).map_err(|error| teach(&error, topic, dropped))
}

/// Refuses a scenario that leaves out a key every scenario needs, naming the scenario and the key to add: serde would say
/// only that a field is missing, and where.
fn named_scenario_keys(table: &toml::Table) -> Result<(), ManifestError> {
    let Some(toml::Value::Array(scenarios)) = table.get("scenarios") else {
        return Ok(());
    };
    for (at, entry) in scenarios.iter().enumerate() {
        let Some(entry) = entry.as_table() else {
            continue;
        };
        let id = entry
            .get("id")
            .and_then(toml::Value::as_str)
            .map_or_else(|| format!("#{}", at + 1), str::to_owned);
        if !entry.contains_key("id") {
            return Err(shape(format!(
                "scenario #{}: it has no `id`: add `id = \"...\"`, a kebab-case name that says what the scenario covers",
                at + 1
            )));
        }
        if !entry.contains_key(SCENARIO_KEY) {
            return Err(shape(format!(
                "scenario `{id}`: it has no `scenario` sentence: add `scenario = \"...\"`, one sentence saying what the scenario shows"
            )));
        }
    }
    Ok(())
}

/// A refusal that lists the keys a table takes, without the format-2 ones the reader still takes (`dropped`), pointing at
/// the topic that says them all.
fn teach(
    error: &toml::de::Error,
    topic: Option<&'static str>,
    dropped: &'static [&'static str],
) -> ManifestError {
    let message = error.message();
    if topic == Some(TEMPLATE_WORD)
        && ["`exact`", "`verbatim`", "`closed`"]
            .iter()
            .any(|key| message.starts_with(&format!("{UNKNOWN_FIELD} {key}")))
    {
        return shape(
            "a template says how much of it a placed file holds with `match`: `\"contains\"` (the default, not written), `\"declarations\"` or `\"text\"`; `exact`, `verbatim` and `closed` are no keys of a template",
        );
    }
    if message.starts_with(&format!("{UNKNOWN_FIELD} `package`"))
        || message.starts_with(&format!("{UNKNOWN_FIELD} `version`"))
    {
        return shape(format!(
            "{}: an entry names its definition in one string, `spec = \"idiomatic.arch/crate\"`; tastemaker.lock pins the version, and a spec writes `idiomatic.arch/crate@0.12.0`",
            error.to_string().trim_end()
        ));
    }
    if message.starts_with(&format!("{UNKNOWN_FIELD} `{LEVEL_KEY}`")) {
        return shape(format!(
            "{}: every finding is an error, so `{LEVEL_KEY}` is not accepted in a rule, a spec, a scenario or an entry; remove the key",
            error.to_string().trim_end()
        ));
    }
    if message.starts_with(&format!("{UNKNOWN_FIELD} `include`")) {
        return shape(format!(
            "{}: a workspace is the directory of its `tastemaker.toml`, every path relative to it; `include` is no key of `workspace`",
            error.to_string().trim_end()
        ));
    }
    let Some(topic) = topic.filter(|_| message.starts_with(UNKNOWN_FIELD)) else {
        return shape(error);
    };
    let offered = dropped.iter().fold(message.to_owned(), |said, key| {
        said.replace(&format!(", `{key}`"), "")
            .replace(&format!("`{key}`, "), "")
    });
    shape(format!(
        "{}; `tm explain {topic}` says each",
        error.to_string().replace(message, &offered).trim_end()
    ))
}

/// Writes a core value as a TOML value.
fn edit_value(value: &Value) -> toml_edit::Value {
    match value {
        Value::Text(text) => toml_edit::Value::from(&*text.0),
        Value::Whole(number) => toml_edit::Value::from(number.0),
        Value::Float(text) => text
            .0
            .parse::<f64>()
            .map_or_else(|_| toml_edit::Value::from(&*text.0), toml_edit::Value::from),
        Value::Flag(flag) => toml_edit::Value::from(*flag == FlagValue::Yes),
        Value::List(items) => {
            let mut array = toml_edit::Array::new();
            for item in items {
                array.push(edit_value(item));
            }
            toml_edit::Value::Array(array)
        }
        Value::Table(table) => {
            let mut inline = toml_edit::InlineTable::new();
            for (key, item) in table {
                drop(inline.insert(&*key.0, edit_value(item)));
            }
            toml_edit::Value::InlineTable(inline)
        }
    }
}

/// The key of the list that holds the entries of a kind.
fn kind_key(kind: DefinitionKind) -> &'static str {
    match kind {
        DefinitionKind::Rule => RULES,
        DefinitionKind::Spec => SPECS,
        DefinitionKind::Template => TEMPLATES,
    }
}

/// Writes a count as a TOML value: a number, or an inline table.
fn count_value(count: &Count) -> toml_edit::Value {
    match count {
        Count::Exactly(amount) => toml_edit::Value::from(i64::from(amount.0)),
        Count::Range { min, max } => {
            let mut table = toml_edit::InlineTable::new();
            drop(table.insert("min", toml_edit::Value::from(i64::from(min.0))));
            if let Bound::AtMost(max) = max {
                drop(table.insert("max", toml_edit::Value::from(i64::from(max.0))));
            }
            toml_edit::Value::InlineTable(table)
        }
        Count::When {
            condition,
            then,
            otherwise,
        } => {
            let mut when = toml_edit::InlineTable::new();
            drop(when.insert("condition", toml_edit::Value::from(&*condition.0)));
            let mut table = toml_edit::InlineTable::new();
            drop(table.insert("when", toml_edit::Value::InlineTable(when)));
            drop(table.insert("then", toml_edit::Value::from(i64::from(then.0))));
            drop(table.insert("else", toml_edit::Value::from(i64::from(otherwise.0))));
            toml_edit::Value::InlineTable(table)
        }
    }
}

/// Writes a selector as a TOML value: a glob as its string, any other as its inline table.
fn selector_value(selector: &Selector) -> toml_edit::Value {
    let keyed = |key: &str, value: &str| {
        let mut table = toml_edit::InlineTable::new();
        drop(table.insert(key, toml_edit::Value::from(value)));
        table
    };
    match selector {
        Selector::Glob(pattern) => toml_edit::Value::from(&*pattern.0),
        Selector::Regex(regex) => toml_edit::Value::InlineTable(keyed("regex", &regex.0)),
        Selector::Part(part) => {
            let mut table = keyed("part", &part.part.0);
            if part.recursion == Recursion::Recursive {
                drop(table.insert("recursive", toml_edit::Value::from(true)));
            }
            toml_edit::Value::InlineTable(table)
        }
        Selector::PlacedBy(placed) => {
            toml_edit::Value::InlineTable(keyed("placed-by", &placed.entry.0))
        }
        Selector::Variable(variable) => {
            toml_edit::Value::InlineTable(keyed("variable", &variable.0))
        }
    }
}

/// Writes a selector list as a TOML array.
fn selectors_value(selectors: &Selectors) -> toml_edit::Value {
    toml_edit::Value::Array(selectors.0.iter().map(selector_value).collect())
}

/// Adds a scenario's keys to `entry`: each list and table only when it holds something, the file only for a template, and the count only where it differs from what the file's path says.
fn scenario_keys(entry: &mut toml_edit::InlineTable, keys: &ScenarioKeys) {
    if !keys.include.0.is_empty() {
        drop(entry.insert("include", selectors_value(&keys.include)));
    }
    if !keys.exclude.0.is_empty() {
        drop(entry.insert("exclude", selectors_value(&keys.exclude)));
    }
    if let FileKey::Named(file) = &keys.file {
        drop(entry.insert("file", toml_edit::Value::from(&*file.0)));
    }
    if let CountKey::Written(count) = &keys.count {
        drop(entry.insert("count", count_value(count)));
    }
    if !keys.params.is_empty() {
        let mut params = toml_edit::InlineTable::new();
        for (name, value) in &keys.params {
            drop(params.insert(&*name.0, edit_value(value)));
        }
        drop(entry.insert("params", toml_edit::Value::InlineTable(params)));
    }
}

/// The word an entry names a definition of `kind` with.
fn kind_word(kind: DefinitionKind) -> &'static str {
    match kind {
        DefinitionKind::Rule => RULE_WORD,
        DefinitionKind::Spec => SPEC_WORD,
        DefinitionKind::Template => TEMPLATE_WORD,
    }
}

/// What `scenario_root` of the port answers, written in TOML: the scenario's workspace, open, its one dependency the
/// package at `path`, and its one entry.
pub(super) fn scenario_root(root: &ScenarioRoot<'_>) -> DefinitionText {
    let placed = root.placed;
    let mut dependency = toml_edit::InlineTable::new();
    drop(dependency.insert(PACKAGE_KEY, toml_edit::Value::from(&*placed.package.0)));
    drop(dependency.insert("path", toml_edit::Value::from(&*root.path.0)));
    let mut workspace = toml_edit::InlineTable::new();
    drop(workspace.insert("name", toml_edit::Value::from(SCENARIO_WORKSPACE)));
    drop(workspace.insert(
        "dependencies",
        toml_edit::Value::Array(std::iter::once(dependency).collect()),
    ));
    let mut entry = toml_edit::InlineTable::new();
    drop(entry.insert(
        kind_word(placed.kind),
        toml_edit::Value::from(format!("{}/{}", placed.package.0, placed.name.0)),
    ));
    drop(entry.insert("description", toml_edit::Value::from(PLACED_ALONE)));
    scenario_keys(&mut entry, root.keys);
    let mut document = toml_edit::DocumentMut::new();
    drop(document.insert("workspace", toml_edit::value(workspace)));
    // Open: it places only the definition under test, and the scenario's other files are its surroundings.
    drop(document.insert("closed", toml_edit::value(false)));
    drop(document.insert(
        kind_key(placed.kind),
        toml_edit::value(std::iter::once(entry).collect::<toml_edit::Array>()),
    ));
    DefinitionText(document.to_string().into())
}

/// What `scenario_wrapper` of the port answers, written in TOML: the wrapper spec, open, with one entry placing the owned
/// definition by its name.
pub(super) fn scenario_wrapper(wrapper: &Wrapper<'_>) -> DefinitionText {
    let (kind, name) = (kind_word(wrapper.kind), &*wrapper.name.0);
    let mut entry = toml_edit::InlineTable::new();
    drop(entry.insert(kind, toml_edit::Value::from(name)));
    drop(entry.insert(
        "description",
        toml_edit::Value::from(format!("The {kind} `{name}`, placed alone to be proven.")),
    ));
    scenario_keys(&mut entry, wrapper.keys);
    let mut document = toml_edit::DocumentMut::new();
    drop(document.insert(
        "description",
        toml_edit::value(format!("Places {kind} `{name}` alone.")),
    ));
    drop(document.insert(
        "why",
        toml_edit::value(format!(
            "Proves {kind} `{name}` of spec `{}` through its scenario, as its spec places it.",
            wrapper.spec.0
        )),
    ));
    drop(document.insert("closed", toml_edit::value(false)));
    drop(document.insert(
        kind_key(wrapper.kind),
        toml_edit::value(std::iter::once(entry).collect::<toml_edit::Array>()),
    ));
    DefinitionText(document.to_string().into())
}

/// What `workspace_file` of the port answers, read or written in TOML.
pub(super) fn workspace_file(text: &DefinitionText) -> Result<WorkspaceFile, ManifestError> {
    WorkspaceFile::try_from(read::<WorkspaceFileSchema>(text, None, &[])?)
}

/// What `spec` of the port answers, read or written in TOML.
pub(super) fn spec(text: &DefinitionText) -> Result<SpecFile, ManifestError> {
    SpecFile::try_from(read::<SpecFileSchema>(text, Some(SPEC_WORD), SPEC_DROPPED)?)
}

/// What `rule` of the port answers, read or written in TOML.
pub(super) fn rule(text: &DefinitionText) -> Result<RuleFile, ManifestError> {
    RuleFile::try_from(read::<RuleFileSchema>(text, Some(RULE_WORD), &[])?)
}

/// What `template` of the port answers, read or written in TOML.
pub(super) fn template(text: &DefinitionText) -> Result<TemplateFile, ManifestError> {
    TemplateFile::try_from(read::<TemplateFileSchema>(text, Some(TEMPLATE_WORD), &[])?)
}

/// What `package` of the port answers, read or written in TOML.
pub(super) fn package(text: &DefinitionText) -> Result<PackageManifest, ManifestError> {
    PackageManifest::try_from(read::<PackageFileSchema>(text, None, &[])?)
}

/// What `lock` of the port answers, read or written in TOML.
pub(super) fn lock(text: &DefinitionText) -> Result<Lock, ManifestError> {
    Lock::try_from(decode::<LockSchema>(text, None, &[])?)
}

/// What `lock_text` of the port answers: the lock's head, then the lock as its schema serializes it.
pub(super) fn lock_text(lock: &Lock) -> DefinitionText {
    let body = toml::to_string(&LockSchema::from(lock)).unwrap_or_default();
    DefinitionText(format!("{LOCK_HEAD}{body}").into())
}

/// What `add_entry` of the port answers, read or written in TOML.
pub(super) fn add_entry(text: &FileText, entry: &EntryDraft) -> Result<FileText, ManifestError> {
    let mut document = text.0.parse::<toml_edit::DocumentMut>().map_err(shape)?;
    let key = kind_key(entry.kind);
    let mut table = toml_edit::InlineTable::new();
    drop(table.insert(
        match entry.kind {
            DefinitionKind::Rule => RULE_WORD,
            DefinitionKind::Spec => SPEC_WORD,
            DefinitionKind::Template => TEMPLATE_WORD,
        },
        toml_edit::Value::from(format!("{}/{}", entry.package.0, entry.name.0)),
    ));
    match entry.kind {
        DefinitionKind::Template => {
            drop(table.insert("file", toml_edit::Value::from(&*entry.file.0)));
        }
        DefinitionKind::Rule | DefinitionKind::Spec => {
            let mut include = toml_edit::Array::new();
            for selector in &entry.include.0 {
                if let Selector::Glob(pattern) = selector {
                    include.push(&*pattern.0);
                }
            }
            drop(table.insert("include", toml_edit::Value::Array(include)));
        }
    }
    let patterns = match entry.kind {
        DefinitionKind::Template => vec![Some(&*entry.file.0)],
        DefinitionKind::Rule | DefinitionKind::Spec => entry
            .include
            .0
            .iter()
            .map(|selector| match selector {
                Selector::Glob(pattern) => Some(&*pattern.0),
                Selector::Regex(_)
                | Selector::Part(_)
                | Selector::PlacedBy(_)
                | Selector::Variable(_) => None,
            })
            .collect(),
    };
    let patterns = patterns
        .into_iter()
        .map(|pattern| pattern.map(|pattern| PathPattern(pattern.into())))
        .collect::<Vec<_>>();
    if entry.kind != DefinitionKind::Rule
        && entry.count
            != derived(
                &patterns.iter().map(Option::as_ref).collect::<Vec<_>>(),
                &BTreeSet::new(),
            )
    {
        drop(table.insert("count", count_value(&entry.count)));
    }
    if !entry.params.0.is_empty() {
        let mut params = toml_edit::InlineTable::new();
        for (name, value) in &entry.params.0 {
            if let ParamValue::Literal(value) = value {
                drop(params.insert(&*name.0, edit_value(value)));
            }
        }
        drop(table.insert("params", toml_edit::Value::InlineTable(params)));
    }
    drop(table.insert("description", toml_edit::Value::from(&*entry.description.0)));
    let item = document
        .entry(key)
        .or_insert(toml_edit::Item::Value(toml_edit::Value::Array(
            toml_edit::Array::new(),
        )));
    let array = item
        .as_array_mut()
        .ok_or_else(|| shape(format!("`{key}` is not a list of entries")))?;
    // An entry that names the same definition at the same place is already there: adding it again leaves the file as it is.
    let texts = |value: Option<&toml_edit::Value>| -> Option<Vec<String>> {
        match value? {
            toml_edit::Value::String(one) => Some(vec![one.value().clone()]),
            toml_edit::Value::Array(items) => Some(
                items
                    .iter()
                    .map(|each| each.as_str().unwrap_or_default().to_owned())
                    .collect(),
            ),
            toml_edit::Value::Integer(_)
            | toml_edit::Value::Float(_)
            | toml_edit::Value::Boolean(_)
            | toml_edit::Value::Datetime(_)
            | toml_edit::Value::InlineTable(_) => None,
        }
    };
    let same = |existing: &toml_edit::Value| {
        existing.as_inline_table().is_some_and(|existing| {
            [RULE_WORD, SPEC_WORD, TEMPLATE_WORD, "file", "include"]
                .into_iter()
                .all(|name| texts(existing.get(name)) == texts(table.get(name)))
        })
    };
    if array.iter().any(same) {
        return Ok(text.clone());
    }
    array.push(toml_edit::Value::InlineTable(table));
    Ok(FileText(document.to_string().into()))
}

/// What `rebased` of the port answers, read or written in TOML.
pub(super) fn rebased(
    text: &FileText,
    here: &crate::domain::paths::types::RepoPath,
) -> Result<FileText, ManifestError> {
    let here = &*here.0;
    let mut document = text.0.parse::<toml_edit::DocumentMut>().map_err(shape)?;
    let resolve = |path: &str| -> Option<String> {
        // Rooted, not only absolute: a held package's `/tastemaker-held/<n>` is no path from `here` on Windows either.
        if std::path::Path::new(path).has_root() {
            return None;
        }
        let mut joined = std::path::PathBuf::from(here);
        for part in std::path::Path::new(path).components() {
            match part {
                std::path::Component::ParentDir => {
                    // `..` above the root stays at the root.
                    let _at_root = joined.pop();
                }
                std::path::Component::CurDir => {}
                other @ (std::path::Component::Prefix(_)
                | std::path::Component::RootDir
                | std::path::Component::Normal(_)) => joined.push(other.as_os_str()),
            }
        }
        Some(joined.display().to_string())
    };
    let rebase = |dependencies: &mut toml_edit::Array| {
        for dependency in dependencies.iter_mut() {
            let Some(table) = dependency.as_inline_table_mut() else {
                continue;
            };
            let Some(absolute) = table
                .get("path")
                .and_then(toml_edit::Value::as_str)
                .and_then(resolve)
            else {
                continue;
            };
            drop(table.insert("path", absolute.into()));
        }
    };
    if let Some(dependencies) = document
        .get_mut("dependencies")
        .and_then(toml_edit::Item::as_array_mut)
    {
        rebase(dependencies);
    }
    if let Some(dependencies) = document
        .get_mut("workspace")
        .and_then(|workspace| workspace.get_mut("dependencies"))
        .and_then(toml_edit::Item::as_array_mut)
    {
        rebase(dependencies);
    }
    Ok(FileText(document.to_string().into()))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
