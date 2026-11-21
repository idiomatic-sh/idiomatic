//! The scan's cases over a fixture pack, through the use case: the worker's state is reset for each file, the fill side
//! makes the table the rule reads from the rows the pack states, the loop finds the rows, and the scan answers its
//! counts. The file scanned is the fill side's fixture tree; a rule over a table reads none of it.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::encoding::service::encoded;
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, ConstantRows, Emptiness, LoweredPack, LoweredRule, MadeFill, MadeTable,
};
use crate::domain::made_tables::fixtures::packs::nodes;
use crate::domain::programs::fixtures::{Collected, FILE, NoFacts};
use crate::domain::programs::service::scratch_for;
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::steps::types::{
    AxisDirection, EmitTarget, MadeColumn, MadeTableId, PlacedStep, ProgramStep, ReadColumn,
    Register, RegisterCount, StepOperand, StepOperands, StepProgram, StepRows, TestOperation,
};
use crate::inbound::use_cases::scan::interface::ScanUseCase as _;
use crate::inbound::use_cases::scan::use_case::ScanUseCaseImpl;
use ryo_language::domain::facts::types::{CellWord, FileFacts};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;
use std::borrow::Cow;

/// The rule a case's rule set holds.
#[derive(Clone, Copy, Debug)]
enum Scene {
    /// The first cell of each row of a constant table, (1, 10), (2, 3) and (3, 7), whose second cell is above 5.
    OverATable,
    /// Each element of the file's own tree that has a parent, with the parent: a root (0), three children (1, 2, 3),
    /// each holding one token (4, 5, 6). An element is shown as its place; its handle holds the file too.
    OverTheTree,
}

/// The scene, and how many files the one worker scans, one after the other.
type Input = (Scene, usize);
/// For each file: the cells found, then 99, the steps, candidates, rows visited, rows made and fill rows the scan counted,
/// then 98.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_scan_makes_the_table_runs_the_rule_and_counts"),
        word: Word::Succeeds,
        input: || (Scene::OverATable, 1),
        expect: Expect::Value(Cow::Borrowed(&[1, 3, 99, 13, 3, 3, 3, 0, 98])),
    },
    Case {
        name: CaseName("the_next_file_is_scanned_from_nothing_with_the_same_state"),
        word: Word::Succeeds,
        input: || (Scene::OverATable, 2),
        expect: Expect::Value(Cow::Borrowed(&[
            1, 3, 99, 13, 3, 3, 3, 0, 98, 1, 3, 99, 13, 3, 3, 3, 0, 98,
        ])),
    },
    Case {
        name: CaseName("a_scan_walks_the_files_own_tree"),
        word: Word::Succeeds,
        input: || (Scene::OverTheTree, 1),
        expect: Expect::Value(Cow::Borrowed(&[
            1, 0, 2, 0, 3, 0, 4, 1, 5, 2, 6, 3, 99, 20, 7, 13, 0, 0, 98,
        ])),
    },
];

/// The steps of the rule over the tree: each node, its parent along the axis, both emitted.
fn tree_steps() -> Vec<ProgramStep> {
    vec![
        ProgramStep::EachRow {
            source: StepRows::Node(Relation::SyntaxTree),
            out: Register(0),
        },
        ProgramStep::Axis {
            relation: Relation::SyntaxTree,
            direction: AxisDirection::Parent,
            start: Register(0),
            out: Register(1),
        },
        ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: StepOperands(Box::new([
                StepOperand::Register(Register(0)),
                StepOperand::Register(Register(1)),
            ])),
        },
    ]
}

/// The rule set: one rule over one constant table, or over the tree.
fn pack(scene: Scene) -> LoweredPack {
    let made = StepRows::Made(MadeTableId(0));
    let read = |column: u16, out: u32| ProgramStep::Read {
        source: made,
        column: ReadColumn::Made(MadeColumn(column)),
        row: Register(0),
        out: Register(out),
    };
    let table_steps = vec![
        ProgramStep::EachRow {
            source: made,
            out: Register(0),
        },
        read(1, 1),
        ProgramStep::Test {
            test: TestOperation::Greater,
            left: StepOperand::Register(Register(1)),
            right: StepOperand::Word(CellWord(5)),
        },
        read(0, 2),
        ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: StepOperands(Box::new([StepOperand::Register(Register(2))])),
        },
    ];
    let steps = match scene {
        Scene::OverATable => table_steps,
        Scene::OverTheTree => tree_steps(),
    };
    let row = |first: u64, second: u64| -> Box<[CellWord]> {
        Box::new([CellWord(first), CellWord(second)])
    };
    LoweredPack {
        definitions: Vec::new(),
        tables: vec![MadeTable {
            node: NodeDigest([2; 16]),
            fill: MadeFill::Constant(ConstantRows(vec![row(1, 10), row(2, 3), row(3, 7)])),
            width: ColumnCount(2),
            indexes: Vec::new(),
            emptiness: Emptiness::Unknown,
        }],
        rules: vec![LoweredRule {
            rule: SelectedRuleOrdinal(0),
            node: NodeDigest([1; 16]),
            branches: vec![Branch {
                emptiness: Emptiness::Unknown,
                program: StepProgram(
                    steps
                        .into_iter()
                        .map(|step| PlacedStep { point: None, step })
                        .collect(),
                ),
            }],
            columns: RuleColumns {
                emit: ColumnKinds(match scene {
                    Scene::OverATable => vec![CellKind::Number],
                    Scene::OverTheTree => vec![CellKind::Node, CellKind::Node],
                }),
                frontier: Vec::new(),
            },
        }],
        registers: RegisterCount(3),
        regexes: Vec::new(),
        texts: Vec::new(),
        reads: Vec::new(),
    }
}

fn subject((scene, files): Input) -> Result<Value, Failure> {
    let programs = Programs(ProgramsEngine {
        code: encoded(&pack(scene)).map_err(|_| ())?,
        ..ProgramsEngine::default()
    });
    let mut scratch = scratch_for(&programs);
    let (nodes, mut all) = (nodes(), Vec::new());
    for _ in 0..files {
        let (mut facts, mut found) = (NoFacts, Collected::default());
        let file = FileFacts {
            table: &nodes,
            source: ByteSlice(b""),
            file: FILE,
            facts: &mut facts,
        };
        let counted = ScanUseCaseImpl::new(std::sync::Arc::new(MatchingUseCaseImpl))
            .scan(&programs, file, &mut scratch, &mut found)
            .map_err(|_| ())?;
        all.extend(
            found
                .rows
                .into_iter()
                .flatten()
                .map(|cell| cell & u64::from(u32::MAX)),
        );
        all.extend([
            99,
            counted.steps.0,
            counted.candidates.0,
            counted.visited.0,
            counted.made_rows.0,
            counted.fill_rows.0,
            98,
        ]);
    }
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
