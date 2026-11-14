//! The explicit project inputs a case declares, with the profile's search roots.
use super::super::canonical::schemas::{Canonical, invalid, table, text};
use super::super::case_wire::schemas::InputsWire;
use super::super::constants;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::BTreeMap;
use std::path::{Component, Path};

/// A relative path with no empty, current, parent or root component.
fn safe_relative(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

/// The explicit project inputs of a case with the profile's search roots, when either is declared.
pub(in super::super) fn project_inputs(
    case: &BTreeMap<String, Canonical>,
    profile: &BTreeMap<String, Canonical>,
) -> Result<Option<InputsWire>, Error> {
    let roots = case
        .get(constants::PROJECT_SEARCH_ROOTS)
        .or_else(|| profile.get(constants::PROJECT_SEARCH_ROOTS))
        .cloned();
    let default_inventory = table([
        (constants::COMPLETENESS, text(constants::PARTIAL)),
        (constants::ENTRIES, Canonical::List(Vec::new())),
    ]);
    let inventory = case
        .get(constants::GIVEN_PROJECT_INVENTORY)
        .cloned()
        .unwrap_or_else(|| default_inventory.clone());
    if let Some(roots) = &roots {
        let valid = roots.as_list().is_some_and(|roots| {
            roots.iter().all(|root| {
                root.as_text().is_some_and(|root| {
                    root.is_empty()
                        || (!Path::new(root).is_absolute()
                            && Path::new(root)
                                .components()
                                .all(|part| matches!(part, Component::Normal(_))))
                })
            })
        });
        if !valid {
            return Err(invalid(constants::SEARCH_ROOTS));
        }
    }
    let shape = inventory.as_table().filter(|fields| {
        fields.len() == 2
            && fields
                .get(constants::COMPLETENESS)
                .and_then(Canonical::as_text)
                .is_some_and(|completeness| {
                    [constants::PARTIAL, constants::COMPLETE_SCOPE].contains(&completeness)
                })
            && fields
                .get(constants::ENTRIES)
                .and_then(Canonical::as_list)
                .is_some()
    });
    let Some(fields) = shape else {
        return Err(invalid(constants::GIVEN_INVENTORY_SHAPE));
    };
    for entry in fields
        .get(constants::ENTRIES)
        .and_then(Canonical::as_list)
        .unwrap_or_default()
    {
        let valid = entry.as_table().is_some_and(|entry| {
            entry.len() == 2
                && entry
                    .get(constants::PATH)
                    .and_then(Canonical::as_text)
                    .is_some_and(safe_relative)
                && entry
                    .get(constants::STATE)
                    .and_then(Canonical::as_text)
                    .is_some_and(|state| constants::ENTRY_STATES.contains(&state))
        });
        if !valid {
            return Err(invalid(constants::GIVEN_ENTRY));
        }
    }
    let complete = fields
        .get(constants::COMPLETENESS)
        .and_then(Canonical::as_text)
        == Some(constants::COMPLETE_SCOPE);
    match roots {
        None if complete => Err(invalid(constants::COMPLETE_NEEDS_ROOTS)),
        None if inventory == default_inventory => Ok(None),
        roots => Ok(Some(InputsWire {
            given_project_inventory: inventory,
            project_search_roots: roots.unwrap_or(Canonical::Null),
        })),
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
