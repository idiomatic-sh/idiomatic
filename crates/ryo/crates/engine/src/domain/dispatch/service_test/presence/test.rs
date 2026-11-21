//! The cases of `dispatch` over a need on a node relation's field, on the fill side's fixture tree: eight elements, the
//! third (element 2) under field 4 and the others under no field; the vocabulary's fields 0 to 4 have the words 100 to
//! 104. Two rules read one table, whose pipeline scans the node rows and whose need says the node rows hold a row with a
//! given field (or any row). A table whose need does not hold is never made, and no stage of its fill runs.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::dispatch::service::dispatch;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{
    Branch, Emptiness, LoweredPack, LoweredRule, MadeTable, RowValue,
};
use crate::domain::made_tables::fixtures::packs::{
    nodes, of, operands, pipeline, rows, scan_nodes,
};
use crate::domain::made_tables::sources::service::table_of;
use crate::domain::programs::fixtures::{framed_over, held};
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::steps::types::{
    EmitTarget, MadeTableId, PlacedStep, ProgramStep, Register, RegisterCount, StepProgram,
    StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::collections::relations::semantic_child::item::SemanticChildColumn;
use ryo_relations::domain::relations::types::{Column, Relation};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The word the need asks the field column for; none: any row of the relation.
type Input = Option<u64>;
/// The rows found by the two rules, whether the table was made (1), the rows the fills' stages read and gave, and the
/// terms of the second rule's allowance beyond its candidates (the node table's elements, 8, once it has read it; the
/// rows into and out of the table).
type Value = [u64; 4];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_field_some_element_is_under_makes_the_table"),
        word: Word::Succeeds,
        input: || Some(104),
        expect: Expect::Value([16, 1, 40, 24]),
    },
    Case {
        name: CaseName("a_field_no_element_is_under_makes_no_table_and_runs_no_stage"),
        word: Word::Succeeds,
        input: || Some(103),
        expect: Expect::Value([0, 0, 0, 8]),
    },
    Case {
        name: CaseName("a_word_that_is_no_field_makes_no_table"),
        word: Word::Succeeds,
        input: || Some(555),
        expect: Expect::Value([0, 0, 0, 8]),
    },
    Case {
        name: CaseName("a_need_for_any_row_holds_in_a_file_with_elements"),
        word: Word::Succeeds,
        input: || None,
        expect: Expect::Value([16, 1, 40, 24]),
    },
];

/// The rule set: the one table, read by two rules.
fn pack(wanted: Input) -> LoweredPack {
    let read = |number: u32| {
        let steps = [
            ProgramStep::EachRow {
                source: StepRows::Made(MadeTableId(0)),
                out: Register(900_u32.saturating_add(number)),
            },
            ProgramStep::Emit {
                target: EmitTarget::Rule(SelectedRuleOrdinal(number)),
                columns: operands([]),
            },
        ];
        LoweredRule {
            rule: SelectedRuleOrdinal(number),
            node: NodeDigest([u8::try_from(number).unwrap_or(u8::MAX); 16]),
            branches: vec![Branch {
                emptiness: Emptiness::Unknown,
                program: StepProgram(
                    steps
                        .into_iter()
                        .map(|step| PlacedStep { point: None, step })
                        .collect(),
                ),
            }],
            columns: RuleColumns::default(),
        }
    };
    let value = wanted.map(|word| RowValue {
        column: Column::SemanticChild(SemanticChildColumn::Field),
        value: CellWord(word),
    });
    let stages = Vec::from(scan_nodes(Relation::SemanticChild));
    LoweredPack {
        tables: vec![MadeTable {
            emptiness: Emptiness::Row(Relation::SemanticChild, value),
            ..rows(1, pipeline(stages, [of(1)]))
        }],
        rules: vec![read(0), read(1)],
        registers: RegisterCount(1000),
        ..LoweredPack::default()
    }
}

fn subject(wanted: Input) -> Result<Value, Failure> {
    let programs = Programs(ProgramsEngine {
        code: encoded(&pack(wanted)).map_err(|_| ())?,
        kinds: (200..204).map(CellWord).collect(),
        fields: (100..105).map(CellWord).collect(),
        ..ProgramsEngine::default()
    });
    let (tree, mut held) = (nodes(), held(1000));
    let (counted, found) = framed_over(&tree, &programs.0, &mut held, &TheLoop, |frame| {
        dispatch(frame, &programs)
    });
    Ok([
        u64::try_from(found.rows.len()).unwrap_or(u64::MAX),
        u64::from(table_of(&held.store, MadeTableId(0)).is_some()),
        counted.fill_rows.0,
        held.state.budget.terms.0,
    ])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
