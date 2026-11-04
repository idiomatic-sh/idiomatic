//! pass: SDK callers construct policies and skip reasons through the public SDK API.

use ryo::linter::{Rule, RuleApplicability, RuleId, SkipReason, Skipped};
use ryo::query::RyoQlQuery;
use std::collections::BTreeMap;

fn main() {
    let query =
        RyoQlQuery::try_from("emit \"syntax-tree\" subject file key(id);").expect("a valid query");
    for applicability in [RuleApplicability::Always, RuleApplicability::AcceptedSyntax] {
        let rule = Rule {
            observations: Vec::new(),
            query: query.clone(),
            applicability,
            parameters: BTreeMap::new(),
            include: Vec::new(),
            exclude: Vec::new(),
        };
        assert_eq!(rule.applicability, applicability);
    }
    assert_eq!(RuleApplicability::default(), RuleApplicability::Always);
    let skipped = Skipped {
        rule: RuleId::try_from("rule").expect("a rule id"),
        path: ryo::linter::RelativePath("bad.py".into()),
        reason: SkipReason::SyntaxRejected,
    };
    assert_eq!(skipped.reason, SkipReason::SyntaxRejected);
}
