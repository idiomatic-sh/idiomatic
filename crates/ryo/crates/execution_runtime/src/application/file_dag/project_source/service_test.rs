//! The project source: it serves exactly the relations the catalog declares `scope Project`, reads each once for the phase and
//! keeps a repeated row once.
use super::service::{opened, read, table_of};
use super::types::ProjectWritten;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::catalog::service::scope_of;
use ryo_relations::domain::catalog::types::RowScope;
use ryo_relations::domain::relations::service::all;
use ryo_relations::domain::relations::types::Relation;
use std::sync::atomic::Ordering;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The relations the source serves, against the catalog's.
    Serves,
    /// A project relation whose one row the project wrote twice, read.
    RepeatedRow,
    /// A project relation read by two reads of the phase.
    TwoReads,
}

#[derive(Debug, PartialEq, Eq)]
enum Observed {
    /// Whether the source serves exactly the catalog's project relations, in catalog order, and how many.
    Serves(bool, usize),
    /// How many rows the phase holds of the relation.
    Rows(usize),
    /// How many times the project's worker was asked for the relation.
    Reads(usize),
}

type Input = Scenario;
type Value = Observed;
type Failure = crate::outbound::facts::errors::FactsError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_project_source_serves_every_project_scope_relation"),
        word: Word::Succeeds,
        input: || Scenario::Serves,
        expect: Expect::Value(Observed::Serves(true, 8)),
    },
    Case {
        name: CaseName("project_rows_are_carried_and_kept_once"),
        word: Word::Succeeds,
        input: || Scenario::RepeatedRow,
        expect: Expect::Value(Observed::Rows(1)),
    },
    Case {
        name: CaseName("each_relation_is_read_once_per_phase"),
        word: Word::Succeeds,
        input: || Scenario::TwoReads,
        expect: Expect::Value(Observed::Reads(1)),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let relation = Relation::ModuleIdentity;
    let mut facts = ProjectFacts::new(relation);
    if matches!(scenario, Scenario::RepeatedRow) {
        for column in &mut facts.columns {
            let first = column.0.clone();
            column.0.extend(first);
        }
    }
    let project_symbols = Symbols::default();
    let mut symbols = Symbols::default();
    let source = opened(
        ProjectWritten {
            rows: &facts,
            symbols: &project_symbols,
        },
        vec![relation],
    );
    match scenario {
        Scenario::Serves => {
            let catalog: Vec<Relation> = all()
                .iter()
                .copied()
                .filter(|each| scope_of(*each) == RowScope::Project)
                .collect();
            Ok(Observed::Serves(source.serves == catalog, source.serves.len()))
        }
        Scenario::RepeatedRow => {
            read(&source, &mut symbols);
            Ok(Observed::Rows(table_of(&source, relation)?.count.0))
        }
        Scenario::TwoReads => {
            read(&source, &mut symbols);
            read(&source, &mut symbols);
            Ok(Observed::Reads(facts.reads.load(Ordering::Relaxed)))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
