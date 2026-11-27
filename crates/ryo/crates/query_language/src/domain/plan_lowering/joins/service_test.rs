//! The `joins` service's cases: a `join` call is the select of its two sources under the aliases `l` and `r`, its key
//! pairs' equalities conjoined in order, its outputs each a column of the side it names; a call whose keys do not pair
//! or whose output names no side is refused.

use super::service::{JoinCall, joined};
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;
use crate::domain::plan_lowering::errors::PlanLoweringError;
use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::expressions::types::{
    ConstantEntry, ConstantEntryKey, PlanConstant, PlanExpression, SourceAlias,
};
use ryo_values::domain::finish_plans::types::{FinishStep, RowCondition, SelectedColumns};
use ryo_values::domain::plan_parts::types::LiteralText;

/// (left-on, right-on, output entries as (name, side, column)).
type Input = (&'static [&'static str], &'static [&'static str], &'static [(&'static str, &'static str, &'static str)]);
/// The select in one line (sources; condition; items), or the refused argument.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two_key_pairs_are_equalities_conjoined_in_order_and_each_output_reads_its_side"),
        word: Word::Succeeds,
        input: || (&["file", "code"], &["file", "code"], &[("directive", "left", "id"), ("prefix", "right", "prefix")]),
        expect: Expect::Value(Cow::Borrowed("3 as l, 7 as r; (l.file=r.file and l.code=r.code); directive=l.id prefix=r.prefix")),
    },
    Case {
        name: CaseName("a_join_without_keys_keeps_every_pair"),
        word: Word::Succeeds,
        input: || (&[], &[], &[("code", "left", "code")]),
        expect: Expect::Value(Cow::Borrowed("3 as l, 7 as r; every; code=l.code")),
    },
    Case {
        name: CaseName("keys_that_do_not_pair_are_refused"),
        word: Word::Succeeds,
        input: || (&["file", "code"], &["file"], &[("code", "left", "code")]),
        expect: Expect::Value(Cow::Borrowed("refused right-on")),
    },
    Case {
        name: CaseName("an_output_that_names_no_side_is_refused"),
        word: Word::Succeeds,
        input: || (&["code"], &["code"], &[("code", "middle", "code")]),
        expect: Expect::Value(Cow::Borrowed("refused output")),
    },
];

fn text(value: &str) -> PlanConstant {
    PlanConstant::Text(LiteralText(value.into()))
}

fn shown(expression: &PlanExpression) -> String {
    match expression {
        PlanExpression::Column(read) => match &read.alias {
            SourceAlias::Written(alias) => format!("{}.{}", alias.0, read.column.0),
            SourceAlias::Unwritten => read.column.0.to_string(),
        },
        PlanExpression::Compare(compare) => format!("{}={}", shown(&compare.left), shown(&compare.right)),
        PlanExpression::And(pair) => format!("({} and {})", shown(&pair.left), shown(&pair.right)),
        other => format!("{other:?}"),
    }
}

fn subject((left_on, right_on, output): Input) -> Result<Value, Failure> {
    let left_on = PlanConstant::List(left_on.iter().map(|c| text(c)).collect());
    let right_on = PlanConstant::List(right_on.iter().map(|c| text(c)).collect());
    let output = PlanConstant::Map(
        output
            .iter()
            .map(|(name, side, column)| ConstantEntry {
                key: ConstantEntryKey((*name).into()),
                value: PlanConstant::List(vec![text(side), text(column)]),
            })
            .collect(),
    );
    let call = JoinCall { left: StepNumber(3), right: StepNumber(7), left_on: &left_on, right_on: &right_on, output: &output };
    Ok(Cow::Owned(match joined(&call) {
        Ok(FinishStep::Select(select)) => {
            let sources: Vec<String> = select
                .sources
                .iter()
                .map(|source| match &source.alias {
                    SourceAlias::Written(alias) => format!("{} as {}", source.step.0, alias.0),
                    SourceAlias::Unwritten => source.step.0.to_string(),
                })
                .collect();
            let condition = match &select.condition {
                RowCondition::Every => "every".to_owned(),
                RowCondition::Tested(expression) => shown(expression),
            };
            let items = match &select.columns {
                SelectedColumns::All => "*".to_owned(),
                SelectedColumns::Items(items) => {
                    items.iter().map(|item| format!("{}={}", item.name.0, shown(&item.value))).collect::<Vec<_>>().join(" ")
                }
            };
            format!("{}; {condition}; {items}", sources.join(", "))
        }
        Ok(other) => format!("not a select: {other:?}"),
        Err(PlanLoweringError::OperatorArgument { argument, .. }) => format!("refused {argument}"),
        Err(other) => format!("refused: {other}"),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
