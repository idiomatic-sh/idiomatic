//! The `kind_sets` service: the comparisons that name a kind, as sets of kinds, with no IO and no ports.

use ryo_language::domain::elements::service::kind_names;
use ryo_language::domain::symbols::service::text_of;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::constants::NO_SYMBOL;

use super::constants::NO_NAME;
use super::types::vocabulary::value::PreparedVocabulary;
use crate::domain::operating::types::KindNaming;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::comparing::collections::comparisons::affix::item::AffixItem;
use ryo_operators::domain::comparing::collections::comparisons::equality::item::EqualityItem;
use ryo_operators::domain::comparing::collections::comparisons::glob::item::GlobItem;
use ryo_operators::domain::comparing::collections::comparisons::interface::ComparisonsItem as _;
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_operators::domain::patterns::types::PatternMatcher;
use ryo_operators::domain::table_scratch::types::NamingPlace;
use ryo_operators::domain::table_texts::types::{Datum, TextView};
use ryo_relations::domain::relations::service::kind_column;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::descriptions::types::KindWord;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::service::column_place;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Operand, PlanComparison, QueryLiteral, Test,
};
use std::sync::LazyLock;

/// The place of the kind among the columns of the syntax tree, found once: the files' walks read it without making the relation's
/// columns again.
static ELEMENT_KIND: LazyLock<Option<ColumnIndex>> =
    LazyLock::new(|| kind_column(Relation::SyntaxTree).map(column_place));

/// Whether the comparison asks of the kind of an element something fixed, which the sets of kinds decide.
pub fn names_a_kind(comparison: &PlanComparison) -> KindNaming {
    if naming(comparison.test).is_some()
        && matches!(
            (&comparison.left, &comparison.right),
            (
                Operand::Column(column),
                Operand::Literal(QueryLiteral::Text(_))
            ) if Some(*column) == *ELEMENT_KIND
        )
    {
        KindNaming::Names
    } else {
        KindNaming::Silent
    }
}

/// The place of a test among the four that can name a kind, with whether the kind must be in the set it names.
fn naming(test: Test) -> Option<NamingPlace> {
    match test {
        Test::Equal => Some(NamingPlace(0)),
        Test::NotEqual => Some(NamingPlace(1)),
        Test::GlobAny => Some(NamingPlace(2)),
        Test::NotGlobAny => Some(NamingPlace(3)),
        Test::LessThan
        | Test::LessThanOrEqual
        | Test::GreaterThan
        | Test::GreaterThanOrEqual
        | Test::NaturalLessThan
        | Test::FloatTextGreaterThanOrEqual
        | Test::NumericGreaterThanOrEqual
        | Test::IntegerSuccessor
        | Test::In
        | Test::NotIn
        | Test::StartsWith
        | Test::Matches
        | Test::EndsWith => None,
    }
}

/// Whether the name of a kind satisfies the test against `wanted`.
pub fn satisfies(
    test: Test,
    name: TextView<'_>,
    wanted: TextView<'_>,
    matcher: PatternMatcher<'_>,
) -> PredicateOutcome {
    let (left, right) = (Datum::Text(name), Datum::Text(wanted));
    match test {
        Test::Equal | Test::NotEqual => EqualityItem.holds(test, &left, &right),
        Test::StartsWith | Test::EndsWith => AffixItem.holds(test, &left, &right),
        Test::Matches => ryo_operators::domain::patterns::service::matches(
            &left,
            &right,
            &mut ryo_operators::domain::table_scratch::types::Patterns::default(),
            matcher,
        )
        .map_err(ryo_operators::domain::comparing::errors::ComparingError::from),
        Test::LessThan
        | Test::LessThanOrEqual
        | Test::GreaterThan
        | Test::GreaterThanOrEqual
        | Test::NaturalLessThan
        | Test::FloatTextGreaterThanOrEqual
        | Test::NumericGreaterThanOrEqual
        | Test::IntegerSuccessor
        | Test::In
        | Test::NotIn
        | Test::GlobAny
        | Test::NotGlobAny => GlobItem(matcher).holds(test, &left, &right),
    }
    .unwrap_or(PredicateOutcome::Fails)
}

/// Resolve ordered names once at the file boundary; an absent spelling retains the old nonmatching behavior.
fn vocabulary_names(
    elements: &ryo_language::domain::elements::types::Elements,
    symbols: &Symbols,
    source: SourceView<'_>,
) -> Vec<Option<KindWord>> {
    kind_names(elements)
        .iter()
        .map(|symbol| {
            if *symbol == NO_SYMBOL {
                Some(KindWord(NO_NAME.into()))
            } else {
                text_of(symbols, ByteSlice(source.0), *symbol).map(|text| KindWord(text.0.into()))
            }
        })
        .collect()
}

/// Select one exact vocabulary generation. Replacing a generation drops all old literal metadata
/// once the previous file's own handle ends; distinct file vocabularies cannot accumulate indefinitely.
pub fn prepare_vocabulary(
    previous: &mut Option<std::sync::Arc<PreparedVocabulary>>,
    elements: &ryo_language::domain::elements::types::Elements,
    symbols: &Symbols,
    source: SourceView<'_>,
) -> Option<std::sync::Arc<PreparedVocabulary>> {
    if kind_names(elements).is_empty() {
        return None;
    }
    let names = vocabulary_names(elements, symbols, source);
    if let Some(prepared) = previous.as_ref().filter(|prepared| prepared.names == names) {
        return Some(std::sync::Arc::clone(prepared));
    }
    let prepared = std::sync::Arc::new(PreparedVocabulary { names });
    *previous = Some(std::sync::Arc::clone(&prepared));
    Some(prepared)
}
