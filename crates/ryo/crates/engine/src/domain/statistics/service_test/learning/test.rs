//! The cases of what a sample learns. A first sample writes each unit of the programs it ran, named by its node, with the
//! runs of its plan points and what it was asked and gave, and the bytes read back as written. A second sample decides:
//! a flip candidate keeps the form that did less counted work, a unit that is no candidate never changes form, and a
//! unit's measured order is denied when the second sample did more work in it than the first by over one part in fifty
//! and fifty steps. A row says the stage read back, then each unit in name order: what it is and the first byte of its
//! node's digest, its form, its shape, its order, its runs, and its uses, keys, rows and build.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::{
    FillCode, MeasuredForm, ProgramCode, ShapeChoice, UnitCode, UnitKey, WordSlot,
};
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::statistics::service::{decoded, learned, learned_from};
use crate::domain::statistics::types::{
    LearnStage, Measured, OrderVerdict, PointCounts, PointCountsEngine, StatisticsBytes, UnitCounts,
};
use crate::domain::steps::types::{ColumnMask, MadeColumn};
use ryo_relations::domain::relations::service::column_name;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::query_plans::types::NodeDigest;
use std::borrow::Cow;

/// What a row learns and reads back.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The first sample alone.
    First,
    /// The first sample, then the second.
    Both,
    /// The second sample learned from the first's bytes, against both learned at once: `same` when the bytes are equal.
    FromBytes,
    /// The second sample's statistics given a third sample: `same` when they are answered as given.
    AfterSecond,
    /// A rule whose first sample did 1,000 steps of work and whose second did this many.
    RuleAgain(usize),
    /// A pattern the lowering did not mark open, which a second sample all the same ran as a table that did less work.
    NotACandidate,
    /// The first sample's bytes with their last byte cut off.
    CutShort,
    /// The first sample's bytes with one byte after their last unit.
    ByteAfter,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_first_sample_writes_each_unit_with_its_runs_and_they_read_back"),
        word: Word::Succeeds,
        input: || Scenario::First,
        expect: Expect::Value(Cow::Borrowed(
            "first; pattern 2 by [0] call open learned [100, 50] 7/5/3/0; pattern 2 by [1] call open learned [40] 1/1/1/0; pattern 3 by [0] call open learned [4, 2] 2/2/1/0; pattern 5 by [0] call decided learned [20, 20] 9/9/9/0; rule 1 call decided learned [10, 10, 5] 1/10/5/0; table 4 call decided learned [0] 3/1/8/40",
        )),
    },
    Case {
        name: CaseName(
            "a_candidate_keeps_the_form_that_did_less_work_and_a_dearer_order_is_denied",
        ),
        word: Word::Succeeds,
        input: || Scenario::Both,
        expect: Expect::Value(Cow::Borrowed(
            "second; pattern 2 by [0] table open learned [100, 50] 7/5/3/0; pattern 2 by [1] table open learned [40] 1/1/1/0; pattern 3 by [0] call open denied [4, 2] 2/2/1/0; pattern 5 by [0] call decided denied [20, 20] 9/9/9/0; rule 1 call decided learned [10, 10, 5] 1/10/5/0; table 4 call decided learned [0] 3/1/8/40",
        )),
    },
    Case {
        name: CaseName("learned_from_the_first_samples_bytes_is_learned_of_both_samples"),
        word: Word::Succeeds,
        input: || Scenario::FromBytes,
        expect: Expect::Value(Cow::Borrowed("same")),
    },
    Case {
        name: CaseName("after_a_second_sample_nothing_more_is_learned"),
        word: Word::Succeeds,
        input: || Scenario::AfterSecond,
        expect: Expect::Value(Cow::Borrowed("same")),
    },
    Case {
        name: CaseName("an_order_that_cost_one_part_in_fifty_and_fifty_steps_more_is_kept"),
        word: Word::Succeeds,
        input: || Scenario::RuleAgain(1070),
        expect: Expect::Value(Cow::Borrowed(
            "second; rule 1 call decided learned [1000] 0/0/0/0",
        )),
    },
    Case {
        name: CaseName("an_order_that_cost_one_step_more_than_that_is_denied"),
        word: Word::Succeeds,
        input: || Scenario::RuleAgain(1071),
        expect: Expect::Value(Cow::Borrowed(
            "second; rule 1 call decided denied [1000] 0/0/0/0",
        )),
    },
    Case {
        name: CaseName("a_unit_that_is_no_candidate_never_changes_form"),
        word: Word::Succeeds,
        input: || Scenario::NotACandidate,
        expect: Expect::Value(Cow::Borrowed(
            "second; pattern 6 by [0] call decided learned [100] 0/0/0/0",
        )),
    },
    Case {
        name: CaseName("units_cut_short_are_absent"),
        word: Word::Succeeds,
        input: || Scenario::CutShort,
        expect: Expect::Value(Cow::Borrowed("absent")),
    },
    Case {
        name: CaseName("a_byte_after_the_last_unit_is_absent"),
        word: Word::Succeeds,
        input: || Scenario::ByteAfter,
        expect: Expect::Value(Cow::Borrowed("absent")),
    },
];

/// One unit of a sample's programs with what the sample counted of it: its name, its form and shape, the runs of its plan
/// points, and its uses, keys, rows and build.
struct Unit(
    UnitKey,
    MeasuredForm,
    ShapeChoice,
    &'static [usize],
    [usize; 4],
);

fn node(first: u8) -> NodeDigest {
    let mut digest = [0; 16];
    digest[0] = first;
    NodeDigest(digest)
}

fn pattern(first: u8) -> UnitKey {
    let mask = |column| ColumnMask(Box::new([MadeColumn(column)]));
    UnitKey::Pattern(node(first), mask(0), mask(1))
}

/// The pattern of a node that binds the other column.
fn turned(first: u8) -> UnitKey {
    let mask = |column| ColumnMask(Box::new([MadeColumn(column)]));
    UnitKey::Pattern(node(first), mask(1), mask(0))
}

/// The programs and the counts of a sample that ran `units`, each unit's plan points after the one before's.
fn sample(units: &[Unit]) -> (Programs, PointCounts) {
    let (mut code, mut counts) = (ProgramCode::default(), PointCountsEngine::default());
    for Unit(key, form, shape, runs, [calls, keys, rows, build]) in units {
        code.units.push(UnitCode {
            key: key.clone(),
            form: *form,
            shape: *shape,
            first: WordSlot(u32::try_from(counts.points.len()).unwrap_or(u32::MAX)),
            points: WordSlot(u32::try_from(runs.len()).unwrap_or(u32::MAX)),
        });
        counts.points.extend(runs.iter().map(|each| Count(*each)));
        counts.units.push(UnitCounts {
            calls: Count(*calls),
            keys: Count(*keys),
            rows: Count(*rows),
            build: Count(*build),
            nanos: Count(77),
            idle_nanos: Count(7),
        });
    }
    let programs = ProgramsEngine {
        code,
        ..ProgramsEngine::default()
    };
    (Programs(programs), PointCounts(counts))
}

const CALL: MeasuredForm = MeasuredForm::Call;
const TABLE: MeasuredForm = MeasuredForm::Table;
const OPEN: ShapeChoice = ShapeChoice::Open;
const DECIDED: ShapeChoice = ShapeChoice::Decided;

/// The first sample, loaded with no statistics: a rule, three flip candidates in call form (two of them patterns of node
/// 2), a group table, and a pattern whose call form the shape rule decided.
fn first() -> Vec<Unit> {
    vec![
        Unit(
            UnitKey::Rule(node(1)),
            CALL,
            DECIDED,
            &[10, 10, 5],
            [1, 10, 5, 0],
        ),
        Unit(pattern(2), CALL, OPEN, &[100, 50], [7, 5, 3, 0]),
        Unit(turned(2), CALL, OPEN, &[40], [1, 1, 1, 0]),
        Unit(pattern(3), CALL, OPEN, &[4, 2], [2, 2, 1, 0]),
        Unit(
            UnitKey::Table(node(4), FillCode(1)),
            CALL,
            DECIDED,
            &[0],
            [3, 1, 8, 40],
        ),
        Unit(pattern(5), CALL, DECIDED, &[20, 20], [9, 9, 9, 0]),
    ]
}

/// The second sample, loaded with the first's statistics: every candidate as a table. The table of node 2 did 70 of work,
/// 35 for each of the two patterns that read it, whose call forms did 150 and 40; the table of node 3 did 32 where its
/// call form did 6. The rule did less work than before, the decided pattern far more.
fn second() -> Vec<Unit> {
    let rows = FillCode(0);
    vec![
        Unit(
            UnitKey::Rule(node(1)),
            CALL,
            DECIDED,
            &[10, 4, 5],
            [1, 10, 5, 0],
        ),
        Unit(pattern(2), TABLE, OPEN, &[], [0; 4]),
        Unit(turned(2), TABLE, OPEN, &[], [0; 4]),
        Unit(pattern(3), TABLE, OPEN, &[], [0; 4]),
        Unit(
            UnitKey::Table(node(2), rows),
            CALL,
            DECIDED,
            &[0],
            [10, 1, 6, 60],
        ),
        Unit(
            UnitKey::Table(node(3), rows),
            CALL,
            DECIDED,
            &[0],
            [2, 1, 6, 30],
        ),
        Unit(
            UnitKey::Table(node(4), FillCode(1)),
            CALL,
            DECIDED,
            &[0],
            [3, 1, 8, 40],
        ),
        Unit(pattern(5), CALL, DECIDED, &[200, 200], [9, 9, 9, 0]),
    ]
}

/// What bytes read back as: the stage, then each unit.
fn read_back(bytes: &[u8]) -> String {
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
        fields: Vec::new(),
        fixed: Vec::new(),
    };
    let Some(statistics) = decoded(StatisticsBytes(bytes), &kinds) else {
        return "absent".to_owned();
    };
    let mut said = vec![match statistics.stage {
        LearnStage::First => "first".to_owned(),
        LearnStage::Second => "second".to_owned(),
    }];
    for unit in &statistics.units {
        let name = match &unit.key {
            UnitKey::Pattern(node, bound, _) => {
                let bound: Vec<u16> = bound.0.iter().map(|column| column.0).collect();
                format!("pattern {} by {bound:?}", node.0[0])
            }
            UnitKey::Rule(node) => format!("rule {}", node.0[0]),
            UnitKey::Table(node, _) => format!("table {}", node.0[0]),
        };
        let form = match unit.form {
            MeasuredForm::Call => "call",
            MeasuredForm::Table => "table",
        };
        let shape = match unit.shape {
            ShapeChoice::Decided => "decided",
            ShapeChoice::Open => "open",
        };
        let order = match unit.order {
            OrderVerdict::Learned => "learned",
            OrderVerdict::Denied => "denied",
        };
        let runs: Vec<usize> = unit.points.iter().map(|runs| runs.0).collect();
        let counts = unit.counts;
        said.push(format!(
            "{name} {form} {shape} {order} {runs:?} {}/{}/{}/{}",
            counts.calls.0, counts.keys.0, counts.rows.0, counts.build.0
        ));
    }
    said.join("; ")
}

fn same(left: &[u8], right: &[u8]) -> String {
    if left == right { "same" } else { "differ" }.to_owned()
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let ((first, counted), (second, recounted)) = (sample(&first()), sample(&second()));
    let measured = |programs, counts| Measured { programs, counts };
    let (one, two) = (measured(&first, &counted), measured(&second, &recounted));
    let said = match scenario {
        Scenario::First => read_back(&learned(one, None).0),
        Scenario::Both => read_back(&learned(one, Some(two)).0),
        Scenario::FromBytes => {
            let bytes = learned(one, None);
            let from = learned_from(StatisticsBytes(&bytes.0), two);
            same(&from.0, &learned(one, Some(two)).0)
        }
        Scenario::AfterSecond => {
            let both = learned(one, Some(two));
            same(&learned_from(StatisticsBytes(&both.0), one).0, &both.0)
        }
        Scenario::RuleAgain(work) => {
            let rule = |runs: &'static [usize]| {
                sample(&[Unit(UnitKey::Rule(node(1)), CALL, DECIDED, runs, [0; 4])])
            };
            let again: &'static [usize] = if work == 1070 { &[1070] } else { &[1071] };
            let ((before, ran), (after, reran)) = (rule(&[1000]), rule(again));
            read_back(&learned(measured(&before, &ran), Some(measured(&after, &reran))).0)
        }
        Scenario::NotACandidate => {
            let (before, ran) = sample(&[Unit(pattern(6), CALL, DECIDED, &[100], [0; 4])]);
            let (after, reran) = sample(&[
                Unit(pattern(6), TABLE, DECIDED, &[], [0; 4]),
                Unit(
                    UnitKey::Table(node(6), FillCode(0)),
                    CALL,
                    DECIDED,
                    &[],
                    [1, 1, 1, 1],
                ),
            ]);
            read_back(&learned(measured(&before, &ran), Some(measured(&after, &reran))).0)
        }
        Scenario::CutShort => {
            let bytes = learned(one, None).0;
            read_back(
                bytes
                    .get(..bytes.len().saturating_sub(1))
                    .unwrap_or_default(),
            )
        }
        Scenario::ByteAfter => {
            let mut bytes = learned(one, None).0;
            bytes.push(0);
            read_back(&bytes)
        }
    };
    Ok(Cow::Owned(said))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// What a sample sized is read by the names of the vocabulary it is decoded with: a kind and a field take that vocabulary's
/// codes, a name it does not hold matches nothing, and a constant bucket is found by its relation, column and text.
#[test]
fn sizes_are_read_by_name() {
    use crate::domain::statistics::bytes::service::written;
    use crate::domain::statistics::service::{element_rows, field_rows, kind_rows, literal_rows};
    use crate::domain::statistics::types::{NamedSize, NamedSizes, SizedRows};
    use ryo_language::domain::node_table::types::FieldCode;
    use ryo_relations::domain::relations::service::{columns, name};
    use ryo_relations::domain::relations::types::{KindBits, Relation};
    use ryo_relations::domain::terms::types::NameText;
    use ryo_values::domain::descriptions::types::{FieldWord, KindWord};

    let named = |name: &str, rows| NamedSize {
        name: name.into(),
        rows: Count(rows),
    };
    let relation = Relation::SyntaxTree;
    let column = columns(relation)[1];
    let bucket = [name(relation).0, column_name(column).0, "global"].join("\n");
    let sizes = NamedSizes {
        files: Count(4),
        elements: Count(4000),
        kinds: vec![named("compare", 12), named("id", 900), named("gone", 5)],
        fields: vec![named("left", 30)],
        literals: vec![named(&bucket, 7)],
    };
    let bytes = written(LearnStage::First, &[], &sizes);
    // The vocabulary of the load: `id` is kind 0 and `compare` kind 1 here, whatever they were when the sample ran.
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: vec![KindWord("id".into()), KindWord("compare".into())],
        fields: vec![FieldWord("".into()), FieldWord("left".into())],
        fixed: Vec::new(),
    };
    let statistics = decoded(StatisticsBytes(&bytes.0), &kinds).expect("the sizes read");
    let sized = |rows| {
        Some(SizedRows {
            rows: Count(rows),
            files: Count(4),
        })
    };
    assert_eq!(element_rows(&statistics), sized(4000));
    assert_eq!(kind_rows(&statistics, KindBits(0)), sized(900));
    assert_eq!(kind_rows(&statistics, KindBits(1)), sized(12));
    assert_eq!(kind_rows(&statistics, KindBits(2)), None);
    assert_eq!(field_rows(&statistics, FieldCode(1)), sized(30));
    assert_eq!(field_rows(&statistics, FieldCode(0)), None);
    assert_eq!(
        literal_rows(&statistics, relation, column, NameText("global")),
        sized(7)
    );
    assert_eq!(
        literal_rows(&statistics, relation, column, NameText("local")),
        None
    );
}
