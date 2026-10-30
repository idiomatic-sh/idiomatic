//! The `built_ins` service: free functions named by verb that add the rules the lint call carries besides the placed
//! ones: each scope's conditions, the imports, the comments that may hold an ignore, and the syntax the contracts
//! compare, with no IO and no ports.

use std::collections::{BTreeMap, BTreeSet};

use super::types::Comparison;
use crate::domain::applications::constants::{
    COMMENTS, COMMENTS_QUERY, CONDITION_PREFIX, DECLARATIONS, EVERYTHING,
};
use crate::domain::applications::service::{loaded, parameters, rule_params};
use crate::domain::applications::types::{
    GlobText, LintRuleId, Nameable, Plan, PlannedRule, Purpose,
};
use crate::domain::contracts::constants::RENDERED;
use crate::domain::contracts::service::data_query;
use crate::domain::contracts::types::{Contract, Syntax};
use crate::domain::definitions::types::{Exactness, LoadedWorkspace, QueryText};
use crate::domain::entries::service::{identity, rule_release};
use crate::domain::entries::types::RuleEntry;
use crate::domain::globs::service::{compress, literal};
use crate::domain::globs::types::FileCounts;
use crate::domain::imports::types::Relation;
use crate::domain::packages::types::DefinitionKind;
use crate::domain::paths::service::{beneath, relative};
use crate::domain::paths::types::RepoPath;
use crate::domain::placement::errors::PlacementError;
use crate::domain::placement::types::Placement;
use crate::domain::scopes::service::composition_of;
use crate::domain::scopes::types::{Identity, Scope, ScopeIndex};
use crate::domain::selectors::service::{covering, resolve};
use crate::domain::selectors::types::{Coverage, PatternCompiler, SelectionContext};

/// The files a rule entry reaches in one scope: the governed files of the scope's directory, those in its nested
/// instances included, that its `include` covers and its `exclude` does not.
fn reach(
    workspace: &LoadedWorkspace,
    scope: &Scope,
    entry: &RuleEntry,
    known: &Nameable,
    compile: &dyn PatternCompiler,
) -> Result<Vec<RepoPath>, PlacementError> {
    let context = SelectionContext {
        base: &scope.dir,
        values: &scope.values,
        instances: &known.instances,
        placed: &known.placed,
        compile,
    };
    let entry_identity = || identity(&entry.header, DefinitionKind::Rule, &entry.rule).0;
    let include =
        resolve(&entry.header.include, &context).map_err(|source| PlacementError::Selector {
            entry: entry_identity(),
            source,
        })?;
    let exclude =
        resolve(&entry.header.exclude, &context).map_err(|source| PlacementError::Selector {
            entry: entry_identity(),
            source,
        })?;
    let candidates = beneath(&workspace.files, &scope.dir)
        .iter()
        .collect::<Vec<_>>();
    let included = covering(&include, &candidates);
    let excluded = covering(&exclude, &candidates);
    Ok(candidates
        .into_iter()
        .zip(included.into_iter().zip(excluded))
        .filter(|(_, (included, excluded))| {
            *included == Coverage::Covered && *excluded == Coverage::Uncovered
        })
        .map(|(file, _)| file.clone())
        .collect())
}

/// Each scope's conditions, as rules of the lint call: yes when the rule selects a row within its entry's `include`,
/// read over every file its entry reaches, those under a `rules-exclude` too.
///
/// # Errors
///
/// `Selector` when an entry's paths do not compile, `Unloaded` when an entry names a rule that was not loaded.
pub fn conditions(
    workspace: &LoadedWorkspace,
    placement: &Placement,
    plan: &mut Plan,
    known: &Nameable,
    compile: &dyn PatternCompiler,
    governed: &FileCounts,
) -> Result<(), PlacementError> {
    let local = workspace
        .local_packages
        .iter()
        .map(|package| package.directory.clone())
        .collect::<Vec<_>>();
    for (index, scope) in placement.scopes.iter().enumerate() {
        let composition = composition_of(workspace, &scope.composition);
        for (name, condition) in &composition.conditions {
            let entry = &condition.entry;
            let release = rule_release(entry);
            let rule = loaded(workspace, entry, &release)?;
            let Some(params) = rule_params(&mut plan.findings, scope, entry, &rule.params) else {
                continue;
            };
            let files = reach(workspace, scope, entry, known, compile)?
                .into_iter()
                .collect::<BTreeSet<_>>();
            if files.is_empty() {
                continue;
            }
            let id = LintRuleId(format!("{CONDITION_PREFIX}{}/{}", scope.dir.0, name.0).into());
            plan.rules.push(PlannedRule {
                id: id.clone(),
                query: rule.query.clone(),
                parameters: parameters(rule, params),
                include: compress(&files, governed, &local, &plan.base),
                exclude: Vec::new(),
            });
            drop(plan.purposes.insert(
                id,
                Purpose::Condition {
                    scope: ScopeIndex(index),
                    name: name.clone(),
                    identity: Identity(
                        identity(&entry.header, DefinitionKind::Rule, &entry.rule).into(),
                    ),
                },
            ));
        }
    }
    Ok(())
}

/// The built-in rules that read the imports, when an entry of a scope has an `imports`: one for each import relation,
/// over every file the check reads.
pub fn imports(plan: &mut Plan, workspace: &LoadedWorkspace, placement: &Placement) {
    plan.imports = crate::domain::imports::service::checks(workspace, placement);
    if plan.imports.is_empty() {
        return;
    }
    for relation in [
        Relation::Edges,
        Relation::Names,
        Relation::Packages,
        Relation::Owners,
    ] {
        let id = LintRuleId(
            format!("{}-{relation:?}", crate::domain::imports::constants::RULE)
                .to_lowercase()
                .into(),
        );
        plan.rules.push(PlannedRule {
            id: id.clone(),
            query: crate::domain::import_rows::service::query(relation),
            parameters: BTreeMap::new(),
            include: vec![GlobText(EVERYTHING.into())],
            exclude: Vec::new(),
        });
        drop(plan.purposes.insert(id, Purpose::Imports(relation)));
    }
}

/// The built-in rule that reads the comments of every file the check reports on, through the shared `comment` relation:
/// a comment may hold an ignore whatever the language that reads it.
pub fn comments(plan: &mut Plan) {
    let files = plan
        .files
        .iter()
        .filter(|file| !plan.excluded.contains(*file))
        .filter_map(|file| relative(file, &plan.base).map(literal))
        .collect::<Vec<_>>();
    if files.is_empty() {
        return;
    }
    let id = LintRuleId(COMMENTS.into());
    plan.rules.push(PlannedRule {
        id: id.clone(),
        query: QueryText(COMMENTS_QUERY.into()),
        parameters: BTreeMap::new(),
        include: files,
        exclude: Vec::new(),
    });
    drop(plan.purposes.insert(id, Purpose::Comments));
}

/// The shared relations every contract is compared over, and those an exact document is compared in order over too.
const SHARED: [Syntax; 8] = [
    Syntax::Defines,
    Syntax::Scopes,
    Syntax::Decorates,
    Syntax::Visibility,
    Syntax::Annotates,
    Syntax::Entries,
    Syntax::Mappings,
    Syntax::Elements,
];

/// The plan with `contracts` compared: one built-in rule for each shared relation the comparison reads, over every
/// placed file and every rendered content, which every other rule leaves out; the runs of prose and the literals only
/// over the exact contracts, which a document compares in order. No rule names a language: each file answers what its
/// language reads.
pub fn compared(plan: &mut Plan, contracts: Vec<Contract>) {
    if contracts.is_empty() {
        return;
    }
    let rendered = GlobText(format!("{RENDERED}/**").into());
    for rule in &mut plan.rules {
        rule.exclude.push(rendered.clone());
    }
    let globs = |listed: &mut dyn Iterator<Item = &Contract>| {
        listed
            .flat_map(|contract| [&contract.target, &contract.rendered])
            .filter_map(|path| relative(path, &plan.base).map(literal))
            .collect::<Vec<_>>()
    };
    let every = globs(&mut contracts.iter());
    let exact = globs(
        &mut contracts
            .iter()
            .filter(|contract| contract.exact != Exactness::Contains),
    );
    let mut built = SHARED
        .iter()
        .map(|syntax| Comparison {
            syntax: *syntax,
            query: data_query(*syntax),
            include: every.clone(),
        })
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        for syntax in [Syntax::Prose, Syntax::Literals] {
            built.push(Comparison {
                syntax,
                query: data_query(syntax),
                include: exact.clone(),
            });
        }
    }
    for comparison in built {
        let id = LintRuleId(
            format!("{DECLARATIONS}-{:?}", comparison.syntax)
                .to_lowercase()
                .into(),
        );
        plan.rules.push(PlannedRule {
            id: id.clone(),
            query: QueryText(comparison.query.0.into()),
            parameters: BTreeMap::new(),
            include: comparison.include,
            exclude: Vec::new(),
        });
        drop(
            plan.purposes
                .insert(id, Purpose::Declarations(comparison.syntax)),
        );
    }
    plan.contracts = contracts;
}
