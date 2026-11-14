//! The fact lookup of one file, through its own methods only: exactly the demanded relations read, each once by the source that
//! serves it, a served relation refused until ensured, outside what was ensured or after release, core's own read in place, a
//! refusal stopping the file, and the partiality `partial_of` gives.

use super::service::FactLookup;
use crate::application::file_dag::file_pass::service::rules::service::opened;
use super::types::FileWants;
use crate::application::file_dag::program::file_session::types::FileStop;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::fixtures::fake::FactsFake;
use crate::outbound::facts::fixtures::sources::SourceFake;
use crate::outbound::facts::types::fact_sources::value::{FactSource as _, FactSources};
use ryo_execution_domain::domain::coverage::service::partial_of;
use ryo_execution_domain::domain::coverage::types::Recovery;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Missing, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::stored_facts::types::{PartialAt, PartialCause, PartialScope, ReadInPart};

/// A scenario over one file's lookup.
type Input = fn() -> Result<Value, Failure>;
/// What the scenario counted, or 1 for a check that held.
type Value = usize;
type Failure = FactsError;

/// What a scenario reads its file for: `function`, then `comment`, both admitted, core saying nothing, the file whole.
const WHOLE: FileWants<'static> = FileWants {
    wanted: &[Relation::Function, Relation::Comment],
    active: &[Relation::Function, Relation::Comment],
    said: &[],
    recovery: Recovery::Whole,
};

/// The length of the first column of `relation`'s rows.
fn rows_of(lookup: &FactLookup<'_, '_>, relation: Relation) -> Result<Value, Failure> {
    Ok(lookup
        .columns(relation)?
        .columns
        .first()
        .map_or(0, |column| column.0.len()))
}

/// Whether the rows of an ensured relation are the numbers the source writes when it is read on its own: 1.
fn rows_after_ensure_equal_what_the_source_writes() -> Result<Value, Failure> {
    let fake = FactsFake::default();
    let mut direct = fake.open_worker()?;
    let mut alone = SourceFake::serving(&[Relation::Function]);
    let _said = alone.read(
        &mut Symbols::default(),
        &[Relation::Function],
        &mut direct.columns_mut()?,
    )?;
    let expected: Vec<Vec<u64>> = direct
        .rows(Relation::Function)?
        .columns
        .iter()
        .map(|column| column.0.clone())
        .collect();
    let mut worker = fake.open_worker()?;
    let mut source = SourceFake::serving(&[Relation::Function]);
    let mut lookup = FactLookup::new(
        FactSources::read(&mut source, Vec::new())?,
        worker.as_mut(),
        WHOLE,
    );
    lookup.ensure(&mut Symbols::default(), &[Relation::Function])?;
    let read: Vec<Vec<u64>> = lookup
        .columns(Relation::Function)?
        .columns
        .iter()
        .map(|column| column.0.clone())
        .collect();
    Ok(usize::from(read == expected))
}

/// How many times a source is read when its relation is ensured three times in one file.
fn each_relation_is_read_once_per_file() -> Result<Value, Failure> {
    let mut worker = FactsFake::default().open_worker()?;
    let mut source = SourceFake::serving(&[Relation::Function]);
    {
        let mut lookup = FactLookup::new(
            FactSources::read(&mut source, Vec::new())?,
            worker.as_mut(),
            WHOLE,
        );
        for _ in 0..3 {
            lookup.ensure(&mut Symbols::default(), &[Relation::Function])?;
        }
    }
    Ok(source.reads.len())
}

/// With a tree source of `function` and a further source of `comment`: whether each was asked for its own relation once, and the
/// further source's rows are readable through the lookup.
fn each_relation_comes_from_the_source_that_serves_it() -> Result<Value, Failure> {
    let mut worker = FactsFake::default().open_worker()?;
    let mut tree = SourceFake::serving(&[Relation::Function]);
    let mut further = SourceFake::serving(&[Relation::Comment]);
    let comments = {
        let mut lookup = FactLookup::new(
            FactSources::read(&mut tree, vec![&mut further])?,
            worker.as_mut(),
            WHOLE,
        );
        lookup.ensure(&mut Symbols::default(), WHOLE.wanted)?;
        rows_of(&lookup, Relation::Comment)?
    };
    Ok(usize::from(
        tree.reads == [vec![Relation::Function]]
            && further.reads == [vec![Relation::Comment]]
            && comments == 1,
    ))
}

/// Whether the lookup's partiality is `partial_of` the source's said parts, then core's, under a recovery.
fn partials_follow_partial_of() -> Result<Value, Failure> {
    let part = |relation, cause: &str| ReadInPart {
        relation,
        cause: PartialCause(cause.into()),
        lacking: Missing::Any,
        at: PartialScope::OneFile(PartialAt(0)),
    };
    let reader = vec![part(Relation::Function, "the reader")];
    let core = vec![part(Relation::FilesystemExecutable, "the engine")];
    let mut worker = FactsFake::default().open_worker()?;
    let mut source = SourceFake::serving(&[Relation::Function]);
    source.said.clone_from(&reader);
    let wants = FileWants {
        said: &core,
        recovery: Recovery::SyntaxErrors,
        ..WHOLE
    };
    let mut lookup = FactLookup::new(
        FactSources::read(&mut source, Vec::new())?,
        worker.as_mut(),
        wants,
    );
    lookup.ensure(&mut Symbols::default(), WHOLE.wanted)?;
    let said: Vec<ReadInPart> = reader.iter().chain(&core).cloned().collect();
    Ok(usize::from(
        lookup.partials() == partial_of(&said, Recovery::SyntaxErrors, WHOLE.active).as_slice(),
    ))
}

/// How many rows a source that refused left in the worker once the file is opened: none.
fn a_refused_file_leaves_no_row() -> Result<Value, Failure> {
    let mut worker = FactsFake::default().open_worker()?;
    let mut source = SourceFake::serving(&[Relation::Function]);
    source.refuses = Some(Relation::Function);
    let refused = opened(
        FactSources::read(&mut source, Vec::new())?,
        worker.as_mut(),
        WHOLE,
        &mut Symbols::default(),
    )
    .err();
    if refused != Some(FileStop::Reader) {
        return Ok(usize::MAX);
    }
    Ok(worker
        .rows(Relation::Function)?
        .columns
        .first()
        .map_or(0, |column| column.0.len()))
}

/// What reading a relation before anything is ensured answers.
fn a_relation_not_ensured_is_refused() -> Result<Value, Failure> {
    let mut worker = FactsFake::default().open_worker()?;
    let mut source = SourceFake::serving(&[Relation::Function]);
    let lookup = FactLookup::new(
        FactSources::read(&mut source, Vec::new())?,
        worker.as_mut(),
        WHOLE,
    );
    rows_of(&lookup, Relation::Function)
}

/// What ensuring a relation whose rows the source refuses answers.
fn a_refused_relation_stops_with_refused() -> Result<Value, Failure> {
    let mut worker = FactsFake::default().open_worker()?;
    let mut source = SourceFake::serving(&[Relation::Function]);
    source.refuses = Some(Relation::Function);
    let mut lookup = FactLookup::new(
        FactSources::read(&mut source, Vec::new())?,
        worker.as_mut(),
        WHOLE,
    );
    lookup.ensure(&mut Symbols::default(), &[Relation::Function])?;
    rows_of(&lookup, Relation::Function)
}

/// What reading an ensured relation answers after the file is released.
fn release_leaves_nothing_readable() -> Result<Value, Failure> {
    let mut worker = FactsFake::default().open_worker()?;
    let mut source = SourceFake::serving(&[Relation::Function]);
    let mut lookup = FactLookup::new(
        FactSources::read(&mut source, Vec::new())?,
        worker.as_mut(),
        WHOLE,
    );
    lookup.ensure(&mut Symbols::default(), &[Relation::Function])?;
    lookup.release()?;
    rows_of(&lookup, Relation::Function)
}

/// What each source is asked for when only `function` is demanded of a file whose further source serves `comment`: the tree
/// source `function`, the further source nothing.
fn only_the_demanded_relations_are_read() -> Result<Value, Failure> {
    let mut worker = FactsFake::default().open_worker()?;
    let mut tree = SourceFake::serving(&[Relation::Function]);
    let mut further = SourceFake::serving(&[Relation::Comment]);
    {
        let mut lookup = FactLookup::new(
            FactSources::read(&mut tree, vec![&mut further])?,
            worker.as_mut(),
            WHOLE,
        );
        lookup.ensure(&mut Symbols::default(), &[Relation::Function])?;
    }
    Ok(usize::from(
        tree.reads == [vec![Relation::Function]] && further.reads == [Vec::<Relation>::new()],
    ))
}

/// What reading `comment` answers once only `function` was ensured.
fn a_relation_outside_what_was_ensured_is_refused() -> Result<Value, Failure> {
    let mut worker = FactsFake::default().open_worker()?;
    let mut tree = SourceFake::serving(&[Relation::Function]);
    let mut further = SourceFake::serving(&[Relation::Comment]);
    let mut lookup = FactLookup::new(
        FactSources::read(&mut tree, vec![&mut further])?,
        worker.as_mut(),
        WHOLE,
    );
    lookup.ensure(&mut Symbols::default(), &[Relation::Function])?;
    rows_of(&lookup, Relation::Comment)
}

/// What a second `ensure` asking for `comment` answers once the first read only `function`: the file is never read twice.
fn a_later_ensure_beyond_the_first_is_refused() -> Result<Value, Failure> {
    let mut worker = FactsFake::default().open_worker()?;
    let mut tree = SourceFake::serving(&[Relation::Function]);
    let mut further = SourceFake::serving(&[Relation::Comment]);
    let mut lookup = FactLookup::new(
        FactSources::read(&mut tree, vec![&mut further])?,
        worker.as_mut(),
        WHOLE,
    );
    lookup.ensure(&mut Symbols::default(), &[Relation::Function])?;
    lookup.ensure(&mut Symbols::default(), &[Relation::Comment])?;
    Ok(1)
}

/// How many relations the source admits when `function` and `comment` are active but only `function` is demanded: `function`.
fn only_demanded_active_relations_are_admitted() -> Result<Value, Failure> {
    let mut worker = FactsFake::default().open_worker()?;
    let mut source = SourceFake::serving(&[Relation::Function, Relation::Comment]);
    {
        let mut lookup = FactLookup::new(
            FactSources::read(&mut source, Vec::new())?,
            worker.as_mut(),
            WHOLE,
        );
        lookup.ensure(&mut Symbols::default(), &[Relation::Function])?;
    }
    Ok(source.admitted.len())
}

/// How many rows `comment`, which no source of the file serves and core wrote none of, reads from the worker once only `function`
/// was ensured: none, read rather than refused.
fn a_relation_no_source_serves_reads_the_worker() -> Result<Value, Failure> {
    let mut worker = FactsFake::default().open_worker()?;
    let mut source = SourceFake::serving(&[Relation::Function]);
    let mut lookup = FactLookup::new(
        FactSources::read(&mut source, Vec::new())?,
        worker.as_mut(),
        WHOLE,
    );
    lookup.ensure(&mut Symbols::default(), &[Relation::Function])?;
    rows_of(&lookup, Relation::Comment)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rows_after_ensure_equal_what_the_source_writes"),
        word: Word::Succeeds,
        input: || rows_after_ensure_equal_what_the_source_writes,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("each_relation_is_read_once_per_file"),
        word: Word::Succeeds,
        input: || each_relation_is_read_once_per_file,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("each_relation_comes_from_the_source_that_serves_it"),
        word: Word::Succeeds,
        input: || each_relation_comes_from_the_source_that_serves_it,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("partials_follow_partial_of"),
        word: Word::Succeeds,
        input: || partials_follow_partial_of,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_refused_file_leaves_no_row"),
        word: Word::Succeeds,
        input: || a_refused_file_leaves_no_row,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("only_the_demanded_relations_are_read"),
        word: Word::Succeeds,
        input: || only_the_demanded_relations_are_read,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("only_demanded_active_relations_are_admitted"),
        word: Word::Succeeds,
        input: || only_demanded_active_relations_are_admitted,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_relation_no_source_serves_reads_the_worker"),
        word: Word::Succeeds,
        input: || a_relation_no_source_serves_reads_the_worker,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_relation_not_ensured_is_refused"),
        word: Word::Refuses,
        input: || a_relation_not_ensured_is_refused,
        expect: Expect::Error(FactsError::NotEnsured {
            relation: Relation::Function,
        }),
    },
    Case {
        name: CaseName("a_relation_outside_what_was_ensured_is_refused"),
        word: Word::Refuses,
        input: || a_relation_outside_what_was_ensured_is_refused,
        expect: Expect::Error(FactsError::NotEnsured {
            relation: Relation::Comment,
        }),
    },
    Case {
        name: CaseName("a_later_ensure_beyond_the_first_is_refused"),
        word: Word::Refuses,
        input: || a_later_ensure_beyond_the_first_is_refused,
        expect: Expect::Error(FactsError::NotEnsured {
            relation: Relation::Comment,
        }),
    },
    Case {
        name: CaseName("a_refused_relation_stops_with_refused"),
        word: Word::Refuses,
        input: || a_refused_relation_stops_with_refused,
        expect: Expect::Error(FactsError::Refused),
    },
    Case {
        name: CaseName("release_leaves_nothing_readable"),
        word: Word::Refuses,
        input: || release_leaves_nothing_readable,
        expect: Expect::Error(FactsError::NotEnsured {
            relation: Relation::Function,
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
