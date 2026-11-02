//! The `workers` service's cases, on values alone.

use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::relations::types::WorkerNumber;
use ryo_values::domain::files::types::{
    CorpusEntry, FileNumber, FileSize, Glob, Hits, Nanoseconds,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};
use ryo_values::domain::records::types::{FileOutcome, Record, Unreadable};
use ryo_values::domain::stored_facts::types::{ArenaBytes, Digest};

use super::service::{
    absorb, add_fed, checking, file_at, gathering, govern, governs_rule, order, size_at, tables_for,
};
use super::types::{Checking, FileOrder, FilesRead, FreshRecord, Tables, VisitSetting, WorkersMet};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::lowering::types::Counting;
use crate::domain::operator::types::{PlanIdentity, PlanLayout, RowsFed, TreeSteps};
use crate::domain::placement::service::decide;
use crate::domain::placement::types::{Governing, Mask, PlacedScopes, RuleScope};
use crate::domain::visit::types::{Noting, RoleRead};
use crate::outbound::matching::fixtures::fake::MatchingFake;
use crate::outbound::matching::interface::MatchingPort as _;
use ryo_values::domain::records::types::Unread;
use ryo_values::domain::runs::types::MemoryBytes;

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
    /// Makes the tables of a worker for two plans of two and three steps.
    TablesFor,
    /// Reads the second of two files.
    FileAt,
    /// Reads a place no file is at.
    NoFileAt,
    /// Reads the size of the second of two files.
    SizeAt,
    /// Reads the size of a place no file is at.
    NoSizeAt,
    /// Decides which rules govern a path, when the only rule includes the Rust files.
    Govern(&'static str),
    /// Whether the visit of this place reads the memory.
    Checking(usize),
    /// Adds rows fed to a total that is nearly full.
    AddFed,
    /// Makes room for the plans of one and two steps.
    Gathering,
    /// Takes what a worker made into what the workers made.
    Absorb,
    /// Puts the unread files and the records in the order of the files.
    Order,
    /// Whether the rule at this place governs a file only the second rule governs.
    GovernsRule(usize),
}

/// What a row of the cases does.
type Input = Scenario;
/// A count, or a yes or no as a number.
type Value = u64;
type Failure = ();

fn count(number: usize) -> u64 {
    u64::try_from(number).unwrap_or(u64::MAX)
}

fn yes(governing: Governing) -> u64 {
    u64::from(governing == Governing::Governs)
}

/// A plan of this many steps, all of them the same.
fn plan(steps: usize) -> Plan {
    Plan {
        parameters: Vec::new(),
        steps: std::iter::repeat_with(|| PlanStep {
            operation: Operation::Project {
                from: ryo_values::domain::plan_parts::types::Input::Relation(
                    ryo_language::domain::relations::types::Relation::Function,
                ),
                columns: vec![ColumnIndex(0)],
                distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
            },
            scope: StepScope::PerFile,
        })
        .take(steps)
        .collect(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

fn entry(path: &str, size: u64) -> CorpusEntry {
    CorpusEntry {
        path: FilePath(path.into()),
        size: FileSize(size),
        modified: Nanoseconds(0),
    }
}

fn record(path: &str) -> Record {
    Record {
        path: FilePath(path.into()),
        size: FileSize(0),
        modified: Nanoseconds(0),
        digest: Digest([0; 16]),
        number: FileNumber(0),
        outcome: FileOutcome::Unread {
            reason: Unreadable::NoLanguage,
            ran: None,
        },
    }
}

fn tables(plans: &[Plan]) -> Tables {
    tables_for(
        WorkerNumber(0),
        plans,
        TreeSteps::Lowered,
        Counting::DoesNotCount,
    )
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let files = [entry("a.rs", 10), entry("b.rs", 20)];
    Ok(match scenario {
        Scenario::TablesFor => {
            let worker = tables(&[plan(2), plan(3)]);
            assert!(worker.pool.layouts.capacity() >= 2);
            assert!(worker.pool.fed.capacity() >= 2);
            u64::from(
                worker.pool.layouts.is_empty()
                    && worker.pool.fed.is_empty()
                    && worker.pool.selection_plans.is_empty()
                    && worker.partials.is_empty(),
            )
        }
        Scenario::FileAt => {
            count(file_at(&files, FileOrder(1)).map_or(0, |each| each.path.0.len()))
        }
        Scenario::NoFileAt => u64::from(file_at(&files, FileOrder(5)).is_none()),
        Scenario::SizeAt => size_at(&files, FileOrder(1)).0,
        Scenario::NoSizeAt => size_at(&files, FileOrder(5)).0,
        Scenario::Govern(path) => {
            let placement = decide(&[RuleScope {
                include: vec![Glob("**/*.rs".into())],
                exclude: Vec::new(),
            }]);
            let globs = placement.globs.clone();
            let placed = PlacedScopes {
                matcher: MatchingFake.compile(&globs).map_err(drop)?,
                placement,
            };
            let setting = VisitSetting {
                unknown: &[],
                plans: &[],
                bindings: &[],
                wanted: &[],
                paths: &[],
                first_bucket: &[],
                ids: &[],
                applicability: &[],
                applicability_can_gate: true,
                placed: &placed,
                records: &[],
                replaying: super::types::Replaying::Findings,
                stored: &NOTHING_STORED,
                handed: &NOTHING_HANDED,
                wanted_buckets: &NOTHING_WANTED,
                budget: MemoryBytes(0),
                noted: Noting::Unwanted,
                roled: RoleRead::Skips,
                needs: &[],
                answering: &crate::domain::render::types::Answering::Findings,
                keeping: crate::domain::visit::types::Keeping::Keeps,
            };
            let mut tables = tables(&[]);
            tables.hits = Hits::default();
            yes(govern(&setting, &mut tables, &FilePath(path.into())))
        }
        Scenario::Checking(place) => u64::from(checking(FileOrder(place)) == Checking::Now),
        Scenario::AddFed => {
            let mut total = RowsFed([1, u64::MAX, 3]);
            add_fed(&mut total, &RowsFed([1, 1, 1]));
            total.0[0] * 1000 + total.0[2] + u64::from(total.0[1] == u64::MAX)
        }
        Scenario::Gathering => {
            let met = gathering(&[plan(1), plan(2)]);
            count(met.fed.iter().map(Vec::len).sum())
        }
        Scenario::Absorb => {
            let plans = [plan(1)];
            let mut worker = tables(&plans);
            worker.pool.layouts = vec![PlanLayout {
                plan: PlanIdentity(std::ptr::from_ref(&plans[0]).addr()),
                ..PlanLayout::default()
            }];
            worker.pool.fed = vec![vec![RowsFed([1, 2, 3])]];
            worker.reads = FilesRead(2);
            worker.arena = ArenaBytes(vec![7]);
            let mut met = gathering(&plans);
            absorb(&mut met, worker, &plans);
            met.fed[0][0].0[2] * 100 + u64::from(met.read.0) * 10 + count(met.arenas.len())
        }
        Scenario::Order => {
            let mut met = WorkersMet::default();
            for path in ["b.rs", "a.rs"] {
                met.unread.push(Unread {
                    path: FilePath(path.into()),
                    reason: Unreadable::NoLanguage,
                });
            }
            for place in [2, 0] {
                met.fresh.push(FreshRecord {
                    place: FileOrder(place),
                    record: record("x.rs"),
                });
            }
            let fresh = order(&mut met);
            let first = u64::from(&*met.unread[0].path.0 == "a.rs");
            first * 100 + count(fresh.places[0].0) * 10 + count(fresh.places[1].0)
        }
        Scenario::GovernsRule(rule) => {
            let mut worker = tables(&[]);
            worker.mask = Mask(vec![0b10]);
            yes(governs_rule(&worker, RuleIndex(rule)))
        }
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_worker_prepares_metadata_without_retaining_rule_execution_rows"),
        word: Word::Succeeds,
        input: || Scenario::TablesFor,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_file_at_a_place_is_the_one_among_the_files_in_that_place"),
        word: Word::Succeeds,
        input: || Scenario::FileAt,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("no_file_is_at_a_place_past_the_last_file"),
        word: Word::Succeeds,
        input: || Scenario::NoFileAt,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_size_at_a_place_is_the_size_of_the_file_there"),
        word: Word::Succeeds,
        input: || Scenario::SizeAt,
        expect: Expect::Value(20),
    },
    Case {
        name: CaseName("the_size_at_a_place_with_no_file_is_nothing"),
        word: Word::Succeeds,
        input: || Scenario::NoSizeAt,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_rule_that_includes_rust_files_governs_a_rust_file"),
        word: Word::Succeeds,
        input: || Scenario::Govern("src/a.rs"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_rule_that_includes_rust_files_does_not_govern_a_text_file"),
        word: Word::Succeeds,
        input: || Scenario::Govern("notes.txt"),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("the_first_visit_reads_the_memory"),
        word: Word::Succeeds,
        input: || Scenario::Checking(0),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_visit_after_it_does_not"),
        word: Word::Succeeds,
        input: || Scenario::Checking(1),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("the_visit_one_in_sixty_four_later_reads_it_again"),
        word: Word::Succeeds,
        input: || Scenario::Checking(64),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("rows_fed_are_added_and_stop_at_the_most_a_count_holds"),
        word: Word::Succeeds,
        input: || Scenario::AddFed,
        expect: Expect::Value(2000 + 4 + 1),
    },
    Case {
        name: CaseName("room_is_made_for_each_step_of_each_plan"),
        word: Word::Succeeds,
        input: || Scenario::Gathering,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("what_a_worker_made_is_taken_into_what_the_workers_made"),
        word: Word::Succeeds,
        input: || Scenario::Absorb,
        expect: Expect::Value(321),
    },
    Case {
        name: CaseName("unread_files_and_records_are_put_in_the_order_of_the_files"),
        word: Word::Succeeds,
        input: || Scenario::Order,
        expect: Expect::Value(102),
    },
    Case {
        name: CaseName("a_rule_governs_the_file_if_its_bit_is_set"),
        word: Word::Succeeds,
        input: || Scenario::GovernsRule(1),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_rule_does_not_govern_the_file_if_its_bit_is_not_set"),
        word: Word::Succeeds,
        input: || Scenario::GovernsRule(0),
        expect: Expect::Value(0),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
