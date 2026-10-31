//! The `kind_sets` service: the comparisons that name a kind, as sets of kinds, with no IO and no ports.

use std::hash::BuildHasher as _;

use ryo_language::domain::elements::service::{kind_names, kind_vocabulary};
use ryo_language::domain::kind_index::service::of_kind;
use ryo_language::domain::relations::constants::NO_SYMBOL;
use ryo_language::domain::relations::types::{KindBits, TextHash};
use ryo_language::domain::symbols::service::text_of;
use ryo_language::domain::symbols::types::ByteSlice;

use super::constants::{NO_NAME, SET_OF_TEST};
use super::types::KindScratch;
use crate::domain::comparing::collections::comparisons::affix::item::AffixItem;
use crate::domain::comparing::collections::comparisons::equality::item::EqualityItem;
use crate::domain::comparing::collections::comparisons::glob::item::GlobItem;
use crate::domain::comparing::collections::comparisons::interface::ComparisonsItem as _;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operating::types::{KindNaming, KindNarrowing};
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::types::{SetMembership, Verdict};
use crate::domain::table_scratch::service::kind_has;
use crate::domain::table_scratch::types::{
    Candidates, KindSet, KindSetPlace, KindTest, NamingPlace, NamingTest,
};
use crate::domain::table_texts::types::{Datum, TextView};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Comparison, Operand, QueryLiteral, Test};

/// Whether the comparison asks of the kind of an element something fixed, which the sets of kinds decide.
pub(crate) fn names_a_kind(comparison: &Comparison) -> KindNaming {
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
        | Test::FloatTextGreaterThanOrEqual
        | Test::In
        | Test::NotIn
        | Test::StartsWith
        | Test::Matches
        | Test::EndsWith => None,
    }
}

/// Whether the name of a kind satisfies the test against `wanted`.
pub(crate) fn satisfies(test: Test, name: TextView<'_>, wanted: TextView<'_>) -> Verdict {
    let (left, right) = (Datum::Text(name), Datum::Text(wanted));
    match test {
        Test::Equal | Test::NotEqual => EqualityItem.holds(test, &left, &right),
        Test::StartsWith | Test::EndsWith => AffixItem.holds(test, &left, &right),
        Test::Matches => crate::domain::matching::service::matches(
            &left,
            &right,
            &mut crate::domain::matching::types::Patterns::default(),
        ),
        Test::LessThan
        | Test::LessThanOrEqual
        | Test::GreaterThan
        | Test::GreaterThanOrEqual
        | Test::FloatTextGreaterThanOrEqual
        | Test::In
        | Test::NotIn
        | Test::GlobAny
        | Test::NotGlobAny => GlobItem.holds(test, &left, &right),
    }
    .unwrap_or(Verdict::Fails)
}

/// Reads the comparisons of a filter that name a kind into `tests`, each as the set of kinds that
/// satisfy it. The sets are made once for a test and its text, from the kinds of the first file, and
/// found by them in every file of that language after: a kind has the same name in every file of a language, and another language
/// has its own kinds.
pub(crate) fn kind_tests(
    tree: &TreeRows<'_>,
    comparisons: &[Comparison],
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
    for comparison in comparisons {
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
            text: TextHash(rustc_hash::FxBuildHasher.hash_one(&*named.0)),
        };
        let set = if let Some(set) = scratch.names.get(&asked) {
            *set
        } else {
            let mut made = KindSet::default();
            made.bits
                .0
                .resize(kind_names(tree.elements).len().div_ceil(64), 0);
            for (bits, symbol) in kind_names(tree.elements).iter().enumerate() {
                let name = if *symbol == NO_SYMBOL {
                    Some(NO_NAME)
                } else {
                    text_of(context.symbols, ByteSlice(context.source.0), *symbol)
                        .map(|found| found.0)
                };
                if name.is_some_and(|name| {
                    satisfies(comparison.test, TextView(name), TextView(&named.0)) == Verdict::Holds
                }) {
                    if let Some(word) = made.bits.0.get_mut(bits / 64) {
                        *word |= 1 << (bits % 64);
                    }
                    made.kinds.extend(u16::try_from(bits).map(KindBits));
                }
            }
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

/// The places of the elements of a tree that the comparisons of their kind leave, when one of them
/// names few enough kinds: found among the elements of those kinds, in the order of the tree.
pub(crate) fn candidates_of(
    tree: &TreeRows<'_>,
    scratch: &KindScratch,
    candidates: &mut Candidates,
) -> KindNarrowing {
    let candidates = &mut candidates.0;
    let Some(set) = scratch
        .tests
        .0
        .iter()
        .map(|set| set_at(scratch, KindSetPlace(*set)))
        .min_by_key(|named| named.kinds.len())
    else {
        return KindNarrowing::Whole;
    };
    let kinds = &set.kinds;
    if kinds.len() * 4 > kind_names(tree.elements).len() {
        return KindNarrowing::Whole;
    }
    candidates.clear();
    for kind in kinds {
        candidates.extend(
            of_kind(tree.elements, *kind)
                .iter()
                .map(|position| position.0),
        );
    }
    if kinds.len() > 1 {
        candidates.sort_unstable();
    }
    KindNarrowing::Narrowed
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
