//! Region sharing is limited to operands that mean the same thing in every rule.
use super::{service::rule_independent, types::RegionSharing};
use ryo_relations::domain::relations::types::Integer;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::plan_parts::types::{ColumnIndex, Operand, ParameterId, QueryLiteral},
};
type Input = Vec<Operand>;
type Value = RegionSharing;
type Failure = crate::domain::operating::errors::OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("literal_and_column_operands_are_the_same_in_every_rule"),
        word: Word::Succeeds,
        input: || {
            vec![
                Operand::Literal(QueryLiteral::Number(Integer(7))),
                Operand::Column(ColumnIndex(0)),
            ]
        },
        expect: Expect::Value(RegionSharing::Shareable),
    },
    Case {
        name: CaseName("no_operands_are_the_same_in_every_rule"),
        word: Word::Succeeds,
        input: Vec::new,
        expect: Expect::Value(RegionSharing::Shareable),
    },
    Case {
        name: CaseName("a_parameter_operand_binds_per_rule_and_is_never_shared"),
        word: Word::Succeeds,
        input: || {
            vec![
                Operand::Literal(QueryLiteral::Number(Integer(7))),
                Operand::Parameter(ParameterId(0)),
            ]
        },
        expect: Expect::Value(RegionSharing::RuleScoped),
    },
];
fn subject(values: Input) -> Result<Value, Failure> {
    Ok(rule_independent(&values))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
