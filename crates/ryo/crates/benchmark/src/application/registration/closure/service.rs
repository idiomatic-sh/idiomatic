//! Which rules and modules a registered rule's stage holds: its cases' cohort and the modules its definitions import.
use crate::outbound::inventory::types::RegisteredModuleName;
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::{ArtifactPath, RuleQuery};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, InputDeclaration, QueryModuleName,
};
use ryo_benchmark_contracts::domain::workloads::types::Tool;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The logical module names a definition imports: `import <name> from "<module>";` at the start of a line.
///
/// This line scan is the registration contract, not a second RyoQL parser: it is the closure rule the authoring has
/// always registered with (`^\s*import\s+\w+\s+from\s+"([^"]+)"\s*;`), which the 413-rule oracle holds the registrar to,
/// and it decides only which module files are staged. Whether the staged definitions parse and link is checked by the
/// worker, through the query language, when it verifies the cases; `imports_for` is internal to the query language and
/// takes parsed declarations, which the SDK does not publish.
fn imports(definition: &RuleQuery) -> Vec<RegisteredModuleName> {
    definition
        .0
        .lines()
        .filter_map(|line| {
            let rest = line
                .trim_start()
                .strip_prefix(super::super::constants::IMPORT)?;
            let rest = rest.strip_prefix(char::is_whitespace)?.trim_start();
            let name_end = rest
                .char_indices()
                .find(|(_index, character)| {
                    !(character.is_alphanumeric()
                        || *character == super::super::constants::UNDERSCORE)
                })
                .map(|(index, _character)| index)?;
            let rest = rest
                .get(name_end..)
                .filter(|after| after.len() != rest.len())?;
            let rest = rest.strip_prefix(char::is_whitespace)?.trim_start();
            let rest = rest.strip_prefix(super::super::constants::FROM)?;
            let rest = rest.strip_prefix(char::is_whitespace)?.trim_start();
            let rest = rest.strip_prefix(super::super::constants::QUOTE)?;
            let (module, after) = rest.split_once(super::super::constants::QUOTE)?;
            after
                .trim_start()
                .starts_with(super::super::constants::END_OF_IMPORT)
                .then(|| RegisteredModuleName(module.to_owned()))
        })
        .collect()
}

/// The rules a rule's cases select, closed over the observations their producers declare, and their definitions.
pub(in super::super) fn cohort(
    suite: &dyn SuitePort,
    root: &Path,
    settings: &Tool,
    rule: &ryo::linter::RuleId,
) -> Result<BTreeSet<ryo::linter::RuleId>, Error> {
    let cases = suite.rule_cases(
        ArtifactPath(&root.join(crate::domain::suite::constants::RULES)),
        rule,
    )?;
    let names: BTreeSet<_> = cases.cases.iter().map(|case| &case.name.0).collect();
    if cases.cases.is_empty() || names.len() != cases.cases.len() {
        return Err(Error::Invalid(
            super::super::constants::CASES_EMPTY_OR_DUPLICATE.into(),
        ));
    }
    let mut cohort = BTreeSet::from([rule.clone()]);
    for case in &cases.cases {
        let selected = match &case.selected_rules {
            CohortSelection::Explicit(rules) => rules.clone(),
            CohortSelection::Frozen => vec![rule.clone()],
        };
        let declarations = match &case.rule_observations {
            InputDeclaration::Declared(declarations) => declarations.clone(),
            InputDeclaration::Undeclared => crate::domain::observations::service::project_profile(
                &settings.rule_observations,
                &selected,
                settings.rule_observation_selection,
            ),
        };
        cohort.extend(crate::domain::observations::service::closure(
            selected,
            |id| declarations.get(id).map(Vec::as_slice),
        ));
    }
    Ok(cohort)
}

/// The modules the staged definitions import, closed over the modules' own imports, by logical name.
pub(in super::super) fn module_closure(
    suite: &dyn SuitePort,
    root: &Path,
    base_modules: &BTreeMap<QueryModuleName, PathBuf>,
    settings: &Tool,
    definitions: &[RuleQuery],
) -> Result<BTreeMap<RegisteredModuleName, PathBuf>, Error> {
    let mut registry = base_modules.clone();
    registry.extend(settings.query_modules.clone());
    let mut closure = BTreeMap::new();
    let mut pending = definitions.to_vec();
    while let Some(definition) = pending.pop() {
        for name in imports(&definition) {
            if closure.contains_key(&name) {
                continue;
            }
            let relative = registry
                .get(&QueryModuleName(name.0.clone()))
                .ok_or_else(|| {
                    Error::Invalid(format!(
                        "{}: {}",
                        super::super::constants::UNDECLARED_MODULE,
                        name.0
                    ))
                })?;
            pending.push(RuleQuery(
                suite
                    .source_text(ArtifactPath(&root.join(relative)))?
                    .0
                    .into(),
            ));
            let _previous = closure.insert(name, relative.clone());
        }
    }
    let mut staged: BTreeMap<_, _> = settings
        .query_modules
        .iter()
        .map(|(name, path)| (RegisteredModuleName(name.0.clone()), path.clone()))
        .collect();
    staged.extend(closure);
    Ok(staged)
}
