//! The identity texts are deterministic and spell the declared tables, prefixes and disabled rules.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::linter::types::RuleContextInput;
use crate::outbound::linter::types::CanonicalText;
use ryo_benchmark_contracts::domain::context::types::{
    ExternalPrefix, PreparedRuleContext, RuleConfiguration, RuleContextTables, TableName,
};

type Input = ();
type Value = bool;
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a prepared configuration is spelled with its tables and prefixes"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];
fn subject((): Input) -> Result<Value, Failure> {
    let prepared = PreparedRuleContext {
        tables: RuleContextTables {
            selected: TableName("chosen".into()),
            disabled: TableName("disabled".into()),
            external: TableName("external".into()),
        },
        configuration: RuleConfiguration {
            external_prefixes: vec![ExternalPrefix("V101".into())],
            disabled_rules: Default::default(),
        },
    };
    let text = CanonicalText::from(RuleContextInput(&prepared)).0;
    assert!(text.contains("chosen") && text.contains("V101"));
    Ok(true)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
