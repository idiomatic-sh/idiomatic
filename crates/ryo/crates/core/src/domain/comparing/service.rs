//! The `comparing` service: whether comparisons hold for a row, with no IO and no ports.

use ryo_language::domain::relations::types::{CellKind, TextHash, Value};
use ryo_language::domain::symbols::service::{hash_in, hash_of, text_of};
use ryo_language::domain::symbols::types::ByteSlice;

use crate::domain::comparing::collections::comparisons::affix::item::AffixItem;
use crate::domain::comparing::collections::comparisons::equality::item::EqualityItem;
use crate::domain::comparing::collections::comparisons::float_text::item::FloatTextItem;
use crate::domain::comparing::collections::comparisons::glob::item::GlobItem;
use crate::domain::comparing::collections::comparisons::interface::ComparisonsItem as _;
use crate::domain::comparing::collections::comparisons::membership::item::MembershipItem;
use crate::domain::comparing::collections::comparisons::ordering::item::OrderingItem;
use crate::domain::functions::types::MadeText;
use crate::domain::globbing::service::{compile, matches_compiled};
use crate::domain::globbing::types::{CompiledGlobs, GlobMatch, GlobText, Subject};
use crate::domain::operands::service::resolve;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::family_of;
use crate::domain::operating::types::{ComparisonNumber, Family, Memo, NameCount, OperatorContext};
use crate::domain::table_model::service::{cell, column_in};
use crate::domain::table_model::types::{ParameterBinding, Verdict};
use crate::domain::table_scratch::types::{
    ComparisonNumbers, ComparisonSlot, GlobSubject, MemoPlace, MemoSlot, SubjectHash, SubjectTag,
    VerdictKey,
};
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_values::domain::plan_parts::service::columns_of;
use ryo_values::domain::plan_parts::types::{Comparison, Operand, QueryLiteral, Test};

/// How many verdicts about texts a worker keeps from file to file.
const VERDICTS: usize = 1 << 14;

/// Whether a name of the file is equal, or not equal, to a text the query wrote, when the comparison is that.
fn equal_to_text(
    comparison: &Comparison,
    row: &[Value],
    context: &OperatorContext<'_>,
) -> Option<Verdict> {
    let equal = match comparison.test {
        Test::Equal => true,
        Test::NotEqual => false,
        Test::LessThan
        | Test::LessThanOrEqual
        | Test::GreaterThan
        | Test::GreaterThanOrEqual
        | Test::FloatTextGreaterThanOrEqual
        | Test::In
        | Test::NotIn
        | Test::GlobAny
        | Test::NotGlobAny
        | Test::StartsWith
        | Test::Matches
        | Test::EndsWith => return None,
    };
    let ((Operand::Column(column), Operand::Literal(QueryLiteral::Text(text)))
    | (Operand::Literal(QueryLiteral::Text(text)), Operand::Column(column))) =
        (&comparison.left, &comparison.right)
    else {
        return None;
    };
    let Value::Symbol(symbol) = column_in(row, *column).copied()? else {
        return None;
    };
    let same = hash_in(context.symbols, symbol)? == hash_of(ByteSlice(text.0.as_bytes()))
        && text_of(context.symbols, ByteSlice(context.source.0), symbol).map(|found| found.0)
            == Some(&*text.0);
    Some(if same == equal {
        Verdict::Holds
    } else {
        Verdict::Fails
    })
}

/// Whether one comparison holds for one row, decided by the family its test belongs to.
fn holds(
    comparison: &Comparison,
    row: &[Value],
    bindings: &[ParameterBinding],
    texts: &Texts<'_>,
    compiled: &mut hashbrown::HashMap<ComparisonSlot, CompiledGlobs>,
    regexes: &mut crate::domain::table_scratch::types::Patterns,
) -> Result<Verdict, OperatingError> {
    let (mut left_made, mut right_made) = (MadeText::default(), MadeText::default());
    let left = resolve(&comparison.left, row, bindings, texts, &mut left_made)?;
    let right = resolve(&comparison.right, row, bindings, texts, &mut right_made)?;
    match family_of(comparison.test) {
        Family::Equality => EqualityItem.holds(comparison.test, &left, &right),
        Family::Ordering => OrderingItem.holds(comparison.test, &left, &right),
        Family::FloatText => FloatTextItem.holds(comparison.test, &left, &right),
        Family::Membership => MembershipItem.holds(comparison.test, &left, &right),
        Family::Affix => AffixItem.holds(comparison.test, &left, &right),
        Family::Regex => crate::domain::matching::service::matches(&left, &right, regexes),
        Family::Glob => {
            if let Some(verdict) = compiled_glob(comparison, &left, &right, compiled) {
                return Ok(verdict);
            }
            GlobItem.holds(comparison.test, &left, &right)
        }
    }
}

/// Uses a fixed operand's compiled matcher; a column operand remains row-dependent.
fn compiled_glob(
    comparison: &Comparison,
    left: &Datum<'_>,
    right: &Datum<'_>,
    compiled: &mut hashbrown::HashMap<ComparisonSlot, CompiledGlobs>,
) -> Option<Verdict> {
    let Datum::Text(text) = left else {
        return None;
    };
    if !columns_of(&comparison.right).is_empty() {
        return None;
    }
    let key = ComparisonSlot(std::ptr::from_ref(comparison).addr());
    let ready = compiled.entry(key).or_insert_with(|| match right {
        Datum::Text(pattern) => compile(&[GlobText(pattern.0)]),
        Datum::Texts(patterns) => compile(
            &patterns
                .iter()
                .map(|pattern| GlobText(&pattern.0))
                .collect::<Vec<_>>(),
        ),
        Datum::Unlocated | Datum::Number(_) | Datum::Flag(_) | Datum::Numbers(_) => {
            CompiledGlobs::Interpreted
        }
    });
    let matched = matches_compiled(ready, Subject(text.0))?;
    Some(
        if (matched == GlobMatch::Matches) == (comparison.test == Test::GlobAny) {
            Verdict::Holds
        } else {
            Verdict::Fails
        },
    )
}

/// What the verdict of a glob is kept under when it asks of one name or of one file's path something that does not depend on the
/// row: the tag of the key (0 for a name, 1 for a file) and the hash of the name's text or the number of the file.
fn glob_subject(comparison: &Comparison, row: &[Value]) -> Option<GlobSubject> {
    let column = match (&comparison.left, &comparison.right) {
        (Operand::Column(index), Operand::Parameter(_) | Operand::Literal(_))
        | (Operand::Parameter(_) | Operand::Literal(_), Operand::Column(index)) => *index,
        _ => return None,
    };
    match column_in(row, column).copied()? {
        Value::Name(name) => Some(GlobSubject {
            tag: SubjectTag::Name,
            hash: SubjectHash(name.hash.0),
        }),
        Value::File(file) => Some(GlobSubject {
            tag: SubjectTag::File,
            hash: SubjectHash(u64::from(file.0)),
        }),
        Value::Node(_) | Value::Symbol(_) | Value::Span(_) | Value::Number(_) | Value::Flag(_) => {
            None
        }
    }
}

/// Where a comparison's decision about a name is remembered: the comparison compares one name
/// column with something that does not depend on the row, and the row's cell is a name of the file.
fn memo_place(
    number: ComparisonNumber,
    comparison: &Comparison,
    row: &[Value],
    kinds: &[CellKind],
    names: NameCount,
) -> Option<MemoPlace> {
    let ((Operand::Column(index), Operand::Parameter(_) | Operand::Literal(_))
    | (Operand::Parameter(_) | Operand::Literal(_), Operand::Column(index))) =
        (&comparison.left, &comparison.right)
    else {
        return None;
    };
    let column = *index;
    if !matches!(column_in(kinds, column), Some(CellKind::Symbol)) {
        return None;
    }
    let Ok(Value::Symbol(symbol)) = cell(row, column) else {
        return None;
    };
    Some(MemoPlace {
        place: MemoSlot(number.0 * names.0 + usize::try_from(symbol.0).unwrap_or(usize::MAX)),
        symbol,
    })
}

/// Whether the comparison numbered `number` holds for the row. A comparison of a name with something fixed is
/// decided once for each distinct name of the file, as the prototype decides a name's predicates once
/// when it interns the name.
fn holds_one(
    number: ComparisonNumber,
    comparison: &Comparison,
    row: &[Value],
    kinds: &[CellKind],
    names: NameCount,
    context: &mut OperatorContext<'_>,
) -> Result<Verdict, OperatingError> {
    // A name equal to a fixed text is told by the hash of its text, which the file keeps for each name: the text is
    // read only when the hashes agree.
    if let Some(verdict) = equal_to_text(comparison, row, context) {
        return Ok(verdict);
    }
    let OperatorContext {
        bindings,
        symbols,
        source,
        paths,
        arenas,
        memo,
        scratch,
    } = context;
    let texts = Texts {
        symbols,
        source: *source,
        paths,
        arenas,
    };
    // A glob of a name that left its file is decided once for each name of the run, as the hash of its text tells: a name
    // is the same text in every file. A glob of a file's path is decided once for each file of the run: the path of a file is
    // the same text in every row that names it (the key of a file is the comparison's address with its low bit set, which
    // no comparison's address has, so the two kinds of key do not meet).
    if family_of(comparison.test) == Family::Glob
        && let Some(GlobSubject { tag, hash }) = glob_subject(comparison, row)
    {
        let tag = match tag {
            SubjectTag::Name => 0,
            SubjectTag::File => 1,
        };
        let key = VerdictKey {
            comparison: ComparisonSlot(std::ptr::from_ref(comparison).addr() | tag),
            text: TextHash(hash.0),
        };
        if let Some(verdict) = scratch.verdicts.get(&key) {
            return Ok(*verdict);
        }
        let verdict = holds(
            comparison,
            row,
            bindings,
            &texts,
            &mut scratch.compiled_globs,
            &mut scratch.compiled_regexes,
        )?;
        if scratch.verdicts.len() >= VERDICTS {
            scratch.verdicts.clear();
        }
        let _: Option<Verdict> = scratch.verdicts.insert(key, verdict);
        return Ok(verdict);
    }
    let place = memo_place(number, comparison, row, kinds, names);
    let remembered = place.and_then(|each| memo.get(each.place.0).copied());
    Ok(match remembered {
        Some(Memo::Holds) => Verdict::Holds,
        Some(Memo::Fails) => Verdict::Fails,
        Some(Memo::Unknown) | None => {
            // The same text is the same verdict in every file of the run: the comparison is told apart by where it is.
            // Only a glob is worth keeping: an equality is decided as fast as it is looked up.
            let known = place
                .filter(|_| family_of(comparison.test) == Family::Glob)
                .and_then(|each| hash_in(texts.symbols, each.symbol))
                .map(|hash| VerdictKey {
                    comparison: ComparisonSlot(std::ptr::from_ref(comparison).addr()),
                    text: hash,
                });
            let decided =
                if let Some(verdict) = known.and_then(|key| scratch.verdicts.get(&key).copied()) {
                    verdict
                } else {
                    let verdict = holds(
                        comparison,
                        row,
                        bindings,
                        &texts,
                        &mut scratch.compiled_globs,
                        &mut scratch.compiled_regexes,
                    )?;
                    if let Some(key) = known {
                        // The verdicts of the texts met so far are kept for the next file; a run with more texts
                        // than this forgets them, and decides again the texts that come back.
                        if scratch.verdicts.len() >= VERDICTS {
                            scratch.verdicts.clear();
                        }
                        let _: Option<Verdict> = scratch.verdicts.insert(key, verdict);
                    }
                    verdict
                };
            if let Some(held) = place.and_then(|each| memo.get_mut(each.place.0)) {
                *held = match decided {
                    Verdict::Holds => Memo::Holds,
                    Verdict::Fails => Memo::Fails,
                };
            }
            decided
        }
    })
}

/// Whether every comparison holds for the row.
pub(crate) fn holds_all(
    comparisons: &[Comparison],
    row: &[Value],
    kinds: &[CellKind],
    names: NameCount,
    context: &mut OperatorContext<'_>,
) -> Result<Verdict, OperatingError> {
    for (number, comparison) in comparisons.iter().enumerate() {
        if holds_one(
            ComparisonNumber(number),
            comparison,
            row,
            kinds,
            names,
            context,
        )? == Verdict::Fails
        {
            return Ok(Verdict::Fails);
        }
    }
    Ok(Verdict::Holds)
}

/// Whether the comparisons numbered `which` hold for the row.
pub(crate) fn holds_some(
    comparisons: &[Comparison],
    which: &ComparisonNumbers,
    row: &[Value],
    kinds: &[CellKind],
    names: NameCount,
    context: &mut OperatorContext<'_>,
) -> Result<Verdict, OperatingError> {
    for number in &which.0 {
        let Some(comparison) = comparisons.get(*number) else {
            continue;
        };
        if holds_one(
            ComparisonNumber(*number),
            comparison,
            row,
            kinds,
            names,
            context,
        )? == Verdict::Fails
        {
            return Ok(Verdict::Fails);
        }
    }
    Ok(Verdict::Holds)
}
