//! The `variables` service: free functions named by verb that put each `{ variable = … }` of a file in place, with no IO
//! and no ports. A variable is scoped to the file that declares it. In a selector list a list variable is spliced in;
//! anywhere else a variable is the whole value.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{Resolving, VariableProblem};
use crate::domain::entries::types::{
    Composition, EntryHeader, ImportPattern, ParamValue, ParamValues, Value,
};
use crate::domain::packages::types::DefinitionName;
use crate::domain::scalars::types::FlagValue;
use crate::domain::scalars::types::{TextValue, ValueKey};
use crate::domain::selectors::constants::{PART_KEY, PLACED_BY_KEY, RECURSIVE_KEY, REGEX_KEY};
use crate::domain::selectors::types::{
    PartSelector, PathPattern, PlacedBySelector, Recursion, RegexText, Selector, Selectors,
    VariableName,
};

/// Puts every variable of `composition` in place, in its entries' selectors and params and its conditions', and in
/// `extra`, the root file's `workspace` selector lists; answers what is wrong, undefined and unused variables included.
pub fn resolve(
    composition: &mut Composition,
    extra: &mut [&mut Selectors],
) -> Vec<VariableProblem> {
    let variables = std::mem::take(&mut composition.variables);
    let mut resolving = Resolving {
        variables: &variables,
        used: BTreeSet::new(),
        problems: BTreeSet::new(),
    };
    place(composition, extra, &mut resolving);
    for name in variables.0.keys() {
        if !resolving.used.contains(name) {
            let _: bool = resolving
                .problems
                .insert(VariableProblem::Unused(name.clone()));
        }
    }
    let problems = resolving.problems.into_iter().collect();
    composition.variables = variables;
    problems
}

/// Puts every variable of `composition` in place, as `resolve` does, for a caller that answers nothing about them.
pub fn apply(composition: &mut Composition, extra: &mut [&mut Selectors]) {
    let variables = std::mem::take(&mut composition.variables);
    let mut resolving = Resolving {
        variables: &variables,
        used: BTreeSet::new(),
        problems: BTreeSet::new(),
    };
    place(composition, extra, &mut resolving);
    composition.variables = variables;
}

/// The variables of `composition` and `extra` put in place, the ones read and the ones wrong noted in `resolving`.
fn place(
    composition: &mut Composition,
    extra: &mut [&mut Selectors],
    resolving: &mut Resolving<'_>,
) {
    for selectors in extra.iter_mut() {
        **selectors = splice(selectors, resolving);
    }
    for entry in &mut composition.rules {
        header(&mut entry.header, resolving);
    }
    for entry in &mut composition.specs {
        header(&mut entry.header, resolving);
    }
    for entry in &mut composition.templates {
        header(&mut entry.header, resolving);
    }
    for condition in composition.conditions.values_mut() {
        header(&mut condition.entry.header, resolving);
    }
    for name in std::mem::take(&mut composition.imports_allow_variables) {
        if let Some(Value::List(items)) = lookup(&name, resolving) {
            for item in items {
                match item {
                    Value::Text(TextValue(text)) => {
                        composition.imports_allow.push(ImportPattern(text));
                    }
                    Value::Whole(_)
                    | Value::Float(_)
                    | Value::Flag(_)
                    | Value::List(_)
                    | Value::Table(_) => {
                        let _: bool = resolving
                            .problems
                            .insert(VariableProblem::NotSelectors(name.clone()));
                    }
                }
            }
        }
    }
}

/// An entry header with each variable of its `include`, `exclude` and params put in place.
fn header(header: &mut EntryHeader, resolving: &mut Resolving<'_>) {
    header.include = splice(&header.include, resolving);
    header.exclude = splice(&header.exclude, resolving);
    header.params = params(&header.params, resolving);
}

/// `values` with each variable put in place.
fn params(values: &ParamValues, resolving: &mut Resolving<'_>) -> ParamValues {
    ParamValues(
        values
            .0
            .iter()
            .map(|(name, value)| {
                let value = match value {
                    ParamValue::Variable(variable) => match lookup(variable, resolving) {
                        Some(found) => ParamValue::Literal(found),
                        None => value.clone(),
                    },
                    ParamValue::Literal(_) | ParamValue::Param(_) => value.clone(),
                };
                (name.clone(), value)
            })
            .collect(),
    )
}

/// The value of the variable `name`, noted as used; an undefined one is noted as a problem.
fn lookup(name: &VariableName, resolving: &mut Resolving<'_>) -> Option<Value> {
    if let Some(value) = resolving.variables.0.get(name) {
        let _: bool = resolving.used.insert(name.clone());
        Some(value.clone())
    } else {
        let _: bool = resolving
            .problems
            .insert(VariableProblem::Undefined(name.clone()));
        None
    }
}

/// `selectors` with each variable's list spliced in.
fn splice(selectors: &Selectors, resolving: &mut Resolving<'_>) -> Selectors {
    let mut spliced = Vec::with_capacity(selectors.0.len());
    for selector in &selectors.0 {
        let Selector::Variable(name) = selector else {
            spliced.push(selector.clone());
            continue;
        };
        let Some(value) = lookup(name, resolving) else {
            continue;
        };
        let items = match value {
            Value::List(items) => items,
            single @ (Value::Text(_)
            | Value::Whole(_)
            | Value::Float(_)
            | Value::Flag(_)
            | Value::Table(_)) => vec![single],
        };
        for item in items {
            match selector_of(&item) {
                Some(found) => spliced.push(found),
                None => {
                    let _: bool = resolving
                        .problems
                        .insert(VariableProblem::NotSelectors(name.clone()));
                }
            }
        }
    }
    Selectors(spliced)
}

/// A value read as a selector: text is a glob, a table one of the other kinds.
#[must_use]
pub fn selector_of(value: &Value) -> Option<Selector> {
    match value {
        Value::Text(TextValue(text)) => Some(Selector::Glob(PathPattern(text.clone()))),
        Value::Table(table) => table_selector(table),
        Value::Whole(_) | Value::Float(_) | Value::Flag(_) | Value::List(_) => None,
    }
}

/// The text under `key` in a table; none when the key is absent or holds another kind of value.
fn text_of<'a>(table: &'a BTreeMap<ValueKey, Value>, key: &ValueKey) -> Option<&'a TextValue> {
    match table.get(key)? {
        Value::Text(text) => Some(text),
        Value::Whole(_) | Value::Float(_) | Value::Flag(_) | Value::List(_) | Value::Table(_) => {
            None
        }
    }
}

/// A table read as a selector of the kind its keys name; none when the keys name no kind or more than one.
fn table_selector(table: &BTreeMap<ValueKey, Value>) -> Option<Selector> {
    if let Some(expression) = text_of(table, &ValueKey(REGEX_KEY.into())) {
        return (table.len() == 1).then(|| Selector::Regex(RegexText(expression.0.clone())));
    }
    if let Some(entry) = text_of(table, &ValueKey(PLACED_BY_KEY.into())) {
        return (table.len() == 1).then(|| {
            Selector::PlacedBy(PlacedBySelector {
                entry: DefinitionName(entry.0.clone()),
            })
        });
    }
    let part = text_of(table, &ValueKey(PART_KEY.into()))?;
    let recursion = match table.get(&ValueKey(RECURSIVE_KEY.into())) {
        None | Some(Value::Flag(FlagValue::No)) => Recursion::Direct,
        Some(Value::Flag(FlagValue::Yes)) => Recursion::Recursive,
        Some(_) => return None,
    };
    let keys = 1 + usize::from(table.contains_key(&ValueKey(RECURSIVE_KEY.into())));
    (table.len() == keys).then(|| {
        Selector::Part(PartSelector {
            part: DefinitionName(part.0.clone()),
            recursion,
        })
    })
}
