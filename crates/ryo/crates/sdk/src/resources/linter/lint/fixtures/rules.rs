//! The test data of `linter lint` that fixes rules.

use crate::domain::file::types::RelativePath;
use crate::domain::project::types::GivenProjectInventory;
use crate::domain::project::types::ProjectInventoryCompleteness;
use crate::domain::project::types::ProjectSearchScope;
use crate::resources::linter::RuleApplicability;
use crate::resources::linter::lint::types::Corpus;
use crate::resources::linter::lint::types::LintRequest;
use crate::resources::linter::lint::types::Rule;
use crate::resources::linter::lint::types::RuleId;
use crate::resources::query::RyoQlQuery;
use std::collections::BTreeMap;

/// A rule over `query` that observes nothing, binds no parameter and filters no file.
pub(crate) fn ordinary(query: &str) -> Rule {
    Rule {
        observations: Vec::new(),
        query: RyoQlQuery::try_from(query).expect("a query"),
        applicability: RuleApplicability::Always,
        parameters: BTreeMap::new(),
        include: Vec::new(),
        exclude: Vec::new(),
    }
}

/// A request over the one listed file `case.txt` of `root`, running `rules` with nothing hosted.
pub(crate) fn listed(root: &std::path::Path, rules: BTreeMap<RuleId, Rule>) -> LintRequest {
    LintRequest {
        project_search_scope: ProjectSearchScope::Undeclared,
        given_project_inventory: GivenProjectInventory {
            entries: Vec::new(),
            completeness: ProjectInventoryCompleteness::Partial,
        },
        source_language: crate::domain::language::types::SourceLanguage::Recognized,
        root: root.to_owned(),
        files: Vec::new(),
        beside: Vec::new(),
        hosted: Vec::new(),
        corpus: Corpus::Listed(vec![RelativePath("case.txt".into())]),
        rules,
    }
}

/// A rule id.
pub(crate) fn rule_id(text: &str) -> RuleId {
    RuleId::try_from(text).expect("a rule id")
}
