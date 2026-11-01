//! The `recording` service's cases, on values alone.

use ryo_language::domain::elements::service::notes_mut;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::relations::types::{
    CellKind, FileId, Integer, Relation, Value as Cell, Words, WorkerNumber,
};
use ryo_values::domain::descriptions::types::RelationName;
use ryo_values::domain::files::types::{CorpusEntry, FileNumber, FileSize, Nanoseconds};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as PlanInput, RepeatedRows, StepId,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};
use ryo_values::domain::records::types::{FileOutcome, Kept, Ran, Record, Unreadable};
use ryo_values::domain::stored_facts::types::{
    ArenaBase, Bucket, Digest, KeptWords, LanguageNumber,
};

use super::service::{cover, finish_file, frontier_flat, made_ran, moved_words, replay, stored_of};
use super::types::{BucketWords, Flat};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::coverage::types::{Lacking, LackingKind, Partial, RuleIndex};
use crate::domain::lowering::types::Counting;
use crate::domain::operator::types::TreeSteps;
use crate::domain::placement::types::{Mask, PlacedScopes, ScopePlacement};
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::Table;
use crate::domain::table_words::service::{encode_table, pack};
use crate::domain::table_words::types::WordSlice;
use crate::domain::trust::types::{Needs, RelationRead};
use crate::domain::visit::types::{Noting, RoleRead};
use crate::domain::workers::service::tables_for;
use crate::domain::workers::types::{FileOrder, Tables, VisitSetting};
use crate::outbound::matching::fixtures::fake::MatchingFake;
use crate::outbound::matching::interface::MatchingPort as _;
use ryo_values::domain::runs::types::MemoryBytes;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::PartialCause;

/// What the last run kept, for a run that uses nothing of it.
static NOTHING_STORED: std::sync::LazyLock<ryo_values::domain::stores::types::Stored> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::Stored::default);

/// What the files of the last run handed, for a run that uses nothing of it.
static NOTHING_HANDED: std::sync::LazyLock<ryo_values::domain::stores::types::StoredHanded> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::StoredHanded::default);

/// No bucket of what the files handed, for a run that reads none.
static NOTHING_WANTED: ryo_values::domain::stores::types::BucketsWanted =
    ryo_values::domain::stores::types::BucketsWanted::These(Vec::new());

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Writes a relation read in part as the interface of a file takes it.
    PartialsOf,
    /// Moves the words a file captured to the numbers every run gives them.
    MovedWords,
    /// Flattens what a file handed to one rule, among what it handed to two.
    FrontierFlat,
    /// Makes the record of what the rules gave of a file.
    MadeRan,
    /// Covers a file read in part that a rule governs and trusts.
    Cover,
    /// Uses again the record of a file the rules were evaluated on.
    ReplayRead,
    /// Ends the visit of a file whose record the run keeps and which a language wrote notes of.
    FinishKept,
}

/// What a row of the cases does.
type Input = Scenario;
/// A count, or several written one after the other as digits.
type Value = u64;
type Failure = ();

fn count(number: usize) -> u64 {
    u64::try_from(number).unwrap_or(u64::MAX)
}

fn tables() -> Tables {
    tables_for(
        WorkerNumber(2),
        &[],
        TreeSteps::Lowered,
        Counting::DoesNotCount,
    )
}

fn placed() -> Result<PlacedScopes, Failure> {
    Ok(PlacedScopes {
        placement: ScopePlacement::default(),
        matcher: MatchingFake.compile(&[]).map_err(drop)?,
    })
}

fn setting(placed: &PlacedScopes) -> VisitSetting<'_> {
    VisitSetting {
        unknown: &[],
        plans: &[],
        bindings: &[],
        wanted: &[],
        paths: &[],
        first_bucket: &[],
        ids: &[],
        placed,
        records: &[],
        replaying: crate::domain::workers::types::Replaying::Findings,
        stored: &NOTHING_STORED,
        handed: &NOTHING_HANDED,
        wanted_buckets: &NOTHING_WANTED,
        budget: MemoryBytes(0),
        noted: Noting::Unwanted,
        roled: RoleRead::Skips,
        needs: &[],
        answering: &crate::domain::render::types::Answering::Findings,
        keeping: crate::domain::visit::types::Keeping::Keeps,
    }
}

fn partial() -> Partial {
    Partial {
        relation: Relation::Defines,
        cause: PartialCause("syntax error".into()),
        lacking: Lacking::Any,
        at: None,
    }
}

fn needs() -> Needs {
    let read = RelationRead {
        relation: Relation::Defines,
        kinds: Vec::new(),
    };
    Needs {
        reads: vec![read.clone()],
        per_file: Vec::new(),
        cross: vec![read],
    }
}

fn words_of_numbers() -> Words {
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    push_row(&mut table, &[Cell::Number(Integer(7))]);
    let mut words = Words::default();
    encode_table(&table, &[], &mut words);
    words
}

fn entry() -> CorpusEntry {
    CorpusEntry {
        path: FilePath("a.rs".into()),
        size: FileSize(10),
        modified: Nanoseconds(5),
    }
}

fn unread() -> FileOutcome {
    FileOutcome::Unread {
        reason: Unreadable::NoLanguage,
        ran: None,
    }
}

fn ran() -> Ran {
    made_ran(
        &tables(),
        FileId(4),
        Digest([0; 16]),
        Vec::new(),
        ArenaBase(0),
    )
}

fn plan_with_a_frontier() -> Plan {
    Plan {
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Project {
                from: PlanInput::Relation(Relation::Function),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Dropped,
            },
            scope: StepScope::PerFile,
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: vec![PlanInput::Step(StepId(0))],
        comparison_origins: Vec::new(),
    }
}

fn cover_scenario(partials: Vec<Partial>, mask: Mask) -> Result<Value, Failure> {
    let placed = placed()?;
    let needs = [needs()];
    let ids = [RuleId("rule".into())];
    let setting = VisitSetting {
        needs: &needs,
        ids: &ids,
        ..setting(&placed)
    };
    let mut worker = tables();
    worker.partials = partials;
    worker.mask = mask;
    cover(&mut worker, &setting, FileId(3), &FilePath("a.rs".into()));
    Ok(count(worker.unknowns.len()) * 10 + count(worker.covered.len()))
}

fn replay_scenario(outcome: FileOutcome) -> Value {
    let record = Record {
        path: FilePath("a.rs".into()),
        size: FileSize(1),
        modified: Nanoseconds(1),
        digest: Digest([0; 16]),
        number: FileNumber(0),
        outcome,
    };
    let mut worker = tables();
    replay(
        &mut worker,
        FileId(3),
        FileOrder(0),
        &record,
        &entry(),
        Digest([1; 16]),
    );
    let fresh = &worker.fresh[0].record;
    let renewed = fresh.number == FileNumber(3) && fresh.digest == Digest([1; 16]);
    count(worker.unread.len()) * 100 + count(worker.fresh.len()) * 10 + u64::from(renewed)
}

fn finish_scenario() -> Result<Value, Failure> {
    let placed = placed()?;
    let setting = VisitSetting {
        noted: Noting::Wanted,
        ..setting(&placed)
    };
    let mut worker = tables();
    worker.pool.partial = vec![partial()];
    notes_mut(&mut worker.elements).0.push(9);
    worker.language = Some(LanguageNumber(0));
    finish_file(
        &mut worker,
        &setting,
        FileOrder(0),
        &entry(),
        Digest([1; 16]),
        FileNumber(3),
        Some(unread()),
    );
    Ok(count(worker.fresh.len()) * 100
        + count(worker.notes.len()) * 10
        + count(worker.pool.partial.len()))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::PartialsOf => {
            let kept = stored_of(&[Partial {
                lacking: Lacking::Kind(LackingKind("fn".into())),
                ..partial()
            }]);
            Ok(count(kept.len()) * 100
                + u64::from(kept[0].relation_name == RelationName("defines".into())) * 10
                + count(kept[0].lacking_kind.0.len()))
        }
        Scenario::MovedWords => {
            let mut worker = tables();
            worker.captured = vec![Kept {
                bucket: Bucket(1),
                words: KeptWords(pack(WordSlice(&words_of_numbers().0))),
            }];
            let moved = moved_words(&worker, FileId(3), ArenaBase(0)).unwrap_or_default();
            Ok(count(moved.len()) * 10
                + u64::from(
                    moved.first().map(|each| pack(WordSlice(&each.words.0)))
                        == Some(worker.captured[0].words.0.clone()),
                ))
        }
        Scenario::FrontierFlat => {
            let placed = placed()?;
            let plans = [plan_with_a_frontier()];
            let first = [Bucket(1)];
            let setting = VisitSetting {
                plans: &plans,
                first_bucket: &first,
                ..setting(&placed)
            };
            let moved = [
                BucketWords {
                    bucket: Bucket(1),
                    words: Words(vec![5, 6]),
                },
                BucketWords {
                    bucket: Bucket(9),
                    words: Words(vec![7]),
                },
            ];
            let mut flat = Flat(vec![99]);
            frontier_flat(&moved, &setting, RuleIndex(0), &mut flat);
            Ok(flat.0.iter().sum())
        }
        Scenario::MadeRan => {
            let ran = made_ran(
                &tables(),
                FileId(5),
                Digest([2; 16]),
                Vec::new(),
                ArenaBase(0),
            );
            Ok(u64::from(ran.number.0) * 10 + u64::from(ran.interface == Digest([2; 16])))
        }
        Scenario::Cover => cover_scenario(vec![partial()], Mask(vec![1])),
        Scenario::ReplayRead => Ok(replay_scenario(FileOutcome::Read(ran()))),
        Scenario::FinishKept => finish_scenario(),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_relation_read_in_part_is_written_with_its_name_and_the_kind_it_lacks"),
        word: Word::Succeeds,
        input: || Scenario::PartialsOf,
        expect: Expect::Value(112),
    },
    Case {
        name: CaseName("words_with_numbers_only_are_the_same_in_another_run"),
        word: Word::Succeeds,
        input: || Scenario::MovedWords,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("one_rules_share_of_what_a_file_handed_over_is_its_buckets_and_words"),
        word: Word::Succeeds,
        input: || Scenario::FrontierFlat,
        expect: Expect::Value(14),
    },
    Case {
        name: CaseName("the_record_of_what_the_rules_gave_names_the_file_and_what_it_shows"),
        word: Word::Succeeds,
        input: || Scenario::MadeRan,
        expect: Expect::Value(51),
    },
    Case {
        name: CaseName("a_file_read_in_part_costs_a_rule_that_trusts_its_missing_rows_a_finding"),
        word: Word::Succeeds,
        input: || Scenario::Cover,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("a_file_the_store_kept_is_renumbered_and_its_record_kept_again"),
        word: Word::Succeeds,
        input: || Scenario::ReplayRead,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName(
            "a_visit_ends_with_the_record_and_the_notes_when_the_run_keeps_its_findings",
        ),
        word: Word::Succeeds,
        input: || Scenario::FinishKept,
        expect: Expect::Value(110),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
