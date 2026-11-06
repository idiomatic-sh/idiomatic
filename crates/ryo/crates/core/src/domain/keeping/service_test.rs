//! The `keeping` service's cases, on values alone.

use ryo_relations::domain::relations::service::{node_file, node_id};
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Name, NamePlace, Offset, Origin, PlaceNumber, Position, Span, TextHash,
    Value as Cell, WorkerNumber,
};

use super::service::{
    configuration_bytes, findings_of, remap_packed, rendered_of, rows_from, rows_of,
};
use super::types::{ArenaStart, Configuration, DigestInput, HostedText, Remap};

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::render::types::{Rendered, RowNumber, RowOrder};
use {
    crate::domain::finish::types::ColumnNumber, crate::domain::finish::types::FileLine,
    ryo_values::domain::runs::types::RuleNumber,
};

use ryo_values::domain::records::types::RenderedText;
use ryo_values::domain::records::types::{RowEnds, RowsTextAt, TextRange};
use ryo_values::domain::table_words::service::{decode_combined, encode_table, pack, unpacked};
use ryo_values::domain::table_words::types::WordSlice;

use ryo_relations::domain::relations::types::Words;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::records::types::{PlacedColumn, PlacedLine, RowTotal};
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stores::types::{
    FindingMade, FindingPlace, StoredFileNumber, StoredFinding, StoredPlace, StoredValue,
};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// The findings of a file go into the store and come out as they were.
    FindingsComeBack,
    /// The findings the store keeps, written as rows and read back, a column whose value holds a colon among them.
    StoredFindingsComeBack,
    /// Words packed and read back: a small one is one byte, the largest ten.
    PackedWordsComeBack,
    /// The words of a file move to another file number and another worker's arena.
    WordsMoveWithTheFile,
    /// Words that point at another file cannot be moved.
    WordsOfAnotherFileStay,
    /// Words that point at text of another worker cannot be moved.
    WordsOfAnotherWorkerStay,
    /// A rule edited, added or reordered is another configuration, and so is another relation hosted.
    ConfigurationFollowsTheRules,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// A number the answer is.
type Value = usize;
type Failure = ();

fn name(place: u32) -> Cell {
    Cell::Name(Name {
        hash: TextHash(9),
        place: PlaceNumber(place),
    })
}

fn place(origin: Origin, file: u32, start: u32, end: u32) -> NamePlace {
    NamePlace {
        origin,
        file: FileId(file),
        start: Offset(start),
        end: Offset(end),
    }
}

fn words_of(cells: Vec<Cell>) -> Vec<u64> {
    words_with(cells, &[])
}

fn words_with(cells: Vec<Cell>, places: &[NamePlace]) -> Vec<u64> {
    let width = cells.len();
    let table = Table {
        kinds: vec![CellKind::Number; width],
        cells,
        count: RowCount(1),
    };
    let mut out = Words::default();
    encode_table(&table, places, &mut out);
    out.0
}

fn map() -> Remap {
    Remap {
        from_file: FileId(3),
        to_file: FileId(7),
        from_worker: WorkerNumber(1),
        to_worker: WorkerNumber(2),
        from_base: ArenaStart(10),
        to_base: ArenaStart(40),
    }
}

/// A map that moves nothing: the file, the worker and the base are where they were.
fn stays() -> Remap {
    Remap {
        to_file: map().from_file,
        to_worker: map().from_worker,
        to_base: map().from_base,
        ..map()
    }
}

/// The words `remap_packed` makes of `words` packed.
fn moved_packed(words: &[u64], map: &Remap) -> Option<Words> {
    let mut out = Words::default();
    remap_packed(&pack(WordSlice(words)), map, &mut out).map(|()| out)
}

fn configuration(rules: &[RuleSpec]) -> DigestInput {
    hosting(rules, &[])
}

fn hosting(rules: &[RuleSpec], hosted: &[HostedText]) -> DigestInput {
    configuration_bytes(&Configuration { rules, hosted })
}

fn rule(id: &str, query: &str) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query: ryo_values::domain::program::types::QueryBundle::from(QueryText(query.into())),
        applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
        given: Vec::new(),
        observations: Vec::new(),
        spelled: RuleParameters(Box::default()),
        include: vec![Glob("**".into())],
        exclude: Vec::new(),
    }
}

/// The words of a file move to its new number and worker.
fn words_move_with_the_file() -> Value {
    let span = Cell::Span(Span {
        start: Offset(1),
        end: Offset(2),
    });
    let words = words_with(
        vec![
            Cell::File(FileId(3)),
            Cell::Node(node_id(FileId(3), Position(5))),
            name(0),
            name(1),
            span,
            Cell::Number(Integer(8)),
        ],
        &[
            place(Origin::Source, 3, 1, 4),
            place(Origin::Derived(WorkerNumber(1)), 3, 12, 15),
        ],
    );
    let mut places = Vec::new();
    let cells = moved_packed(&words, &map())
        .map(|moved| decode_combined(&pack(WordSlice(&moved.0)), &mut places).cells)
        .unwrap_or_default();
    usize::from(
        cells.len() == 6
            && cells[0] == Cell::File(FileId(7))
            && matches!(cells[1], Cell::Node(node) if node_file(node) == FileId(7))
            && places
                == vec![
                    place(Origin::Source, 7, 1, 4),
                    place(Origin::Derived(WorkerNumber(2)), 7, 42, 45),
                ]
            && cells[4] == span
            && cells[5] == Cell::Number(Integer(8))
            && moved_packed(&words, &stays()) == Some(Words(words.clone())),
    )
}

/// The configuration changes with each rule's query, their order and what is hosted.
fn configuration_follows_the_rules() -> Value {
    let rules = [rule("a", "q1"), rule("b", "q2")];
    let edited = [rule("a", "q1"), rule("b", "q3")];
    let reordered = [rule("b", "q2"), rule("a", "q1")];
    let added = [rule("a", "q1"), rule("b", "q2"), rule("c", "q1")];
    let same = configuration(&rules);
    let with_module = |body: &str| {
        let root = ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("rules/F401")
                .expect("root id is nonblank"),
            "root query",
        );
        let module = ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("python/ruff/suppress")
                .expect("module id is nonblank"),
            body,
        );
        let bundle = ryo_values::domain::program::service::query_bundle(root, [module])
            .expect("source ids are unique");
        let mut rule = rule("a", "root query");
        rule.query = bundle;
        configuration(&[rule])
    };
    usize::from(
        same == configuration(&rules)
            && same != configuration(&edited)
            && same != configuration(&reordered)
            && same != configuration(&added)
            && same != hosting(&rules, &[HostedText("a.rs\tspec\tentry\ttemplate".into())])
            && with_module("one") != with_module("two")
            && with_module("one") != configuration(&[rule("a", "root query")]),
    )
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::FindingsComeBack => {
            let rendered = Rendered {
                text: RenderedText("ra.rs".to_owned()),
                cells: vec![
                    Some(ryo_values::domain::records::types::Cell::Text(TextRange {
                        start: RowsTextAt(0),
                        end: RowsTextAt(1),
                    })),
                    Some(ryo_values::domain::records::types::Cell::File(TextRange {
                        start: RowsTextAt(1),
                        end: RowsTextAt(5),
                    })),
                    None,
                ],
                ends: RowEnds(vec![3]),
                order: vec![RowOrder {
                    rule: RuleNumber(1),
                    line: FileLine(2),
                    column: ColumnNumber(3),
                    row: RowNumber(0),
                }],
                total: RowTotal(1),
                keys: Vec::new(),
            };
            usize::from(rendered_of(&rows_of(&rendered)) == rendered)
        }
        Scenario::StoredFindingsComeBack => {
            let finding = |line: u32| StoredFinding {
                rule: RuleId("r".into()),
                made: FindingMade::InFile(StoredFileNumber(4)),
                place: if line == 0 {
                    FindingPlace::Unplaced
                } else {
                    FindingPlace::At(StoredPlace {
                        line: PlacedLine(line),
                        column: PlacedColumn(3),
                    })
                },
                subject: Some(ryo_values::domain::records::types::Cell::File(
                    "a.rs".into(),
                ))
                .into(),
                key: vec![
                    Some(ryo_values::domain::records::types::Cell::Text("k,1".into())).into(),
                    None.into(),
                    Some(ryo_values::domain::records::types::Cell::Number(Integer(4))).into(),
                ],
                values: vec![
                    StoredValue {
                        column: ColumnName("name".into()),
                        value: Some(ryo_values::domain::records::types::Cell::Text(
                            "ma:in".into(),
                        ))
                        .into(),
                    },
                    StoredValue {
                        column: ColumnName("span".into()),
                        value: Some(ryo_values::domain::records::types::Cell::Span(Span {
                            start: Offset(0),
                            end: Offset(4),
                        }))
                        .into(),
                    },
                ],
            };
            let findings = [finding(2), finding(0)];
            let rows = rows_from(&findings.iter().collect::<Vec<_>>(), &[RuleId("r".into())])
                .unwrap_or_default();
            usize::from(findings_of(&rows, FindingMade::InFile(StoredFileNumber(4))) == findings)
        }
        Scenario::PackedWordsComeBack => {
            let words = [0, 1, 127, 128, 300, u64::MAX];
            let packed = pack(WordSlice(&words));
            usize::from(
                unpacked(&packed).map(|each| each.0).eq(words)
                    && packed.0.len() == 1 + 1 + 1 + 2 + 2 + 10,
            )
        }
        Scenario::WordsMoveWithTheFile => words_move_with_the_file(),
        Scenario::WordsOfAnotherFileStay => {
            let words = words_of(vec![Cell::File(FileId(4))]);
            let named = words_with(vec![name(0)], &[place(Origin::Source, 4, 0, 1)]);
            usize::from(
                moved_packed(&words, &map()).is_none() && moved_packed(&named, &map()).is_none(),
            )
        }
        Scenario::WordsOfAnotherWorkerStay => {
            let far = place(Origin::Derived(WorkerNumber(5)), 3, 12, 15);
            let near = place(Origin::Derived(WorkerNumber(1)), 3, 2, 5);
            usize::from(
                moved_packed(&words_with(vec![name(0)], &[far]), &map()).is_none()
                    && moved_packed(&words_with(vec![name(0)], &[near]), &map()).is_none(),
            )
        }
        Scenario::ConfigurationFollowsTheRules => configuration_follows_the_rules(),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("findings_go_into_the_store_and_come_out_as_they_were"),
        word: Word::Succeeds,
        input: || Scenario::FindingsComeBack,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_findings_the_store_keeps_written_as_rows_read_back_as_they_were"),
        word: Word::Succeeds,
        input: || Scenario::StoredFindingsComeBack,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("words_packed_read_back_as_they_were_a_small_one_in_one_byte"),
        word: Word::Succeeds,
        input: || Scenario::PackedWordsComeBack,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_words_of_a_file_move_to_its_new_number_and_arena"),
        word: Word::Succeeds,
        input: || Scenario::WordsMoveWithTheFile,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("words_that_point_at_another_file_stay_where_they_are"),
        word: Word::Succeeds,
        input: || Scenario::WordsOfAnotherFileStay,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("words_that_point_at_text_of_another_worker_stay_where_they_are"),
        word: Word::Succeeds,
        input: || Scenario::WordsOfAnotherWorkerStay,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_configuration_follows_the_rules_and_what_is_hosted"),
        word: Word::Succeeds,
        input: || Scenario::ConfigurationFollowsTheRules,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

use ryo_values::domain::rows::types::{RowCount, Table};

use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
