//! The identity texts are deterministic and spell the declared tables, prefixes and disabled rules.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::linter::types::CanonicalText;
use ryo_benchmark_contracts::domain::context::types::{
    ExternalPrefix, PreparedRuleContext, RuleConfiguration, RuleContextTables, TableName,
};

/// What a row of the cases asks of the identity texts.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Spells a prepared rule context.
    Spelled,
    /// Binds the rule observations of a case whose profile declares none.
    NoDeclarations,
}
type Input = Scenario;
type Value = bool;
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a prepared configuration is spelled with its tables and prefixes"),
        word: Word::Succeeds,
        input: || Scenario::Spelled,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a profile without declarations binds no observations"),
        word: Word::Succeeds,
        input: || Scenario::NoDeclarations,
        expect: Expect::Value(true),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    if let Scenario::NoDeclarations = scenario {
        let case = crate::outbound::linter::fixtures::data::case();
        let profile = crate::outbound::linter::fixtures::data::tool();
        return Ok(super::effective_observations(&case, &profile).is_none());
    }
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
    let text = CanonicalText::from(&prepared).0;
    assert!(text.contains("chosen") && text.contains("V101"));
    Ok(true)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
