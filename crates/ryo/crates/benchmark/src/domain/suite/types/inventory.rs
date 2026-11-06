//! Complete rule inventories never turn missing or unrun work into success.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::BTreeSet;

/// One independently identified rule's verification outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleResult {
    /// Stable identifier supplied by the pinned tool inventory.
    pub rule: String,
    /// True only after executed evidence satisfies the registered cases.
    pub passed: bool,
    /// Why this rule failed, including missing registration and not run.
    pub reason: Option<String>,
    /// Complete case pairs.
    pub cases: usize,
    /// Case pairs with identical unprojected diagnostic/skip reports.
    pub raw_exact: usize,
    /// Exact native case observation digest, absent until executed proof exists.
    pub case_results_sha256: Option<String>,
    /// Explicitly evidenced reference differences.
    pub documented: usize,
    /// Passing cases whose declared diagnostic contract cannot establish raw report equality.
    pub contract_only: usize,
}

/// Every pinned rule, in its declared order, from initial FAIL to final result.
#[derive(Clone, Debug)]
pub struct Inventory {
    /// Exactly one row for every declared rule.
    pub rules: Vec<RuleResult>,
}
impl Inventory {
    /// Initialize all rules as FAIL, rejecting empty or duplicate inventories.
    ///
    /// # Errors
    /// Refuses invalid, duplicate or unsafe rule identities.
    pub fn new(rules: &[String]) -> Result<Self, Error> {
        let unique = rules.iter().collect::<BTreeSet<_>>();
        if rules.is_empty()
            || unique.len() != rules.len()
            || rules.iter().any(|r| r.trim().is_empty())
        {
            return Err(Error::Invalid(
                crate::domain::suite::constants::INVENTORY_MUST_CONTAIN_UNIQUE_NONEMPTY_RULE.into(),
            ));
        }
        for rule in rules {
            ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(rule))?;
        }
        Ok(Self {
            rules: rules
                .iter()
                .map(|rule| RuleResult {
                    rule: rule.clone(),
                    passed: false,
                    reason: Some(crate::domain::suite::constants::NOT_RUN.into()),
                    cases: 0,
                    raw_exact: 0,
                    documented: 0,
                    contract_only: 0,
                    case_results_sha256: None,
                })
                .collect(),
        })
    }
    /// Strict success requires every pinned rule to pass.
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.rules.iter().all(|row| row.passed)
    }
}
