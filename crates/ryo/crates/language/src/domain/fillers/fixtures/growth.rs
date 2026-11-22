//! The growth harness of the `fillers` concept, which every language's contract test runs: it holds a language's fillers to what
//! the catalog declares of each relation they fill: a relation's rows grow with the file as its `size` says. Every relation
//! is asked in the file's one request.

use crate::domain::facts::service::{base_words, file_words};
use crate::domain::fact_tables::reading::service::every_row;
use crate::domain::fact_tables::service::{answer_of, begun, request_of, table_of};
use crate::domain::fact_tables::types::FactSet;
use crate::domain::facts::types::{BaseWords, FactAnswer, FileFacts};
use crate::domain::fixed_names::errors::FixedNamesError;
use crate::domain::fixed_names::service::checked;
use crate::domain::fixed_names::types::FixedNames;
use crate::domain::node_table::service::element_count;
use ryo_relations::domain::catalog::service::{fact_form, relation_size};
use ryo_relations::domain::relations::service::all;
use ryo_relations::domain::relations::types::{Count, FactForm, Relation, RelationSize};

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


/// What a growth run found that a relation's catalog item does not declare, or could not judge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrowthFault {
    /// The language's fixed names cannot be base words of a run: one is stated twice or is the empty text. No file is asked for.
    FixedNames(FixedNamesError),
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
    /// What does not hold; empty when every filler is as its relation's catalog item declares.
    pub faults: Vec<GrowthFault>,
}

/// One relation of one lent file: the rows its filler wrote.
struct Measured {
    /// The relation.
    relation: Relation,
    /// The rows written; none when it was answered in part.
    rows: Option<Count>,
}

/// Asks every filled relation of the catalog for the lent file, in one request.
fn measured(file: &mut FileFacts<'_>, base: &BaseWords) -> Vec<Measured> {
    let filled: Vec<Relation> = all()
        .iter()
        .copied()
        .filter(|relation| fact_form(*relation) != FactForm::NodeTable)
        .collect();
    let request = request_of(&filled);
    let mut words = file_words(base);
    let mut set = FactSet::default();
    begun(&mut set, &request);
    file.facts.facts(&request, &mut words, &mut set);
    filled
        .into_iter()
        .filter_map(|relation| {
            let rows = match answer_of(&set, relation) {
                FactAnswer::Refused(_) => return None,
                FactAnswer::Partial(_) => None,
                FactAnswer::Answered => Some(Count(
                    usize::try_from(every_row(table_of(&set, relation)).end.0).unwrap_or(usize::MAX),
                )),
            };
            Some(Measured { relation, rows })
        })
        .collect()
}

/// Holds a language's fillers to what the catalog declares of each relation they fill: the growth harness a language's contract
/// test runs. `lend` walks the language's generated source of about the elements asked and hands the lent file to the visit; it
/// is asked for `elements` and for four times as many. For each relation the language answers, the rows a whole fill writes
/// must grow as the relation's `size` says (no more than one row for `PerFile`; no faster than nine eighths of the file's own
/// growth for `PerElement`, which is 4.5 times at four times the elements; `PerPair` is not held). `names` are the language's fixed names (`Fillers::vocabulary`): names that cannot be base words of a
/// run are the one fault of the report, and no file is asked for. A relation with no row for the larger file is a fault too: a
/// generator that makes no row of it proves nothing.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fixed_names::errors::FixedNamesError;
/// use ryo_language::domain::fixed_names::types::{FixedNames, FixedWord};
/// use ryo_language::domain::fillers::fixtures::growth::{GrowthFault, grown};
/// use ryo_relations::domain::relations::types::Count;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let report = grown(&FixedNames::default(), Count(8), &mut |_elements, _visit| {});
/// assert!(!report.faults.is_empty());
///
/// let twice = FixedNames(vec![NameText("next-token"), NameText("next-token")]);
/// let report = grown(&twice, Count(8), &mut |_elements, _visit| unreachable!("no file is asked for"));
/// let refused = FixedNamesError::Twice { first: FixedWord(0), again: FixedWord(1) };
/// assert_eq!(report.faults, vec![GrowthFault::FixedNames(refused)]);
/// ```
pub fn grown(
    names: &FixedNames,
    elements: Count,
    lend: &mut dyn FnMut(Count, &mut dyn FnMut(FileFacts<'_>)),
) -> GrowthReport {
    let mut report = GrowthReport {
        small: Count(0),
        large: Count(0),
        relations: Vec::new(),
        faults: Vec::new(),
    };
    let base = match checked(&names.0) {
        Ok(fixed) => base_words(fixed, &[]),
        Err(refused) => {
            report.faults.push(GrowthFault::FixedNames(refused));
            return report;
        }
    };
    let mut sides: Vec<(Count, Vec<Measured>)> = Vec::new();
    for asked in [elements, Count(elements.0.saturating_mul(4))] {
        lend(asked, &mut |mut file| {
            sides.push((element_count(file.table), measured(&mut file, &base)));
        });
    }
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
    }
    let lost = before
        .iter()
        .filter(|each| after.iter().all(|larger| larger.relation != each.relation));
    report
        .faults
        .extend(lost.map(|each| GrowthFault::Unproven(each.relation)));
    report
}
