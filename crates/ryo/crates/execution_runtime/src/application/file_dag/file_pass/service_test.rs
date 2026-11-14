//! The roots of a planned program in one file: a select of the tree by a kind starts from that kind's nodes in tree order, a
//! reader of the tree with no root kind from every node, and a kind the file does not name from none.

use super::service::{file_roots, fill_roots, roots_of};
use ryo_execution_domain::domain::fused_program::service::{assemble, logical};
use ryo_execution_domain::domain::fused_program::types::program_inputs::value::ProgramInputs;
use ryo_execution_domain::domain::fused_program::types::{RuleBindings, RuleInput, RuleObservations};
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::index_kinds;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_operators::domain::table_texts::types::Texts;
use ryo_operators::inbound::use_cases::plan::interface::PlanUseCase as _;
use ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
use ryo_relations::domain::relations::service::node_position;
use ryo_relations::domain::relations::types::{FileId, KindBits, Offset, Relation};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, LiteralText, Operand, PlanComparison, QueryLiteral,
    RepeatedRows, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use std::borrow::Cow;
use std::sync::Arc;

/// The kind the select of the tree names.
type Input = &'static str;
/// The positions each of the two nodes starts from, as text.
type Value = Cow<'static, str>;
/// Why a scenario is refused: the program did not plan.
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_select_by_a_kind_starts_from_its_nodes_and_a_plain_reader_from_all"),
        word: Word::Succeeds,
        input: || "word",
        expect: Expect::Value(Cow::Borrowed("[1, 3] [0, 1, 2, 3]")),
    },
    Case {
        name: CaseName("a_kind_the_file_does_not_name_starts_from_none"),
        word: Word::Succeeds,
        input: || "absent",
        expect: Expect::Value(Cow::Borrowed("[] [0, 1, 2, 3]")),
    },
];

/// The tree of `abc`: a `file` node over a `word`, an `other` and a `word`, indexed by kind.
fn tree() -> (Elements, Symbols) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"abc"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    for (bits, start) in [(2, 0), (3, 1), (2, 2)] {
        token(&mut builder, KindBits(bits), Offset(start), Offset(start + 1));
    }
    leave_node(&mut builder, Offset(3));
    for (bits, name) in [(1, "file"), (2, "word"), (3, "other")] {
        let mut names = completion(&mut elements, &mut symbols, ByteSlice(b"abc"), FileId(0));
        name_kind(&mut names, KindBits(bits), NameText(name));
    }
    index_kinds(&mut elements);
    (elements, symbols)
}

/// A one-step rule on each file.
fn rule(operation: Operation) -> Arc<Plan> {
    Arc::new(Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        frontier: Vec::new(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        steps: vec![PlanStep {
            operation,
            scope: StepScope::PerFile,
        }],
    })
}

fn subject(kind: Input) -> Result<Value, Failure> {
    let by_kind = rule(Operation::Select {
        from: StepInput::Relation(Relation::SyntaxTree),
        comparisons: vec![PlanComparison {
            test: Test::Equal,
            left: Operand::Column(ColumnIndex(1)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(kind.into()))),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: Vec::new(),
    });
    let plain = rule(Operation::Project {
        from: StepInput::Relation(Relation::SyntaxChild),
        columns: vec![ColumnIndex(1)],
        distinct: RepeatedRows::Kept,
    });
    let rules = [&by_kind, &plain].map(|plan| RuleInput {
        plan,
        bindings: RuleBindings(&[]),
        observations: RuleObservations(&[]),
    });
    let mut inputs = ProgramInputs::default();
    let build = logical(&rules, &mut inputs).map_err(|_| ())?;
    let planned = PlanUseCaseImpl.plan(build.program()).map_err(|_| ())?;
    let program = assemble(build, planned);
    let (elements, symbols) = tree();
    let texts = Texts {
        symbols: &symbols,
        source: SourceView(b"abc"),
        paths: &[],
        arenas: &[],
    };
    let mut roots = file_roots(&program);
    fill_roots(&mut roots, &elements, &texts, FileId(0));
    let at = |node| -> Vec<u32> { roots_of(&roots, PlanNodeId(node)).0.iter().map(|id| node_position(*id).0).collect() };
    Ok(Cow::Owned(format!("{:?} {:?}", at(0), at(1))))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
