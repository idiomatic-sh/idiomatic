//! A plan expression as a JSON array headed by its kind (`["col", name, alias]`, `["cmp", "=", a, b]`, ..), and a
//! constant as a JSON value.

use super::super::ordered::schemas::OrderedMap;
use crate::domain::finish_plans::expressions::types::{
    ColumnRead, Comparator, ConstantEntry, ConstantEntryKey, ExpressionPair, FractionNumber,
    FunctionCall, FunctionName, PlanConstant, PlanExpression, PlanNumber, RowComparison,
    SourceAlias,
};
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::plan_parts::types::{LiteralText, WholeNumber};
use ryo_values::domain::query_tokens::types::AliasName;
use serde::{Deserialize, Serialize};

/// `"col"`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(in crate::outbound::plan_codec::adapters::json) enum ColTag {
    /// The tag.
    #[serde(rename = "col")]
    Col,
}
/// `"lit"`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(in crate::outbound::plan_codec::adapters::json) enum LitTag {
    /// The tag.
    #[serde(rename = "lit")]
    Lit,
}
/// `"num"`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(in crate::outbound::plan_codec::adapters::json) enum NumTag {
    /// The tag.
    #[serde(rename = "num")]
    Num,
}
/// `"list"`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(in crate::outbound::plan_codec::adapters::json) enum ListTag {
    /// The tag.
    #[serde(rename = "list")]
    List,
}
/// `"call"`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(in crate::outbound::plan_codec::adapters::json) enum CallTag {
    /// The tag.
    #[serde(rename = "call")]
    Call,
}
/// `"cmp"`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(in crate::outbound::plan_codec::adapters::json) enum CmpTag {
    /// The tag.
    #[serde(rename = "cmp")]
    Cmp,
}
/// The tags of the two-operand kinds.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(in crate::outbound::plan_codec::adapters::json) enum PairTag {
    /// `"in"`.
    #[serde(rename = "in")]
    In,
    /// `"notin"`.
    #[serde(rename = "notin")]
    NotIn,
    /// `"and"`.
    #[serde(rename = "and")]
    And,
    /// `"or"`.
    #[serde(rename = "or")]
    Or,
}
/// `"not"`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(in crate::outbound::plan_codec::adapters::json) enum NotTag {
    /// The tag.
    #[serde(rename = "not")]
    Not,
}

/// How a comparison's operator is written.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(in crate::outbound::plan_codec::adapters::json) enum ComparatorSchema {
    /// `=`.
    #[serde(rename = "=")]
    Equal,
    /// `!=`.
    #[serde(rename = "!=")]
    NotEqual,
    /// `<`.
    #[serde(rename = "<")]
    Less,
    /// `<=`.
    #[serde(rename = "<=")]
    LessOrEqual,
    /// `>`.
    #[serde(rename = ">")]
    Greater,
    /// `>=`.
    #[serde(rename = ">=")]
    GreaterOrEqual,
}

/// An expression as the plan writes it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(in crate::outbound::plan_codec::adapters::json) enum ExpressionSchema {
    /// `["col", name, alias]`.
    Column(ColTag, String, Option<String>),
    /// `["lit", value]`.
    Literal(LitTag, ConstantSchema),
    /// `["num", n]`.
    Number(NumTag, NumberSchema),
    /// `["list", [..]]`.
    List(ListTag, Vec<ExpressionSchema>),
    /// `["call", name, [..]]`.
    Call(CallTag, String, Vec<ExpressionSchema>),
    /// `["cmp", op, left, right]`.
    Compare(CmpTag, ComparatorSchema, Box<ExpressionSchema>, Box<ExpressionSchema>),
    /// `["in" | "notin" | "and" | "or", left, right]`.
    Pair(PairTag, Box<ExpressionSchema>, Box<ExpressionSchema>),
    /// `["not", value]`.
    Not(NotTag, Box<ExpressionSchema>),
}

/// A number as JSON: whole, or with a fraction.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(in crate::outbound::plan_codec::adapters::json) enum NumberSchema {
    /// Whole.
    Whole(i64),
    /// With a fraction.
    Fraction(f64),
}

/// A constant as JSON.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(in crate::outbound::plan_codec::adapters::json) enum ConstantSchema {
    /// `null`.
    Absent(()),
    /// Text.
    Text(String),
    /// A number.
    Number(NumberSchema),
    /// A list.
    List(Vec<ConstantSchema>),
    /// An object.
    Map(OrderedMap<ConstantSchema>),
}

impl From<&PlanNumber> for NumberSchema {
    fn from(number: &PlanNumber) -> Self {
        match number {
            PlanNumber::Whole(whole) => Self::Whole(whole.0),
            PlanNumber::Fraction(fraction) => Self::Fraction(f64::from_bits(fraction.0)),
        }
    }
}

impl From<NumberSchema> for PlanNumber {
    fn from(number: NumberSchema) -> Self {
        match number {
            NumberSchema::Whole(whole) => Self::Whole(WholeNumber(whole)),
            NumberSchema::Fraction(fraction) => Self::Fraction(FractionNumber(fraction.to_bits())),
        }
    }
}

impl From<&PlanConstant> for ConstantSchema {
    fn from(constant: &PlanConstant) -> Self {
        match constant {
            PlanConstant::Absent => Self::Absent(()),
            PlanConstant::Text(text) => Self::Text(text.0.to_string()),
            PlanConstant::Number(number) => Self::Number(number.into()),
            PlanConstant::List(items) => Self::List(items.iter().map(Self::from).collect()),
            PlanConstant::Map(entries) => Self::Map(OrderedMap(
                entries
                    .iter()
                    .map(|entry| (entry.key.0.to_string(), Self::from(&entry.value)))
                    .collect(),
            )),
        }
    }
}

impl From<ConstantSchema> for PlanConstant {
    fn from(constant: ConstantSchema) -> Self {
        match constant {
            ConstantSchema::Absent(()) => Self::Absent,
            ConstantSchema::Text(text) => Self::Text(LiteralText(text.into())),
            ConstantSchema::Number(number) => Self::Number(number.into()),
            ConstantSchema::List(items) => Self::List(items.into_iter().map(Self::from).collect()),
            ConstantSchema::Map(entries) => Self::Map(
                entries
                    .0
                    .into_iter()
                    .map(|(key, value)| ConstantEntry {
                        key: ConstantEntryKey(key.into()),
                        value: value.into(),
                    })
                    .collect(),
            ),
        }
    }
}

/// A boxed pair's schemas.
fn pair_schema(pair: &ExpressionPair) -> (Box<ExpressionSchema>, Box<ExpressionSchema>) {
    (
        Box::new((&*pair.left).into()),
        Box::new((&*pair.right).into()),
    )
}

impl From<&PlanExpression> for ExpressionSchema {
    fn from(expression: &PlanExpression) -> Self {
        match expression {
            PlanExpression::Column(read) => Self::Column(
                ColTag::Col,
                read.column.0.to_string(),
                match &read.alias {
                    SourceAlias::Unwritten => None,
                    SourceAlias::Written(alias) => Some(alias.0.to_string()),
                },
            ),
            PlanExpression::Literal(constant) => Self::Literal(LitTag::Lit, constant.into()),
            PlanExpression::Number(number) => Self::Number(NumTag::Num, number.into()),
            PlanExpression::List(items) => Self::List(ListTag::List, items.iter().map(Self::from).collect()),
            PlanExpression::Call(call) => Self::Call(
                CallTag::Call,
                call.function.0.to_string(),
                call.arguments.iter().map(Self::from).collect(),
            ),
            PlanExpression::Compare(comparison) => Self::Compare(
                CmpTag::Cmp,
                match comparison.comparator {
                    Comparator::Equal => ComparatorSchema::Equal,
                    Comparator::NotEqual => ComparatorSchema::NotEqual,
                    Comparator::Less => ComparatorSchema::Less,
                    Comparator::LessOrEqual => ComparatorSchema::LessOrEqual,
                    Comparator::Greater => ComparatorSchema::Greater,
                    Comparator::GreaterOrEqual => ComparatorSchema::GreaterOrEqual,
                },
                Box::new((&*comparison.left).into()),
                Box::new((&*comparison.right).into()),
            ),
            PlanExpression::In(pair) => {
                let (left, right) = pair_schema(pair);
                Self::Pair(PairTag::In, left, right)
            }
            PlanExpression::NotIn(pair) => {
                let (left, right) = pair_schema(pair);
                Self::Pair(PairTag::NotIn, left, right)
            }
            PlanExpression::And(pair) => {
                let (left, right) = pair_schema(pair);
                Self::Pair(PairTag::And, left, right)
            }
            PlanExpression::Or(pair) => {
                let (left, right) = pair_schema(pair);
                Self::Pair(PairTag::Or, left, right)
            }
            PlanExpression::Not(inner) => Self::Not(NotTag::Not, Box::new((&**inner).into())),
        }
    }
}

impl From<ExpressionSchema> for PlanExpression {
    fn from(expression: ExpressionSchema) -> Self {
        let boxed = |schema: Box<ExpressionSchema>| Box::new(Self::from(*schema));
        match expression {
            ExpressionSchema::Column(ColTag::Col, column, alias) => Self::Column(ColumnRead {
                column: ColumnName(column.into()),
                alias: alias.map_or(SourceAlias::Unwritten, |alias| {
                    SourceAlias::Written(AliasName(alias.into()))
                }),
            }),
            ExpressionSchema::Literal(LitTag::Lit, constant) => Self::Literal(constant.into()),
            ExpressionSchema::Number(NumTag::Num, number) => Self::Number(number.into()),
            ExpressionSchema::List(ListTag::List, items) => {
                Self::List(items.into_iter().map(Self::from).collect())
            }
            ExpressionSchema::Call(CallTag::Call, function, arguments) => Self::Call(FunctionCall {
                function: FunctionName(function.into()),
                arguments: arguments.into_iter().map(Self::from).collect(),
            }),
            ExpressionSchema::Compare(CmpTag::Cmp, comparator, left, right) => {
                Self::Compare(RowComparison {
                    comparator: match comparator {
                        ComparatorSchema::Equal => Comparator::Equal,
                        ComparatorSchema::NotEqual => Comparator::NotEqual,
                        ComparatorSchema::Less => Comparator::Less,
                        ComparatorSchema::LessOrEqual => Comparator::LessOrEqual,
                        ComparatorSchema::Greater => Comparator::Greater,
                        ComparatorSchema::GreaterOrEqual => Comparator::GreaterOrEqual,
                    },
                    left: boxed(left),
                    right: boxed(right),
                })
            }
            ExpressionSchema::Pair(tag, left, right) => {
                let pair = ExpressionPair {
                    left: boxed(left),
                    right: boxed(right),
                };
                match tag {
                    PairTag::In => Self::In(pair),
                    PairTag::NotIn => Self::NotIn(pair),
                    PairTag::And => Self::And(pair),
                    PairTag::Or => Self::Or(pair),
                }
            }
            ExpressionSchema::Not(NotTag::Not, inner) => Self::Not(boxed(inner)),
        }
    }
}
