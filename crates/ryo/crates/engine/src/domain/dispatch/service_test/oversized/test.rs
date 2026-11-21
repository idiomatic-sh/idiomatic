//! The cases of `dispatch` over a relation's declared size. One rule reads every row of a filled relation and emits it; the
//! file's tree has two elements, so a relation that grows with the file's elements may hold 16 rows an element and 256
//! beside: 288. A relation of one row a file may hold one.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::dispatch::service::dispatch;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::findings::types::{RuleColumns, WorkVerdict};
use crate::domain::lowered::types::{Branch, Emptiness, LoweredPack, LoweredRule};
use crate::domain::made_tables::fixtures::packs::operands;
use crate::domain::programs::fixtures::{Telling, framed_with, held};
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::steps::types::{
    EmitTarget, PlacedStep, ProgramStep, Register, RegisterCount, StepProgram, StepRows,
};
use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node};
use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Relation};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// The relation the rule reads, how many cells a row of it has, and how many rows the language writes.
type Input = (Relation, usize, u64);
/// For each notice the rule, what it says (0 over its allowance, 1 a relation oversized), the work and the allowance;
/// then 99 and how many rows were found.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_relation_that_grows_with_the_elements_at_its_bound_is_not_said"),
        word: Word::Succeeds,
        input: || (Relation::TokenLink, 5, 288),
        expect: Expect::Value(Cow::Borrowed(&[99, 288])),
    },
    Case {
        name: CaseName("one_row_past_its_bound_it_is_said_oversized_and_every_row_is_found"),
        word: Word::Succeeds,
        input: || (Relation::TokenLink, 5, 289),
        expect: Expect::Value(Cow::Borrowed(&[0, 1, 289, 36992, 99, 289])),
    },
    Case {
        name: CaseName("a_relation_of_one_row_a_file_with_one_row_is_not_said"),
        word: Word::Succeeds,
        input: || (Relation::FilesystemFile, 8, 1),
        expect: Expect::Value(Cow::Borrowed(&[99, 1])),
    },
    Case {
        name: CaseName("a_relation_of_one_row_a_file_with_two_rows_is_said_oversized"),
        word: Word::Succeeds,
        input: || (Relation::FilesystemFile, 8, 2),
        expect: Expect::Value(Cow::Borrowed(&[0, 1, 2, 256, 99, 2])),
    },
];

/// A file's tree of two elements.
fn tree() -> NodeTable {
    let mut made = NodeTable::default();
    let mut writer = builder(&mut made, Count(4));
    enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
    enter_node(&mut writer, KindBits(2), FieldCode(0), Offset(0));
    leave_node(&mut writer, Offset(2));
    leave_node(&mut writer, Offset(8));
    let _table = finished(&mut writer);
    made
}

/// The rule set: one rule that reads every row of the relation and emits it.
fn pack(relation: Relation) -> LoweredPack {
    let steps = [
        ProgramStep::EachRow {
            source: StepRows::Filled(relation),
            out: Register(0),
        },
        ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: operands([]),
        },
    ];
    LoweredPack {
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
            columns: RuleColumns::default(),
        }],
        registers: RegisterCount(2),
        ..LoweredPack::default()
    }
}

fn subject((relation, width, rows): Input) -> Result<Value, Failure> {
    let programs = Programs(ProgramsEngine {
        code: encoded(&pack(relation)).map_err(|_| ())?,
        ..ProgramsEngine::default()
    });
    let (nodes, mut held) = (tree(), held(2));
    let mut facts = Telling {
        relation,
        width,
        rows,
    };
    let (_counted, found) = framed_with(
        &nodes,
        &mut facts,
        &programs.0,
        &mut held,
        &TheLoop,
        |frame| dispatch(frame, &programs),
    );
    let mut all = Vec::new();
    for notice in &found.notices {
        all.extend([
            u64::from(notice.rule.0),
            match notice.verdict {
                WorkVerdict::Over => 0,
                WorkVerdict::Oversized(read) => 1 + u64::from(read != relation),
            },
            notice.work.0,
            notice.allowance.0,
        ]);
    }
    all.extend([99, u64::try_from(found.rows.len()).unwrap_or(u64::MAX)]);
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
