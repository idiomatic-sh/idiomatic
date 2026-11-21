//! The cases of `dispatch` over the order inside a need's formula. One rule reads a table of constants (table 1) and its
//! branch's need names, in the order a case states, a second table (table 0, whose pipeline scans the file's nodes) and
//! a kind of node: kind 1, which the file's one node has, or kind 3, which no node has. A part that costs nothing (a
//! kind's presence) is decided before a part that makes a table, so a need the kind decides makes no table.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::dispatch::service::dispatch;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{Branch, Emptiness, LoweredPack, LoweredRule};
use crate::domain::made_tables::fixtures::packs::{
    constants, of, operands, pipeline, rows, scan_nodes,
};
use crate::domain::made_tables::sources::service::table_of;
use crate::domain::programs::fixtures::{framed_over, held};
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::steps::types::{
    AnchorKinds, EmitTarget, MadeTableId, PlacedStep, ProgramStep, Register, RegisterCount,
    StepProgram, StepRows,
};
use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node};
use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Relation};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The need of the rule's branch.
#[derive(Clone, Copy, Debug)]
enum Need {
    /// Every part holds: the table has a row, and a node is of the kind. The table is named first.
    TableThenKind(u16),
    /// Some part holds: the table has a row, or a node is of the kind. The table is named first.
    TableOrKind(u16),
}

type Input = Need;
/// The rows found, whether the need's table holds a row (1: it was made), and the rows the fills' stages read and gave.
type Value = [u64; 3];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_kind_no_node_has_decides_an_all_before_its_table_is_made"),
        word: Word::Succeeds,
        input: || Need::TableThenKind(3),
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName("a_kind_a_node_has_leaves_an_all_to_its_table"),
        word: Word::Succeeds,
        input: || Need::TableThenKind(1),
        expect: Expect::Value([2, 1, 5]),
    },
    Case {
        name: CaseName("a_kind_a_node_has_decides_an_any_before_its_table_is_made"),
        word: Word::Succeeds,
        input: || Need::TableOrKind(1),
        expect: Expect::Value([2, 0, 0]),
    },
    Case {
        name: CaseName("a_kind_no_node_has_leaves_an_any_to_its_table"),
        word: Word::Succeeds,
        input: || Need::TableOrKind(3),
        expect: Expect::Value([2, 1, 5]),
    },
];

/// A file's tree of one node of kind 1.
fn tree() -> NodeTable {
    let mut made = NodeTable::default();
    let mut writer = builder(&mut made, Count(4));
    enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
    leave_node(&mut writer, Offset(8));
    let _table = finished(&mut writer);
    made
}

/// The rule set: the need's table (0), the constants the rule reads (1), one rule.
fn pack(need: Need) -> LoweredPack {
    let kind = |bits: u16| Emptiness::Kind(AnchorKinds(Box::new([KindBits(bits)])));
    let table = || Emptiness::Table(MadeTableId(0));
    let emptiness = match need {
        Need::TableThenKind(bits) => Emptiness::All(Box::new([table(), kind(bits)])),
        Need::TableOrKind(bits) => Emptiness::Any(Box::new([table(), kind(bits)])),
    };
    let steps = [
        ProgramStep::EachRow {
            source: StepRows::Made(MadeTableId(1)),
            out: Register(900),
        },
        ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: operands([]),
        },
    ];
    LoweredPack {
        tables: vec![
            rows(
                1,
                pipeline(Vec::from(scan_nodes(Relation::SyntaxTree)), [of(1)]),
            ),
            constants(&[1, 2]),
        ],
        rules: vec![LoweredRule {
            rule: SelectedRuleOrdinal(0),
            node: NodeDigest([1; 16]),
            branches: vec![Branch {
                emptiness,
                program: StepProgram(
                    steps
                        .into_iter()
                        .map(|step| PlacedStep { point: None, step })
                        .collect(),
                ),
            }],
            columns: RuleColumns::default(),
        }],
        registers: RegisterCount(1000),
        ..LoweredPack::default()
    }
}

fn subject(need: Input) -> Result<Value, Failure> {
    let programs = Programs(ProgramsEngine {
        code: encoded(&pack(need)).map_err(|_| ())?,
        ..ProgramsEngine::default()
    });
    let (nodes, mut held) = (tree(), held(1000));
    let (counted, found) = framed_over(&nodes, &programs.0, &mut held, &TheLoop, |frame| {
        dispatch(frame, &programs)
    });
    Ok([
        u64::try_from(found.rows.len()).unwrap_or(u64::MAX),
        u64::from(table_of(&held.store, MadeTableId(0)).is_some_and(|slot| slot.rows.count.0 > 0)),
        counted.fill_rows.0,
    ])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
