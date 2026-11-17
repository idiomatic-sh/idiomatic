//! The `entries` service: free functions named by verb over entries, counts and values, with no IO and no ports.

use super::types::{
    Alias, Amount, Bound, Branch, Composition, Count, CountText, EntryHeader, EntryIdentity,
    ParamName, ParamValue, ParamValues, RuleEntry, Setting, SpecEntry, TemplateEntry, Value,
};
use crate::domain::captures::service::pieces;
use crate::domain::captures::types::{CaptureBinding, PatternPiece, Template};
use crate::domain::packages::service::release_id;
use crate::domain::packages::types::{
    DefinitionKind, DefinitionName, Ownership, Release, VersionText,
};
use crate::domain::scalars::types::TextValue;
use crate::domain::selectors::types::PathPattern;

/// The release an entry names.
#[must_use]
pub fn release(header: &EntryHeader, kind: DefinitionKind, name: &DefinitionName) -> Release {
    Release {
        package: header.package.clone(),
        kind,
        name: name.clone(),
        version: header.version.clone(),
        ownership: header.ownership.clone(),
    }
}

/// The release a rule entry names.
#[must_use]
pub fn rule_release(entry: &RuleEntry) -> Release {
    release(&entry.header, DefinitionKind::Rule, &entry.rule)
}

/// The release a spec entry names.
#[must_use]
pub fn spec_release(entry: &SpecEntry) -> Release {
    release(&entry.header, DefinitionKind::Spec, &entry.spec)
}

/// The release a template entry names.
#[must_use]
pub fn template_release(entry: &TemplateEntry) -> Release {
    release(&entry.header, DefinitionKind::Template, &entry.template)
}

/// How output names an entry: `package/name`, `package/spec/name` for an owned definition, with `(alias)` when it has one.
#[must_use]
pub fn identity(
    header: &EntryHeader,
    kind: DefinitionKind,
    name: &DefinitionName,
) -> EntryIdentity {
    let id = release_id(&release(header, kind, name)).0;
    EntryIdentity(match &header.alias {
        Alias::Named(alias) => format!("{id}({})", alias.0),
        Alias::Unnamed => id,
    })
}

/// The name other entries refer to an entry by: its alias, or its definition's name.
#[must_use]
pub fn reference<'a>(header: &'a EntryHeader, name: &'a DefinitionName) -> &'a DefinitionName {
    match &header.alias {
        Alias::Named(alias) => alias,
        Alias::Unnamed => name,
    }
}

/// Every owned entry of `composition`, the spec `owner`'s, given the spec's package and version.
pub fn adopt(composition: &mut Composition, owner: &Release) {
    let own = |header: &mut EntryHeader| {
        if let Ownership::Owned(_) = header.ownership {
            header.package = owner.package.clone();
            header.version = owner.version.clone();
            header.ownership = Ownership::Owned(owner.name.clone());
        }
    };
    composition
        .rules
        .iter_mut()
        .for_each(|entry| own(&mut entry.header));
    composition
        .specs
        .iter_mut()
        .for_each(|entry| own(&mut entry.header));
    composition
        .templates
        .iter_mut()
        .for_each(|entry| own(&mut entry.header));
    composition
        .conditions
        .values_mut()
        .for_each(|condition| own(&mut condition.entry.header));
}

/// Gives each shared entry of `composition` written without a version the one `version_of` answers for the release it
/// names: what the lock pins for its definition, else the latest its package offers.
pub fn pin(composition: &mut Composition, version_of: &dyn Fn(&Release) -> VersionText) {
    let set = |header: &mut EntryHeader, kind: DefinitionKind, name: &DefinitionName| {
        if header.ownership == Ownership::Shared && header.version.0.is_empty() {
            header.version = version_of(&release(header, kind, name));
        }
    };
    for entry in &mut composition.rules {
        set(&mut entry.header, DefinitionKind::Rule, &entry.rule);
    }
    for condition in composition.conditions.values_mut() {
        set(
            &mut condition.entry.header,
            DefinitionKind::Rule,
            &condition.entry.rule,
        );
    }
    for entry in &mut composition.specs {
        set(&mut entry.header, DefinitionKind::Spec, &entry.spec);
    }
    for entry in &mut composition.templates {
        set(&mut entry.header, DefinitionKind::Template, &entry.template);
    }
}

/// Every release `composition` names: its rule entries and conditions, its spec entries and its template entries.
#[must_use]
pub fn releases(composition: &Composition) -> Vec<Release> {
    composition
        .rules
        .iter()
        .chain(
            composition
                .conditions
                .values()
                .map(|condition| &condition.entry),
        )
        .map(rule_release)
        .chain(composition.specs.iter().map(spec_release))
        .chain(composition.templates.iter().map(template_release))
        .collect()
}

/// The count an entry's paths say: each literal path is one, so as many as there are; any glob, capture or selector
/// that is no glob (`None`) is at least one. A `{name}` of `interpolated` is put in place, so it captures nothing.
#[must_use]
pub fn derived(
    patterns: &[Option<&PathPattern>],
    interpolated: &std::collections::BTreeSet<ParamName>,
) -> Count {
    let names_one = |pattern: &PathPattern| {
        pieces(Template(&pattern.0))
            .iter()
            .all(|piece| match piece {
                PatternPiece::Literal(_) => true,
                PatternPiece::Capture(name)
                | PatternPiece::Span(name)
                | PatternPiece::Directory(name) => interpolated.contains(&ParamName(name.0.into())),
                PatternPiece::Star | PatternPiece::Question => false,
            })
    };
    if !patterns.is_empty()
        && patterns
            .iter()
            .all(|pattern| pattern.is_some_and(names_one))
    {
        Count::Exactly(Amount(u32::try_from(patterns.len()).unwrap_or(u32::MAX)))
    } else {
        Count::Range {
            min: Amount(1),
            max: Bound::Unbounded,
        }
    }
}

/// The fewest a count accepts, when it does not depend on a condition.
#[must_use]
pub fn minimum(count: &Count) -> Option<Amount> {
    match count {
        Count::Exactly(amount) | Count::Range { min: amount, .. } => Some(*amount),
        Count::When { .. } => None,
    }
}

/// The count with its condition answered: `Exactly` the branch the answer picks.
#[must_use]
pub fn answered(count: &Count, branch: Branch) -> Count {
    match count {
        Count::When {
            then, otherwise, ..
        } => Count::Exactly(match branch {
            Branch::Then => *then,
            Branch::Otherwise => *otherwise,
        }),
        Count::Exactly(_) | Count::Range { .. } => count.clone(),
    }
}

/// A count as a message writes it.
#[must_use]
pub fn count_text(count: &Count) -> CountText {
    CountText(match count {
        Count::Exactly(amount) => format!("exactly {}", amount.0),
        Count::Range {
            min,
            max: Bound::Unbounded,
        } => format!("at least {}", min.0),
        Count::Range {
            min,
            max: Bound::AtMost(max),
        } if min == max => format!("exactly {}", min.0),
        Count::Range {
            min,
            max: Bound::AtMost(max),
        } => format!("between {} and {}", min.0, max.0),
        Count::When {
            condition,
            then,
            otherwise,
        } => format!(
            "{} when `{}` holds, otherwise {}",
            then.0, condition.0, otherwise.0
        ),
    })
}

/// `value` with each `{name}` in its text replaced by the value of that name, through the strings of lists and tables:
/// `package = "ryo-{lang}"` placed at `languages/{lang}` is `ryo-demo` at `languages/demo`. A `{…}` naming nothing stays.
#[must_use]
pub fn interpolate(value: &Value, values: &[CaptureBinding<'_>]) -> Value {
    match value {
        Value::Text(text) => Value::Text(TextValue(
            crate::domain::captures::service::substitute(Template(&text.0), values.iter().copied())
                .0
                .into(),
        )),
        Value::List(items) => {
            Value::List(items.iter().map(|item| interpolate(item, values)).collect())
        }
        Value::Table(table) => Value::Table(
            table
                .iter()
                .map(|(key, item)| (key.clone(), interpolate(item, values)))
                .collect(),
        ),
        Value::Whole(_) | Value::Float(_) | Value::Flag(_) => value.clone(),
    }
}

/// Each top-level rule, spec and template entry, by its reference and pinned release.
#[must_use]
pub fn named_releases(
    composition: &Composition,
    name: Option<&DefinitionName>,
) -> Vec<super::types::NamedRelease> {
    composition
        .rules
        .iter()
        .map(|entry| super::types::NamedRelease {
            entry: reference(&entry.header, &entry.rule).clone(),
            release: rule_release(entry),
        })
        .chain(
            composition
                .specs
                .iter()
                .map(|entry| super::types::NamedRelease {
                    entry: reference(&entry.header, &entry.spec).clone(),
                    release: spec_release(entry),
                }),
        )
        .chain(
            composition
                .templates
                .iter()
                .map(|entry| super::types::NamedRelease {
                    entry: reference(&entry.header, &entry.template).clone(),
                    release: template_release(entry),
                }),
        )
        .filter(|entry| name.is_none_or(|name| entry.entry == *name))
        .collect()
}

/// Settings as literal text parameters of an entry.
#[must_use]
pub fn literal_params(settings: &std::collections::BTreeMap<Setting, Setting>) -> ParamValues {
    ParamValues(
        settings
            .iter()
            .map(|(param, value)| {
                (
                    ParamName(param.0.clone()),
                    ParamValue::Literal(Value::Text(TextValue(value.0.clone()))),
                )
            })
            .collect(),
    )
}
