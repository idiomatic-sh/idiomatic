//! The cases of the `sets` part: which comparisons hold a text to a set of literals, however the plan writes the set, and
//! which stay a test of the text. Every expected value is what a run answered, read and found right.

use super::service::literals;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::fixtures::{digests_of, named};
use crate::domain::lowering::types::SelectedRule;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, LiteralText, Operand, ParameterId, PlanComparison, QueryLiteral,
    Test,
};
use ryo_values::domain::query_bindings::types::{ParameterBinding, RuleBindings};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// What stands on the right of a comparison.
#[derive(Clone, Copy, Debug)]
enum Right {
    /// A text the query wrote.
    Written(&'static str),
    /// A parameter bound to one text.
    Bound(&'static str),
    /// A parameter bound to a list of texts.
    Listed(&'static [&'static str]),
}

/// The test, and its right side; the left side is a column.
type Input = (Test, Right);
/// The literals, a space between two; none when the comparison holds its left side to no set of literals.
type Value = Option<Cow<'static, str>>;
type Failure = ();

const fn row(
    name: &'static str,
    input: fn() -> Input,
    expect: Option<&'static str>,
) -> Case<Input, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Succeeds,
        input,
        expect: Expect::Value(match expect {
            Some(set) => Some(Cow::Borrowed(set)),
            None => None,
        }),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    row(
        "a_glob_of_literal_alternatives_is_the_set_of_them",
        || (Test::GlobAny, Right::Written("{name,attribute,bin-op}")),
        Some("name attribute bin-op"),
    ),
    row(
        "a_glob_with_no_syntax_is_its_own_text",
        || (Test::GlobAny, Right::Written("yield-from")),
        Some("yield-from"),
    ),
    row(
        "a_literal_written_twice_is_in_the_set_once",
        || (Test::GlobAny, Right::Written("{name,call,name}")),
        Some("name call"),
    ),
    row(
        "a_glob_with_a_star_in_an_alternative_is_no_set",
        || (Test::GlobAny, Right::Written("{name,attr*}")),
        None,
    ),
    row(
        "a_glob_with_one_character_open_is_no_set",
        || (Test::GlobAny, Right::Written("nam?")),
        None,
    ),
    row(
        "a_glob_with_a_class_is_no_set",
        || (Test::GlobAny, Right::Written("{name,[ab]c}")),
        None,
    ),
    row(
        "a_glob_with_an_escape_is_no_set",
        || (Test::GlobAny, Right::Written("{name,a\\*}")),
        None,
    ),
    row(
        "a_glob_with_text_beside_its_group_is_no_set",
        || (Test::GlobAny, Right::Written("list-{comp,display}")),
        None,
    ),
    row(
        "a_glob_with_an_empty_alternative_is_no_set",
        || (Test::GlobAny, Right::Written("{name,}")),
        None,
    ),
    row(
        "a_glob_with_a_group_in_a_group_is_no_set",
        || (Test::GlobAny, Right::Written("{name,{call,await}}")),
        None,
    ),
    row(
        "a_glob_a_parameter_is_bound_to_is_read_as_one_the_query_wrote",
        || (Test::GlobAny, Right::Bound("{call,await}")),
        Some("call await"),
    ),
    row(
        "a_list_of_literal_globs_is_the_set_of_all_their_literals",
        || (Test::GlobAny, Right::Listed(&["name", "{call,await}"])),
        Some("name call await"),
    ),
    row(
        "a_list_with_one_wildcard_glob_is_no_set",
        || (Test::GlobAny, Right::Listed(&["name", "cal*"])),
        None,
    ),
    row(
        "an_in_list_is_the_set_of_its_texts",
        || (Test::In, Right::Listed(&["subject", "object", "subject"])),
        Some("subject object"),
    ),
    row(
        "an_in_list_takes_each_text_as_written_whatever_it_holds",
        || (Test::In, Right::Listed(&["a*", "{b,c}"])),
        Some("a* {b,c}"),
    ),
    row(
        "an_in_of_one_text_is_no_set",
        || (Test::In, Right::Bound("subject")),
        None,
    ),
    row(
        "an_expression_held_to_both_ends_over_a_group_of_words_is_the_set_of_them",
        || {
            (
                Test::Matches,
                Right::Written("^(?:name|bin-op|type_alias)$"),
            )
        },
        Some("name bin-op type_alias"),
    ),
    row(
        "a_group_that_captures_gives_the_same_set",
        || (Test::Matches, Right::Written("^(name|call)$")),
        Some("name call"),
    ),
    row(
        "an_expression_held_to_both_ends_over_one_word_is_that_word",
        || (Test::Matches, Right::Written("^name$")),
        Some("name"),
    ),
    row(
        "an_expression_that_searches_within_the_text_is_no_set",
        || (Test::Matches, Right::Written("name|call")),
        None,
    ),
    row(
        "an_expression_held_to_the_start_alone_is_no_set",
        || (Test::Matches, Right::Written("^(?:name|call)")),
        None,
    ),
    row(
        "an_expression_with_any_character_in_a_word_is_no_set",
        || (Test::Matches, Right::Written("^(?:na.e|call)$")),
        None,
    ),
    row(
        "an_expression_with_its_ends_inside_the_alternatives_is_no_set",
        || (Test::Matches, Right::Written("^name|call$")),
        None,
    ),
    row(
        "an_expression_a_parameter_is_bound_to_is_no_set",
        || (Test::Matches, Right::Bound("^(?:name|call)$")),
        None,
    ),
    row(
        "an_equality_is_no_set",
        || (Test::Equal, Right::Written("name")),
        None,
    ),
    row(
        "a_glob_that_must_not_match_is_no_set",
        || (Test::NotGlobAny, Right::Written("{name,call}")),
        None,
    ),
    row(
        "a_list_that_must_not_hold_the_text_is_no_set",
        || (Test::NotIn, Right::Listed(&["name", "call"])),
        None,
    ),
];

/// The literals of a comparison of column 1 with the right side, in a rule whose first parameter is bound as it says.
fn subject((test, right): Input) -> Result<Value, Failure> {
    let texts = |written: &[&str]| {
        written
            .iter()
            .map(|text| LiteralText((*text).into()))
            .collect()
    };
    let (operand, bound) = match right {
        Right::Written(text) => (
            Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
            Vec::new(),
        ),
        Right::Bound(text) => (
            Operand::Parameter(ParameterId(0)),
            vec![ParameterBinding::Text(LiteralText(text.into()))],
        ),
        Right::Listed(written) => (
            Operand::Parameter(ParameterId(0)),
            vec![ParameterBinding::Texts(texts(written))],
        ),
    };
    let plan = named("one tree rule");
    let digests = digests_of(&plan, RuleBindings(&bound));
    let rule = SelectedRule {
        rule: SelectedRuleOrdinal(0),
        plan: &plan,
        bindings: RuleBindings(&bound),
        digests: &digests,
    };
    let comparison = PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(1)),
        right: operand,
        origin: ComparisonOrigin::Unrecorded,
    };
    Ok(literals(&rule, &comparison).map(|set| {
        let words: Vec<&str> = set.iter().map(|literal| &*literal.0).collect();
        Cow::Owned(words.join(" "))
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
