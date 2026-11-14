//! Required publication aliases share normalization and release its backing at the last consumer.
use super::super::super::super::service::RelationOwner;
use super::super::fixtures::{columns, read, row};
use crate::application::file_dag::relations::types::{CheckCount, Liveness, Sameness};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::{
    operating::errors::OperatingError,
    table_model::types::Rows,
};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, ColumnsView, Integer, Relation, Value as CellValue},
};
use std::borrow::Cow;
use std::sync::Arc;

/// The publication two consumers hold.
#[derive(Clone, Copy, Debug)]
enum Published {
    /// A source of `SourceLocation` rows whose second column is one row short, so its stored
    /// view is not readable as is and the owner normalizes it on demand.
    LazySource,
    /// A computed table of the numbers 41 and 43.
    Computed([i64; 2]),
}

/// The owner's publication charge: required, optional and unique bytes.
type Charge = (usize, usize, usize);

/// What the normalized source's consumers observed.
#[derive(Debug, PartialEq)]
struct Normalized {
    checks_before_demand: CheckCount,
    normalized_before_demand: bool,
    optional_before_demand: usize,
    reads_the_first_source_row_alone: bool,
    charged_required: bool,
    charged_optional: usize,
    required_is_unique: bool,
    alive_after_first_consumer: bool,
    alias_reads_same_table: bool,
    checks_after_alias: CheckCount,
    alias_charged_again: bool,
    alive_after_last_consumer: bool,
    charge_after_last_consumer: Charge,
}

/// What the computed publication's consumers observed.
#[derive(Debug, PartialEq)]
struct Kept {
    alias: Sameness,
    charged_required: bool,
    charged_optional: usize,
    required_is_unique: bool,
    alive_after_first_consumer: bool,
    charge_kept: bool,
    rows: Cow<'static, [Cow<'static, [CellValue]>]>,
    charge_kept_while_read: bool,
    generation: Sameness,
    live: Liveness,
    alive_after_last_consumer: bool,
    charge_after_last_consumer: Charge,
    generation_after_release: Sameness,
    live_after_release: Liveness,
}

/// What the consumers observed.
#[derive(Debug, PartialEq)]
enum Observed {
    Normalized(Normalized),
    Kept(Kept),
}

type Input = Published;
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("lazy_source_normalization_is_shared_and_lives_until_the_last_consumer"),
        word: Word::Succeeds,
        input: || Published::LazySource,
        expect: Expect::Value(Observed::Normalized(Normalized {
            checks_before_demand: CheckCount(0),
            normalized_before_demand: false,
            optional_before_demand: 0,
            reads_the_first_source_row_alone: true,
            charged_required: true,
            charged_optional: 0,
            required_is_unique: true,
            alive_after_first_consumer: true,
            alias_reads_same_table: true,
            checks_after_alias: CheckCount(1),
            alias_charged_again: false,
            alive_after_last_consumer: false,
            charge_after_last_consumer: (0, 0, 0),
        })),
    },
    Case {
        name: CaseName(
            "computed_publication_clones_keep_required_backing_until_last_handle_in_live_generation",
        ),
        word: Word::Succeeds,
        input: || Published::Computed([41, 43]),
        expect: Expect::Value(Observed::Kept(Kept {
            alias: Sameness::Same,
            charged_required: true,
            charged_optional: 0,
            required_is_unique: true,
            alive_after_first_consumer: true,
            charge_kept: true,
            rows: Cow::Borrowed(&[
                Cow::Borrowed(&[CellValue::Number(Integer(41))]),
                Cow::Borrowed(&[CellValue::Number(Integer(43))]),
            ]),
            charge_kept_while_read: true,
            generation: Sameness::Same,
            live: Liveness::Live,
            alive_after_last_consumer: false,
            charge_after_last_consumer: (0, 0, 0),
            generation_after_release: Sameness::Same,
            live_after_release: Liveness::Live,
        })),
    },
];

/// Publishes the input, has a consumer and its alias read it in turn, and drops them.
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(match input {
        Published::LazySource => Observed::Normalized(normalized()?),
        Published::Computed(numbers) => Observed::Kept(kept(numbers)?),
    })
}

fn normalized() -> Result<Normalized, OperatingError> {
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    let relation = Relation::SourceLocation;
    let schema = kinds(relation);
    let mut physical = columns(schema);
    physical[1].0.truncate(1);
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let publication = owner.source(relation, &Rows::Stored(source))?;
    let consumer = publication.clone();
    let weak = Arc::downgrade(publication.record());
    let checks_before_demand = owner.checks.get();
    let normalized_before_demand = publication.as_ref().normalized.get().is_some();
    let optional_before_demand = owner.derived.publication_ownership_bytes().1;
    let Rows::Computed(table) = owner.normalize(&publication, Rows::Stored(source))? else {
        panic!("readable physical fallback must normalize")
    };
    let normalized_address = std::ptr::from_ref(table);
    let reads_the_first_source_row_alone = read(&Rows::Computed(table)) == [row(schema, 0)];
    let charged = owner.derived.publication_ownership_bytes();
    drop(publication);
    let alive_after_first_consumer = weak.upgrade().is_some();
    let Rows::Computed(table) = owner.normalize(&consumer, Rows::Stored(source))? else {
        panic!("same demanded fallback")
    };
    let alias_reads_same_table = normalized_address == std::ptr::from_ref(table);
    let checks_after_alias = owner.checks.get();
    let alias_charged_again = owner.derived.publication_ownership_bytes() != charged;
    drop(consumer);
    Ok(Normalized {
        checks_before_demand,
        normalized_before_demand,
        optional_before_demand,
        reads_the_first_source_row_alone,
        charged_required: charged.0 > 0,
        charged_optional: charged.1,
        required_is_unique: charged.0 == charged.2,
        alive_after_first_consumer,
        alias_reads_same_table,
        checks_after_alias,
        alias_charged_again,
        alive_after_last_consumer: weak.upgrade().is_some(),
        charge_after_last_consumer: owner.derived.publication_ownership_bytes(),
    })
}

fn kept(numbers: [i64; 2]) -> Result<Kept, OperatingError> {
    use ryo_values::domain::rows::types::{RowCount, Table};
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        crate::application::file_dag::fixtures::program_derived_port(),
    );
    let generation = owner.current_generation();
    let publication = owner.computed(Table {
        kinds: vec![CellKind::Number],
        cells: numbers
            .iter()
            .map(|&number| CellValue::Number(Integer(number)))
            .collect(),
        count: RowCount(2),
    })?;
    let record = Arc::downgrade(publication.record());
    let alias = publication.clone();
    let sameness = alias.same_record(&publication);
    let charged = owner.derived.publication_ownership_bytes();
    drop(publication);
    let alive_after_first_consumer = record.upgrade().is_some();
    let charge_kept = owner.derived.publication_ownership_bytes() == charged;
    let (rows, charge_kept_while_read) = {
        let rows = owner.resolve(alias.as_ref(), &|_| Err(OperatingError::NotRead))?;
        (
            Cow::Owned(read(&rows).into_iter().map(Cow::Owned).collect()),
            owner.derived.publication_ownership_bytes() == charged,
        )
    };
    let generation_before = owner.matches_generation(&generation);
    let live = owner.current_generation().is_live();
    drop(alias);
    Ok(Kept {
        alias: sameness,
        charged_required: charged.0 > 0,
        charged_optional: charged.1,
        required_is_unique: charged.0 == charged.2,
        alive_after_first_consumer,
        charge_kept,
        rows,
        charge_kept_while_read,
        generation: generation_before,
        live,
        alive_after_last_consumer: record.upgrade().is_some(),
        charge_after_last_consumer: owner.derived.publication_ownership_bytes(),
        generation_after_release: owner.matches_generation(&generation),
        live_after_release: owner.current_generation().is_live(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
