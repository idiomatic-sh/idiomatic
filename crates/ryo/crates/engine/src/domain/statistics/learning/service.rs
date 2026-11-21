//! The service of the `learning` part: what a sample says of each unit of the programs it ran, and what the second sample
//! of a rule set decides of the first's units. Work is counted, never timed: a unit's work is the runs of its plan points
//! and, for a table, the rows its fill read and gave. The one rule for a form: a flip candidate (a unit the lowering
//! marked open) ran as a call in the first sample and as a table in the second, and keeps the form that did less work;
//! nothing else changes form. The one rule for an order: a unit keeps its measured order unless the second sample, which
//! ran it in that order, did more work in it than the first by over one part in fifty and fifty steps.

use crate::domain::code::types::{FillCode, MeasuredForm, ShapeChoice, UnitKey, WordSlot};
use crate::domain::lowered::service::fill_code;
use crate::domain::lowered::types::MadeFill;
use crate::domain::statistics::constants::{ORDER_SHARE, ORDER_STEPS};
use crate::domain::statistics::types::{
    FieldSize, KindSize, LiteralSize, Measured, MeasuredSizes, MeasuredUnit, NamedSize, NamedSizes,
    OrderVerdict,
};
use ryo_language::domain::facts::service::{base_count, file_words, text};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::types::FieldCode;
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::service::{
    column_name, column_named, columns, name, relation_named,
};
use ryo_relations::domain::relations::types::Count;
use ryo_relations::domain::relations::types::KindBits;
use ryo_values::domain::descriptions::types::KindVocabulary;
use ryo_values::domain::plan_parts::types::RepeatedRows;

/// The unit of a name among units sorted by name.
#[must_use]
pub(crate) fn unit_named<'u>(units: &'u [MeasuredUnit], key: &UnitKey) -> Option<&'u MeasuredUnit> {
    match units.binary_search_by(|unit| unit.key.cmp(key)) {
        Ok(place) => units.get(place),
        Err(_) => None,
    }
}

/// The work a sample counted in a unit: the runs of its plan points, and the rows its fill read and gave.
#[must_use]
pub(crate) fn unit_work(unit: &MeasuredUnit) -> Count {
    let runs = unit.points.iter();
    Count(runs.fold(unit.counts.build.0, |sum, each| sum.saturating_add(each.0)))
}

/// Each unit of the programs a sample ran, with what the sample counted of it, in the form it ran in; sorted by name, a
/// name two units bear kept once (they are one program, lowered from one node).
#[must_use]
pub(crate) fn sampled(sample: Measured<'_>) -> Vec<MeasuredUnit> {
    let (code, counts) = (&sample.programs.0.code, &sample.counts.0);
    let place = |slot: WordSlot| usize::try_from(slot.0).unwrap_or(usize::MAX);
    let mut units: Vec<MeasuredUnit> = code
        .units
        .iter()
        .enumerate()
        .map(|(at, unit)| {
            let first = place(unit.first);
            let runs = counts
                .points
                .get(first..first.saturating_add(place(unit.points)));
            // A unit the sample never reached ran no point: its runs are noughts, one for each of its points.
            let points = runs.map_or_else(|| vec![Count(0); place(unit.points)], <[Count]>::to_vec);
            let counted = counts.units.get(at).copied().unwrap_or_default();
            MeasuredUnit {
                key: unit.key.clone(),
                form: unit.form,
                shape: unit.shape,
                order: OrderVerdict::Learned,
                points,
                counts: counted,
            }
        })
        .collect();
    units.sort_by(|left, right| left.key.cmp(&right.key));
    units.dedup_by(|later, kept| later.key == kept.key);
    units
}

/// What a sample sized, each under its name: the files and their elements; the elements of each kind and of each field
/// the programs' language names, by the name's text; and the rows of each constant bucket the programs' plans test (the
/// code's size sites), by the relation's and the column's catalog names and the value's text (a text as written, any
/// other cell as its number).
#[must_use]
pub(crate) fn sized(sample: Measured<'_>) -> NamedSizes {
    let (programs, counts) = (&sample.programs.0, &sample.counts.0);
    let words = file_words(&programs.words);
    let named = |codes: &[CellWord], rows: &[Count]| -> Vec<NamedSize> {
        let each = codes.iter().enumerate().map(|(code, word)| NamedSize {
            name: text(&words, *word).0.into(),
            rows: rows.get(code).copied().unwrap_or_default(),
        });
        // The empty name is no kind and no field (field nought is "no field"): nothing is asked by it.
        each.filter(|size| !size.name.is_empty()).collect()
    };
    let code = &programs.code;
    let place = |slot: WordSlot| usize::try_from(slot.0).unwrap_or(usize::MAX);
    let literals = code
        .sizes
        .iter()
        .zip(&counts.literals)
        .filter_map(|(site, rows)| {
            let relation = *code.relations.get(place(site.relation))?;
            let column = *columns(relation).get(usize::from(site.column.0))?;
            let value = *code.constants.get(place(site.value))?;
            let value = if value < base_count(&words) {
                text(&words, value).0.to_owned()
            } else {
                value.0.to_string()
            };
            let parts = [name(relation).0, column_name(column).0, value.as_str()];
            Some(NamedSize {
                name: parts.join("\n").into(),
                rows: *rows,
            })
        });
    NamedSizes {
        files: counts.files,
        elements: counts.elements,
        kinds: named(&programs.kinds, &counts.kinds),
        fields: named(&programs.fields, &counts.fields),
        literals: literals.collect(),
    }
}

/// The sizes of statistics read with a vocabulary: a kind and a field by the code the vocabulary gives its name, a name
/// it does not hold left out; a constant bucket by the relation and the column the catalog names so.
#[must_use]
pub(crate) fn coded(sizes: &NamedSizes, names: &KindVocabulary) -> MeasuredSizes {
    let code = |named: &NamedSize, all: Vec<&str>| {
        let place = all.iter().position(|each| *each == &*named.name)?;
        u16::try_from(place).ok()
    };
    let kinds = sizes.kinds.iter().filter_map(|named| {
        let kind = code(named, names.kinds.iter().map(|kind| &*kind.0).collect())?;
        Some(KindSize {
            kind: KindBits(kind),
            ids: named.rows,
        })
    });
    let fields = sizes.fields.iter().filter_map(|named| {
        let field = code(named, names.fields.iter().map(|field| &*field.0).collect())?;
        Some(FieldSize {
            field: FieldCode(field),
            rows: named.rows,
        })
    });
    let literals = sizes.literals.iter().filter_map(|named| {
        let mut parts = named.name.splitn(3, '\n');
        let relation = relation_named(NameRef(parts.next()?))?;
        let column = column_named(relation, NameRef(parts.next()?))?;
        Some(LiteralSize {
            relation,
            column,
            value: parts.next()?.into(),
            rows: named.rows,
        })
    });
    let (mut kinds, mut fields): (Vec<KindSize>, Vec<FieldSize>) =
        (kinds.collect(), fields.collect());
    kinds.sort_by_key(|size| size.kind);
    fields.sort_by_key(|size| size.field);
    MeasuredSizes {
        files: sizes.files,
        elements: sizes.elements,
        kinds,
        fields,
        literals: literals.collect(),
    }
}

/// The code of the fill of a definition's own rows: what the table of a pattern in table form is named with.
#[must_use]
pub(crate) fn rows_code() -> FillCode {
    fill_code(&MadeFill::Rows(Box::new([]), RepeatedRows::Kept))
}

/// The form a flip candidate keeps. Its call form cost the first sample the runs of its points. Its table form cost the
/// second sample its table's fill and the reads of the table, shared among the patterns of its definition that read the
/// table in table form. The table form is kept when it cost less; a candidate the second sample did not run as a table
/// keeps its call form.
fn form_kept(candidate: &MeasuredUnit, second: &[MeasuredUnit]) -> MeasuredForm {
    let node = match &candidate.key {
        UnitKey::Pattern(node, _, _) => *node,
        UnitKey::Rule(_) | UnitKey::Table(..) => return candidate.form,
    };
    let again = unit_named(second, &candidate.key).map(|unit| unit.form);
    let table = unit_named(second, &UnitKey::Table(node, rows_code()));
    match (candidate.form, again, table) {
        (MeasuredForm::Call, Some(MeasuredForm::Table), Some(table)) => {
            let readers = second.iter().filter(|unit| match &unit.key {
                UnitKey::Pattern(of, _, _) => *of == node && unit.form == MeasuredForm::Table,
                UnitKey::Rule(_) | UnitKey::Table(..) => false,
            });
            let as_table = unit_work(table).0.saturating_add(table.counts.calls.0);
            let as_call = unit_work(candidate).0.saturating_mul(readers.count());
            if as_table < as_call {
                MeasuredForm::Table
            } else {
                MeasuredForm::Call
            }
        }
        (MeasuredForm::Call, Some(MeasuredForm::Call | MeasuredForm::Table) | None, _)
        | (MeasuredForm::Table, _, _) => candidate.form,
    }
}

/// Whether a unit's measured order stands: the second sample ran it in that order, and did not do more work in it than the
/// first by over one part in fifty and fifty steps. A unit the second sample did not run was never run in its measured order.
fn order_kept(unit: &MeasuredUnit, second: &[MeasuredUnit]) -> OrderVerdict {
    let Some(again) = unit_named(second, &unit.key) else {
        return OrderVerdict::Denied;
    };
    let before = unit_work(unit).0;
    let allowed = before
        .saturating_add(before / ORDER_SHARE.0)
        .saturating_add(ORDER_STEPS.0);
    if unit_work(again).0 > allowed {
        OrderVerdict::Denied
    } else {
        OrderVerdict::Learned
    }
}

/// What the second sample decides of the first's units: each as the first counted it, a flip candidate in the form that did
/// less work, and each with its order's verdict. A candidate that keeps its call form is denied its measured order, which
/// never ran in that form; one that keeps its table form has no step of its own to order.
#[must_use]
pub(crate) fn decided(first: &[MeasuredUnit], second: &[MeasuredUnit]) -> Vec<MeasuredUnit> {
    let each = first.iter().map(|unit| {
        let (form, order) = match unit.shape {
            ShapeChoice::Open => match form_kept(unit, second) {
                MeasuredForm::Call => (MeasuredForm::Call, OrderVerdict::Denied),
                MeasuredForm::Table => (MeasuredForm::Table, OrderVerdict::Learned),
            },
            ShapeChoice::Decided => (unit.form, order_kept(unit, second)),
        };
        MeasuredUnit {
            form,
            order,
            ..unit.clone()
        }
    });
    each.collect()
}
