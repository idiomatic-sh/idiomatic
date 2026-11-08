//! The `kind_sets` service: the comparisons that name a kind, as sets of kinds, with no IO and no ports.

use ryo_language::domain::elements::service::{kind_names, kind_vocabulary};
use ryo_language::domain::symbols::service::text_of;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::constants::NO_SYMBOL;
use ryo_relations::domain::relations::types::KindBits;

use super::constants::{NO_NAME, SET_OF_TEST};
use crate::domain::matching::types::PatternMatcher;
use super::types::vocabulary::value::{PreparedKindTest, PreparedVocabulary};
use super::types::{ComparisonOrdinal, KindScratch, ListAddress};
use crate::domain::comparing::collections::comparisons::affix::item::AffixItem;
use crate::domain::comparing::collections::comparisons::equality::item::EqualityItem;
use crate::domain::comparing::collections::comparisons::glob::item::GlobItem;
use crate::domain::comparing::collections::comparisons::interface::ComparisonsItem as _;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operating::types::KindNaming;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::types::{SetMembership, Verdict};
use crate::domain::table_scratch::service::kind_has;
use crate::domain::table_scratch::types::{ KindSet, KindSetPlace, KindTest, NamingPlace, NamingTest};
use crate::domain::table_texts::types::{Datum, TextView};
use ryo_values::domain::descriptions::types::KindWord;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, LiteralText, Operand, PlanComparison, QueryLiteral, Test};

/// Whether the comparison asks of the kind of an element something fixed, which the sets of kinds decide.
pub fn names_a_kind(comparison: &PlanComparison) -> KindNaming {
    if naming(comparison.test).is_some()
        && matches!(
            (&comparison.left, &comparison.right),
            (
                Operand::Column(ColumnIndex(1)),
                Operand::Literal(QueryLiteral::Text(_))
            )
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
) -> Verdict {
    let (left, right) = (Datum::Text(name), Datum::Text(wanted));
    match test {
        Test::Equal | Test::NotEqual => EqualityItem.holds(test, &left, &right),
        Test::StartsWith | Test::EndsWith => AffixItem.holds(test, &left, &right),
        Test::Matches => crate::domain::matching::service::matches(
            &left,
            &right,
            &mut crate::domain::table_scratch::types::Patterns::default(),
            matcher,
        ),
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
    .unwrap_or(Verdict::Fails)
}

/// Reads the comparisons of a filter that name a kind into `tests`, each as the set of kinds that
/// satisfy it. The sets are made once for a test and its text, from the kinds of the first file, and
/// found by them in every file of that language after: a kind has the same name in every file of a language, and another language
/// has its own kinds.
pub(crate) fn kind_tests(
    tree: &TreeRows<'_>,
    comparisons: &[PlanComparison],
    context: &mut OperatorContext<'_>,
    scratch: &mut KindScratch,
) {
    scratch.tests.0.clear();
    // A file nothing read names no kind: it is no ground to decide what a name means.
    if tree.form != TreeForm::Elements || kind_names(tree.elements).is_empty() {
        return;
    }
    // A kind has the same name in every file of a language, and another language numbers its kinds otherwise: the sets are the
    // language's, found by what its kinds are called.
    let vocabulary = kind_vocabulary(tree.elements);
    for (ordinal, comparison) in comparisons.iter().enumerate() {
        let (
            Some(which),
            Operand::Column(ColumnIndex(1)),
            Operand::Literal(QueryLiteral::Text(named)),
        ) = (naming(comparison.test), &comparison.left, &comparison.right)
        else {
            continue;
        };
        let asked = KindTest {
            vocabulary,
            test: NamingTest(u8::try_from(which.0).unwrap_or(u8::MAX)),
            text: named.clone(),
        };
        let set = if let Some(set) = scratch.names.get(&asked) {
            *set
        } else {
            let key = PreparedKindTest {
                comparisons: ListAddress(comparisons.as_ptr().addr()),
                ordinal: ComparisonOrdinal(ordinal),
                test: comparison.test,
                text: named.clone(),
            };
            let made = if let Some(prepared) = context.prepared_kinds {
                #[cfg(test)]
                let _previous = prepared
                    .lookups
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let mut masks = prepared
                    .masks
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                std::sync::Arc::clone(masks.entry(key).or_insert_with(|| {
                    #[cfg(test)]
                    let _previous = prepared
                        .builds
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    std::sync::Arc::new(make_set(&prepared.names, comparison.test, named, context.matcher))
                }))
            } else {
                let names =
                    vocabulary_names(tree.elements, context.symbols.as_ref(), context.source);
                std::sync::Arc::new(make_set(&names, comparison.test, named, context.matcher))
            };
            scratch.sets.push(made);
            let _: Option<KindSetPlace> = scratch
                .names
                .insert(asked, KindSetPlace(scratch.sets.len() - 1));
            KindSetPlace(scratch.sets.len() - 1)
        };
        scratch.tests.0.push(set.0);
    }
}

/// The set of kinds at `place`.
///
/// # Panics
///
/// On a place no set is at: a test's place is pushed with its set, so another is a bug that must crash, not fail every kind.
#[expect(
    clippy::expect_used,
    reason = "a test's place is pushed with its set; a place past the sets is a bug that must crash, not fail every kind"
)]
fn set_at(scratch: &KindScratch, place: KindSetPlace) -> &KindSet {
    scratch.sets.get(place.0).expect(SET_OF_TEST)
}

/// Whether an element of this kind passes every comparison that names a kind.
pub(crate) fn passes(scratch: &KindScratch, kind: KindBits) -> Verdict {
    if scratch
        .tests
        .0
        .iter()
        .all(|set| kind_has(set_at(scratch, KindSetPlace(*set)), kind) == SetMembership::In)
    {
        Verdict::Holds
    } else {
        Verdict::Fails
    }
}

/// The kinds of the smallest set the tests name, in order: every kind that passes them is among these.
pub(crate) fn named_kinds(scratch: &KindScratch) -> Option<&[KindBits]> {
    scratch
        .tests
        .0
        .iter()
        .map(|set| set_at(scratch, KindSetPlace(*set)))
        .min_by_key(|named| named.kinds.len())
        .map(|named| named.kinds.as_slice())
}

/// Resolve ordered names once at the file boundary; an absent spelling retains the old nonmatching behavior.
fn vocabulary_names(
    elements: &ryo_language::domain::elements::types::Elements,
    symbols: &ryo_language::domain::symbols::types::Symbols,
    source: ryo_values::domain::files::types::SourceView<'_>,
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
    symbols: &ryo_language::domain::symbols::types::Symbols,
    source: ryo_values::domain::files::types::SourceView<'_>,
) -> Option<std::sync::Arc<PreparedVocabulary>> {
    if kind_names(elements).is_empty() {
        return None;
    }
    let names = vocabulary_names(elements, symbols, source);
    if let Some(prepared) = previous.as_ref().filter(|prepared| prepared.names == names) {
        return Some(std::sync::Arc::clone(prepared));
    }
    let prepared = std::sync::Arc::new(PreparedVocabulary {
        names,
        masks: std::sync::Mutex::default(),
        #[cfg(test)]
        lookups: std::sync::atomic::AtomicUsize::new(0),
        #[cfg(test)]
        builds: std::sync::atomic::AtomicUsize::new(0),
    });
    *previous = Some(std::sync::Arc::clone(&prepared));
    Some(prepared)
}

/// Evaluate a literal predicate once over a vocabulary, never over source rows.
fn make_set(
    names: &[Option<KindWord>],
    test: Test,
    literal: &LiteralText,
    matcher: PatternMatcher<'_>,
) -> KindSet {
    let mut made = KindSet::default();
    made.bits.0.resize(names.len().div_ceil(64), 0);
    for (bits, name) in names.iter().enumerate() {
        if name.as_ref().is_some_and(|name| {
            satisfies(test, TextView(&name.0), TextView(&literal.0), matcher) == Verdict::Holds
        }) {
            if let Some(word) = made.bits.0.get_mut(bits / 64) {
                *word |= 1 << (bits % 64);
            }
            made.kinds.extend(u16::try_from(bits).map(KindBits));
        }
    }
    made
}
