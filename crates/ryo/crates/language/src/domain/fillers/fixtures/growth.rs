//! The growth harness of the `fillers` concept, which every language's contract test runs: it holds a language's fillers to what
//! the catalog declares of each relation they fill. A whole fill's rows grow with the file as the relation's `size` says, and a
//! fill by a declared key answers `Asked`, writes that key's rows alone, and writes no more for a larger file.

use crate::domain::facts::service::{base_words, file_words};
use crate::domain::facts::types::{
    CellWord, FactAnswer, FileFacts, FillExtent, FilledColumns, FixedNames, Wanted,
};
use crate::domain::node_table::service::element_count;
use ryo_relations::domain::catalog::service::{fact_form, key_columns, relation_size};
use ryo_relations::domain::relations::service::{all, columns};
use ryo_relations::domain::relations::types::{Column, Count, FactForm, Relation, RelationSize};

/// One relation's rows at the two sizes of a growth run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Growth {
    /// The relation.
    pub relation: Relation,
    /// What its catalog item declares.
    pub size: RelationSize,
    /// The rows a whole fill wrote for the smaller file.
    pub small: Count,
    /// The rows a whole fill wrote for the larger file.
    pub large: Count,
}

/// One declared key's rows at the two sizes of a growth run: the rows a fill of the key with the fewest rows wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyGrowth {
    /// The declared key column.
    pub column: Column,
    /// The rows a fill of one key wrote for the smaller file.
    pub small: Count,
    /// The rows a fill of one key wrote for the larger file.
    pub large: Count,
}

/// What a growth run found that a relation's catalog item does not declare, or could not judge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrowthFault {
    /// The language lent no file for one of the two sizes, or the larger is under three times the smaller's elements.
    Sources,
    /// No relation was answered: there is nothing to hold.
    NothingAnswered,
    /// The relation was answered in part, for one of the two files alone, or with no row for the larger: its growth cannot be
    /// judged.
    Unproven(Relation),
    /// The relation's rows grew faster than its catalog item declares: a `PerFile` relation held more than one row, or a
    /// `PerElement` relation's rows grew by more than nine eighths of the file's own growth (4.5 times at four times the
    /// elements).
    Grew(Relation),
    /// A fill of a declared key did not answer `Asked`: it wrote the relation whole, in part, or refused.
    KeyNotAsked(Column),
    /// A fill of a declared key wrote other rows than the rows of the whole fill that hold the key.
    KeyRows(Column),
    /// A fill of one key of a declared column wrote more rows for the larger file.
    KeyGrew(Column),
}

/// What a growth run measured and what it found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrowthReport {
    /// The elements of the smaller file the language lent.
    pub small: Count,
    /// The elements of the larger file.
    pub large: Count,
    /// Each relation the language answered, in the catalog's order.
    pub relations: Vec<Growth>,
    /// Each declared key of those relations.
    pub keys: Vec<KeyGrowth>,
    /// What does not hold; empty when every filler is as its relation's catalog item declares.
    pub faults: Vec<GrowthFault>,
}

/// One relation of one lent file: the rows of its whole fill and, for each declared key, what a fill of one key did.
struct Measured {
    /// The relation.
    relation: Relation,
    /// The rows the whole fill wrote; none when it was answered in part.
    rows: Option<Count>,
    /// What a fill of one key did, for each declared key column.
    keys: Vec<MeasuredKey>,
}

/// What a fill of one key of a declared key column did for one lent file.
struct MeasuredKey {
    /// The column.
    column: Column,
    /// What the fill answered.
    answer: FactAnswer,
    /// Whether it wrote exactly the rows of the whole fill that hold the key.
    wrote: KeyWrote,
    /// How many rows it wrote.
    rows: Count,
}

/// Whether a fill by key wrote exactly the rows of the whole fill that hold the key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyWrote {
    /// The key's rows and no other.
    Alone,
    /// Other rows, more rows or fewer.
    Other,
}

/// The rows of filled columns, row by row, sorted: what two fills are compared by.
fn rows_of(columns: &FilledColumns, keep: impl Fn(&[CellWord]) -> bool) -> Vec<Vec<CellWord>> {
    let count = columns.0.first().map_or(0, |column| column.0.len());
    let mut rows: Vec<Vec<CellWord>> = (0..count)
        .map(|row| {
            columns
                .0
                .iter()
                .filter_map(|column| column.0.get(row).copied())
                .collect()
        })
        .filter(|row: &Vec<CellWord>| keep(row))
        .collect();
    rows.sort_unstable();
    rows
}

/// The cell of the column that the fewest rows hold (the smallest such cell): the key a growth run asks.
fn rarest(cells: &[CellWord]) -> Option<CellWord> {
    let mut sorted = cells.to_vec();
    sorted.sort_unstable();
    sorted
        .chunk_by(|left, right| left == right)
        .min_by_key(|run| run.len())
        .and_then(|run| run.first().copied())
}

/// Fills every filled relation of the catalog for the lent file, whole and then by one key of each declared key column.
fn measured(file: &mut FileFacts<'_>, names: &FixedNames) -> Vec<Measured> {
    let mut found = Vec::new();
    for relation in all().iter().copied() {
        if fact_form(relation) == FactForm::NodeTable {
            continue;
        }
        let mut words = file_words(&base_words(names, &[]));
        let mut whole = FilledColumns::default();
        let rows = match file
            .facts
            .fill(relation, Wanted::Whole, &mut words, &mut whole)
        {
            FactAnswer::Refused(_) => continue,
            FactAnswer::Partial(_) => None,
            FactAnswer::Answered(_) => {
                Some(Count(whole.0.first().map_or(0, |column| column.0.len())))
            }
        };
        let mut keys = Vec::new();
        for column in key_columns(relation) {
            let place = columns(relation).iter().position(|each| each == column);
            let cells = place.and_then(|place| whole.0.get(place));
            let Some((place, key)) = place.zip(cells.and_then(|cells| rarest(&cells.0))) else {
                continue;
            };
            let mut by_key = FilledColumns::default();
            let answer =
                file.facts
                    .fill(relation, Wanted::Key(*column, key), &mut words, &mut by_key);
            let written = rows_of(&by_key, |_| true);
            let wrote = if written == rows_of(&whole, |row| row.get(place) == Some(&key)) {
                KeyWrote::Alone
            } else {
                KeyWrote::Other
            };
            keys.push(MeasuredKey {
                column: *column,
                answer,
                wrote,
                rows: Count(written.len()),
            });
        }
        found.push(Measured {
            relation,
            rows,
            keys,
        });
    }
    found
}

/// Holds a language's fillers to what the catalog declares of each relation they fill: the growth harness a language's contract
/// test runs. `lend` walks the language's generated source of about the elements asked and hands the lent file to the visit; it
/// is asked for `elements` and for four times as many. For each relation the language answers, the rows a whole fill writes
/// must grow as the relation's `size` says (no more than one row for `PerFile`; no faster than nine eighths of the file's own
/// growth for `PerElement`, which is 4.5 times at four times the elements; `PerPair` is not held), and for each of its declared
/// `keys` a fill of one key must answer `Asked`, write exactly the rows of the whole fill that hold the key, and write no more
/// for the larger file. `names` are the language's fixed names (`Fillers::vocabulary`). A relation with no row for the larger
/// file is a fault too: a generator that makes no row of it proves nothing.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::FixedNames;
/// use ryo_language::domain::fillers::fixtures::growth::grown;
/// use ryo_relations::domain::relations::types::Count;
///
/// let report = grown(&FixedNames::default(), Count(8), &mut |_elements, _visit| {});
/// assert!(!report.faults.is_empty());
/// ```
pub fn grown(
    names: &FixedNames,
    elements: Count,
    lend: &mut dyn FnMut(Count, &mut dyn FnMut(FileFacts<'_>)),
) -> GrowthReport {
    let mut sides: Vec<(Count, Vec<Measured>)> = Vec::new();
    for asked in [elements, Count(elements.0.saturating_mul(4))] {
        lend(asked, &mut |mut file| {
            sides.push((element_count(file.table), measured(&mut file, names)));
        });
    }
    let mut report = GrowthReport {
        small: Count(0),
        large: Count(0),
        relations: Vec::new(),
        keys: Vec::new(),
        faults: Vec::new(),
    };
    let [(small, before), (large, after)] = sides.as_slice() else {
        report.faults.push(GrowthFault::Sources);
        return report;
    };
    (report.small, report.large) = (*small, *large);
    if large.0 < small.0.saturating_mul(3) {
        report.faults.push(GrowthFault::Sources);
    }
    if after.is_empty() {
        report.faults.push(GrowthFault::NothingAnswered);
    }
    for larger in after {
        let relation = larger.relation;
        let smaller = before.iter().find(|each| each.relation == relation);
        let narrow = smaller.and_then(|each| each.rows);
        let (Some(size), Some(wide), Some(narrow)) = (relation_size(relation), larger.rows, narrow)
        else {
            report.faults.push(GrowthFault::Unproven(relation));
            continue;
        };
        report.relations.push(Growth {
            relation,
            size,
            small: narrow,
            large: wide,
        });
        if wide.0 == 0 {
            report.faults.push(GrowthFault::Unproven(relation));
        }
        let grew = match size {
            RelationSize::PerFile => narrow.0 > 1 || wide.0 > 1,
            // rows(large) / rows(small) <= 9/8 * elements(large) / elements(small), in whole numbers.
            RelationSize::PerElement => {
                wide.0.saturating_mul(small.0).saturating_mul(8)
                    > narrow.0.saturating_mul(large.0).saturating_mul(9)
            }
            RelationSize::PerPair => false,
        };
        if grew {
            report.faults.push(GrowthFault::Grew(relation));
        }
        for wide_key in &larger.keys {
            let column = wide_key.column;
            let narrow_key =
                smaller.and_then(|each| each.keys.iter().find(|other| other.column == column));
            let mut faults = Vec::new();
            for each in std::iter::once(wide_key).chain(narrow_key) {
                if each.answer != FactAnswer::Answered(FillExtent::Asked) {
                    faults.push(GrowthFault::KeyNotAsked(column));
                } else if each.wrote == KeyWrote::Other {
                    faults.push(GrowthFault::KeyRows(column));
                }
            }
            if narrow_key.is_some_and(|before| wide_key.rows.0 > before.rows.0) {
                faults.push(GrowthFault::KeyGrew(column));
            }
            faults.dedup();
            report.faults.extend(faults);
            report.keys.push(KeyGrowth {
                column,
                small: narrow_key.map_or(Count(0), |before| before.rows),
                large: wide_key.rows,
            });
        }
    }
    let lost = before
        .iter()
        .filter(|each| after.iter().all(|larger| larger.relation != each.relation));
    report
        .faults
        .extend(lost.map(|each| GrowthFault::Unproven(each.relation)));
    report
}
