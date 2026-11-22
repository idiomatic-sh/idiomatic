//! The types of the `expressions` part: a select's tests and items, and the constants a step is given. Plain data.

use crate::domain::descriptions::types::ColumnName;
use crate::domain::plan_parts::types::{LiteralText, WholeNumber};
use crate::domain::query_tokens::types::AliasName;

/// A value a select computes or tests, over the columns of its sources.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::{ColumnRead, PlanExpression, SourceAlias};
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// let read = PlanExpression::Column(ColumnRead { column: ColumnName("id".into()), alias: SourceAlias::Unwritten });
/// assert!(matches!(read, PlanExpression::Column(_)));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PlanExpression {
    /// A column of a source: `["col", name, alias]`.
    Column(ColumnRead),
    /// A constant the query wrote, or a parameter's value: `["lit", value]`.
    Literal(PlanConstant),
    /// A number the query wrote, or a parameter's number: `["num", n]`.
    Number(PlanNumber),
    /// A list of values, for `in`: `["list", [..]]`.
    List(Vec<PlanExpression>),
    /// A function of values: `["call", name, [..]]`.
    Call(FunctionCall),
    /// A comparison: `["cmp", op, left, right]`.
    Compare(RowComparison),
    /// Membership: `["in", value, list]`.
    In(ExpressionPair),
    /// No membership: `["notin", value, list]`.
    NotIn(ExpressionPair),
    /// Both: `["and", left, right]`.
    And(ExpressionPair),
    /// Either: `["or", left, right]`.
    Or(ExpressionPair),
    /// The negation: `["not", value]`.
    Not(Box<PlanExpression>),
}

/// A column of a select's source, read under the source's alias, or under none.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::{ColumnRead, SourceAlias};
/// use ryo_values::domain::descriptions::types::ColumnName;
/// use ryo_values::domain::query_tokens::types::AliasName;
///
/// let read = ColumnRead { column: ColumnName("file".into()), alias: SourceAlias::Written(AliasName("l".into())) };
/// assert_eq!(&*read.column.0, "file");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ColumnRead {
    /// The column.
    pub column: ColumnName,
    /// The alias the query wrote before it.
    pub alias: SourceAlias,
}

/// The alias a select gives a source, which its columns are read under: written after `as`, or none.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::SourceAlias;
/// use ryo_values::domain::query_tokens::types::AliasName;
///
/// assert_ne!(SourceAlias::Written(AliasName("l".into())), SourceAlias::Unwritten);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SourceAlias {
    /// The query wrote none.
    Unwritten,
    /// The query wrote this one.
    Written(AliasName),
}

/// A function the query names, applied to values.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::{FunctionCall, FunctionName};
///
/// let call = FunctionCall { function: FunctionName("lower".into()), arguments: Vec::new() };
/// assert!(call.arguments.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionCall {
    /// The function.
    pub function: FunctionName,
    /// Its arguments, in order.
    pub arguments: Vec<PlanExpression>,
}

/// The name of a function, as the query writes it: `matches`, `starts_with`.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::FunctionName;
///
/// assert_eq!(&*FunctionName("matches".into()).0, "matches");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionName(pub Box<str>);

/// A comparison of two values.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::{Comparator, PlanExpression, PlanNumber, RowComparison};
/// use ryo_values::domain::plan_parts::types::WholeNumber;
///
/// let one = || Box::new(PlanExpression::Number(PlanNumber::Whole(WholeNumber(1))));
/// let test = RowComparison { comparator: Comparator::Less, left: one(), right: one() };
/// assert_eq!(test.comparator, Comparator::Less);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RowComparison {
    /// How they are compared.
    pub comparator: Comparator,
    /// The left value.
    pub left: Box<PlanExpression>,
    /// The right value.
    pub right: Box<PlanExpression>,
}

/// How a comparison compares: `=`, `!=`, `<`, `<=`, `>`, `>=`.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::Comparator;
///
/// assert_ne!(Comparator::Equal, Comparator::NotEqual);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Comparator {
    /// `=`.
    Equal,
    /// `!=`.
    NotEqual,
    /// `<`.
    Less,
    /// `<=`.
    LessOrEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterOrEqual,
}

/// The two values of a binary expression.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::{ExpressionPair, PlanConstant, PlanExpression};
///
/// let absent = || Box::new(PlanExpression::Literal(PlanConstant::Absent));
/// let pair = ExpressionPair { left: absent(), right: absent() };
/// assert_eq!(pair.left, pair.right);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ExpressionPair {
    /// The first value.
    pub left: Box<PlanExpression>,
    /// The second value.
    pub right: Box<PlanExpression>,
}

/// A constant: text, a number, a list or a map of constants, or the value of a parameter no caller gave (`null`).
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::PlanConstant;
/// use ryo_values::domain::plan_parts::types::LiteralText;
///
/// let text = PlanConstant::Text(LiteralText("call".into()));
/// assert_ne!(text, PlanConstant::Absent);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PlanConstant {
    /// Text, verbatim between the quotes, or a word the query wrote unquoted.
    Text(LiteralText),
    /// A number.
    Number(PlanNumber),
    /// A list.
    List(Vec<PlanConstant>),
    /// A map, its entries in the order the query wrote them.
    Map(Vec<ConstantEntry>),
    /// No value: a parameter its caller did not give.
    Absent,
}

/// One entry of a constant map.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::{ConstantEntry, ConstantEntryKey, PlanConstant};
///
/// let entry = ConstantEntry { key: ConstantEntryKey("scope".into()), value: PlanConstant::Absent };
/// assert_eq!(&*entry.key.0, "scope");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstantEntry {
    /// Its key.
    pub key: ConstantEntryKey,
    /// Its value.
    pub value: PlanConstant,
}

/// The key of a constant map's entry.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::ConstantEntryKey;
///
/// assert_eq!(&*ConstantEntryKey("code".into()).0, "code");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstantEntryKey(pub Box<str>);

/// A number the query wrote: whole, or with a fraction.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::PlanNumber;
/// use ryo_values::domain::plan_parts::types::WholeNumber;
///
/// assert_eq!(PlanNumber::Whole(WholeNumber(2)), PlanNumber::Whole(WholeNumber(2)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlanNumber {
    /// A whole number.
    Whole(WholeNumber),
    /// A number with a fraction.
    Fraction(FractionNumber),
}

/// A number with a fraction, held as its bits so that two equal numbers are one step's.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::FractionNumber;
///
/// assert_eq!(FractionNumber(0.5_f64.to_bits()), FractionNumber(0.5_f64.to_bits()));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FractionNumber(pub u64);
