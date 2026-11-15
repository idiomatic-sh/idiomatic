//! Complete rule inventories never turn missing or unrun work into success.
use crate::domain::suite::constants;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RuleLabel;
use ryo_benchmark_contracts::domain::workloads::types::capability_name::value::CapabilityName;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::collections::BTreeSet;

/// Why one rule failed, including missing registration and not run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailureReason(
    /// The words of the failure.
    pub String,
);

/// The executed evidence of one passing rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassedRule {
    /// Complete case pairs.
    pub cases: CaseCount,
    /// Case pairs with identical unprojected diagnostic/skip reports.
    pub raw_exact: CaseCount,
    /// Explicitly evidenced reference differences.
    pub documented: CaseCount,
    /// Passing cases whose declared diagnostic contract cannot establish raw report equality.
    pub contract_only: CaseCount,
    /// Exact native case observation digest.
    pub case_results_sha256: ArtifactDigest,
}

/// What executed evidence says of one rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleEvidence {
    /// The rule has not passed.
    Failed(FailureReason),
    /// Executed evidence satisfies the registered cases.
    Passed(PassedRule),
    /// The profile defers the default under this capability; nothing ran.
    Deferred(CapabilityName),
}

/// Whether no pinned rule failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryVerdict {
    /// Every pinned rule passed or is deferred.
    Passed,
    /// At least one pinned rule failed.
    Failed,
}

/// One independently identified rule's verification outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleResult {
    rule: RuleLabel,
    outcome: RuleEvidence,
}

impl RuleResult {
    /// Stable identifier supplied by the pinned tool inventory.
    #[must_use]
    pub fn rule(&self) -> &RuleLabel {
        &self.rule
    }

    /// What executed evidence says of this rule.
    #[must_use]
    pub fn outcome(&self) -> &RuleEvidence {
        &self.outcome
    }
}

/// Every pinned rule, in its declared order, from initial FAIL to final result.
#[derive(Clone, Debug)]
pub struct Inventory {
    rules: Vec<RuleResult>,
}

impl Inventory {
    /// Initialize all rules as FAIL, rejecting empty or duplicate inventories.
    ///
    /// # Errors
    /// Refuses invalid, duplicate or unsafe rule identities.
    pub fn new(rules: &[RuleLabel]) -> Result<Self, Error> {
        let unique = rules.iter().collect::<BTreeSet<_>>();
        if rules.is_empty()
            || unique.len() != rules.len()
            || rules.iter().any(|rule| rule.0.trim().is_empty())
        {
            return Err(Error::Invalid(
                constants::INVENTORY_MUST_CONTAIN_UNIQUE_NONEMPTY_RULE.into(),
            ));
        }
        for rule in rules {
            RelativeArtifactPath::try_from(std::path::Path::new(&rule.0))?;
        }
        Ok(Self {
            rules: rules
                .iter()
                .map(|rule| RuleResult {
                    rule: rule.clone(),
                    outcome: RuleEvidence::Failed(FailureReason(constants::NOT_RUN.into())),
                })
                .collect(),
        })
    }

    /// Every pinned rule, in its declared order.
    #[must_use]
    pub fn rules(&self) -> &[RuleResult] {
        &self.rules
    }

    /// Record the failure of one pinned rule; an unpinned rule changes nothing.
    pub fn fail(&mut self, rule: &RuleLabel, reason: FailureReason) {
        for row in self.rules.iter_mut().filter(|row| row.rule == *rule) {
            row.outcome = RuleEvidence::Failed(reason.clone());
        }
    }

    /// Record the same failure for every pinned rule.
    pub fn fail_all(&mut self, reason: &FailureReason) {
        for row in &mut self.rules {
            row.outcome = RuleEvidence::Failed(reason.clone());
        }
    }

    /// Record the executed evidence of one pinned rule; an unpinned rule changes nothing.
    pub fn pass(&mut self, rule: &RuleLabel, evidence: PassedRule) {
        for row in self.rules.iter_mut().filter(|row| row.rule == *rule) {
            row.outcome = RuleEvidence::Passed(evidence.clone());
        }
    }

    /// Record the deferral of one pinned rule; an unpinned rule changes nothing.
    pub fn defer(&mut self, rule: &RuleLabel, capability: &CapabilityName) {
        for row in self.rules.iter_mut().filter(|row| row.rule == *rule) {
            row.outcome = RuleEvidence::Deferred(capability.clone());
        }
    }

    /// Success requires that no pinned rule failed: each passed or is deferred.
    #[must_use]
    pub fn verdict(&self) -> InventoryVerdict {
        if !self
            .rules
            .iter()
            .any(|row| matches!(row.outcome, RuleEvidence::Failed(_)))
        {
            InventoryVerdict::Passed
        } else {
            InventoryVerdict::Failed
        }
    }
}
