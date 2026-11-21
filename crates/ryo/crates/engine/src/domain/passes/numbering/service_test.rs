//! The `numbering` pass's cases: each unit of a pack is numbered from nought in its pre-order, through its inline bodies and
//! across a rule's branches and a fill's pipelines; and a point is its step's own, so a step a later pass moves keeps it.
//! A row says each unit's steps in the order the shared walk meets them (`each_step`; a fill's pipelines in its inputs'
//! order), each as the register it writes (`-` when it writes none) and the point it holds.

use super::service::passed;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::ShapeChoice;
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{
    Branch, ConstantRows, DefinitionPattern, Emptiness, FillInput, GraphFill, GroupFill,
    LoweredPack, LoweredRule, MadeFill, MadeTable, OrderedFill, PackDefinition, PatternForm,
};
use crate::domain::passes::{hoisting, ordering};
use crate::domain::steps::stages::types::{FillStage, Pipeline, PlacedStage};
use crate::domain::steps::traversal::service::each_step;
use crate::domain::steps::types::{
    ColumnList, ColumnMask, DefinitionId, MadeTableId, NearestEdges, PlacedStep, PlanPoint,
    Polarity, ProgramStep, Register, RegisterList, StepBody, StepMemo, StepOperand, StepOperands,
    StepProgram, StepRows,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;
use std::borrow::Cow;

/// The pack a row numbers.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// No definition, rule or table.
    Empty,
    /// One rule of one branch that holds every step with a body: an `Exists` over steps written in place, with an `Exists`
    /// inside it; an `Or` of two inline bodies and a use of a definition; a `Fork` of two slots; a `Nearest` over derived
    /// edges and one over the tree; and a `Closure`.
    Bodies,
    /// A definition of a call form and a table form; a rule of two branches and a rule of one; and a table of each fill, the
    /// rows fill and the ordered match of two pipelines each.
    Units,
    /// A rule of four steps, the second an `Exists` over two: numbered, and then its `Exists` moved to the end of its level,
    /// as a pass that orders steps moves one; then the passes that follow the numbering.
    Permuted,
}

type Input = Scenario;
/// Each unit and its steps, as the module says.
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_pack_has_nothing_to_number"),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("a_unit_is_numbered_in_its_pre_order_through_its_inline_bodies"),
        word: Word::Succeeds,
        input: || Scenario::Bodies,
        expect: Expect::Value(Cow::Borrowed(
            "rule: 1@0 -@1 2@2 -@3 3@4 -@5 4@6 5@7 6@8 -@9 7@10 8@11 9@12 93@13 10@14 11@15 90@16 12@17 94@18 13@19 14@20",
        )),
    },
    Case {
        name: CaseName("each_unit_is_numbered_from_nought_across_its_branches_and_its_pipelines"),
        word: Word::Succeeds,
        input: || Scenario::Units,
        expect: Expect::Value(Cow::Borrowed(
            "pattern: 1@0 2@1; rule: 3@0 4@1 -@2 5@3 6@4; rule: 7@0 8@1; table: 10@0 11@1 12@2; table: 13@0 14@1; table: 15@0; table: 16@0 17@1 18@2; table: 19@0 20@1; table: ",
        )),
    },
    Case {
        name: CaseName("a_permuted_step_keeps_its_point"),
        word: Word::Succeeds,
        input: || Scenario::Permuted,
        expect: Expect::Value(Cow::Borrowed("rule: 1@0 2@4 3@5 -@1 5@2 6@3")),
    },
];

fn set(out: u32) -> ProgramStep {
    ProgramStep::Let {
        from: StepOperand::File,
        out: Register(out),
    }
}

fn program<const N: usize>(steps: [ProgramStep; N]) -> StepProgram {
    StepProgram(Box::new(steps.map(|step| PlacedStep { point: None, step })))
}

fn inline<const N: usize>(steps: [ProgramStep; N]) -> StepBody {
    StepBody::Inline(program(steps))
}

fn exists<const N: usize>(steps: [ProgramStep; N]) -> ProgramStep {
    ProgramStep::Exists {
        polarity: Polarity::Present,
        body: inline(steps),
        memo: StepMemo::None,
    }
}

fn nearest(edges: NearestEdges, condition: u32) -> ProgramStep {
    ProgramStep::Nearest {
        edges,
        condition: inline([set(condition)]),
        element: Register(90),
        start: Register(91),
        out: Register(92),
        memo: StepMemo::None,
    }
}

/// A pipeline of one stage for each register, which the stage writes.
fn pipeline<const N: usize>(outs: [u32; N]) -> Pipeline {
    let stage = |out| PlacedStage {
        point: None,
        stage: FillStage::Product {
            source: StepRows::Made(MadeTableId(0)),
            out: Register(out),
        },
    };
    Pipeline {
        stages: Box::new(outs.map(stage)),
        row: StepOperands::default(),
    }
}

fn input<const N: usize>(outs: [u32; N]) -> FillInput {
    FillInput {
        pipelines: Box::new([pipeline(outs)]),
        keys: ColumnList::default(),
        order: None,
    }
}

fn table(fill: MadeFill) -> MadeTable {
    MadeTable {
        node: NodeDigest::default(),
        fill,
        width: ColumnCount(1),
        indexes: Vec::new(),
        emptiness: Emptiness::Unknown,
    }
}

fn pattern(form: PatternForm) -> DefinitionPattern {
    DefinitionPattern {
        bound: ColumnMask::default(),
        collected: ColumnMask::default(),
        form,
        shape: ShapeChoice::Decided,
    }
}

fn rule<const N: usize>(ordinal: u32, branches: [StepProgram; N]) -> LoweredRule {
    LoweredRule {
        rule: SelectedRuleOrdinal(ordinal),
        node: NodeDigest::default(),
        branches: branches
            .map(|program| Branch {
                emptiness: Emptiness::Unknown,
                program,
            })
            .into(),
        columns: RuleColumns::default(),
    }
}

/// The rule of `Scenario::Bodies`.
fn bodies() -> LoweredPack {
    let used = StepBody::Definition {
        definition: DefinitionId(0),
        bound: ColumnMask::default(),
        collected: ColumnMask::default(),
        values: StepOperands::default(),
    };
    let or = ProgramStep::Or {
        bodies: Box::new([inline([set(4)]), used, inline([set(5), set(6)])]),
        outputs: RegisterList::default(),
        memo: StepMemo::None,
    };
    let fork = ProgramStep::Fork {
        slots: Box::new([program([set(7)]), program([set(8), set(9)])]),
    };
    let derived = NearestEdges::Derived {
        edges: inline([set(10)]),
        from: Register(93),
    };
    let closure = ProgramStep::Closure {
        edges: inline([set(13)]),
        from: Register(94),
        seed: Register(95),
        out: Register(96),
    };
    let steps = [
        set(1),
        exists([set(2), exists([set(3)])]),
        or,
        fork,
        nearest(derived, 11),
        nearest(NearestEdges::Tree(Relation::SyntaxTree), 12),
        closure,
        set(14),
    ];
    LoweredPack {
        rules: vec![rule(0, [program(steps)])],
        ..LoweredPack::default()
    }
}

/// The pack of `Scenario::Units`.
fn units() -> LoweredPack {
    let definition = PackDefinition {
        node: NodeDigest::default(),
        formals: RegisterList::default(),
        emptiness: Emptiness::Unknown,
        patterns: vec![
            pattern(PatternForm::Call(program([set(1), set(2)]))),
            pattern(PatternForm::Table(MadeTableId(0))),
        ],
    };
    let rows = MadeFill::Rows(
        Box::new([pipeline([10, 11]), pipeline([12])]),
        RepeatedRows::Dropped,
    );
    LoweredPack {
        definitions: vec![definition],
        rules: vec![
            rule(
                0,
                [
                    program([set(3)]),
                    program([set(4), exists([set(5)]), set(6)]),
                ],
            ),
            rule(1, [program([set(7), set(8)])]),
        ],
        tables: vec![
            table(rows),
            table(MadeFill::Group(GroupFill::Places, input([13, 14]))),
            table(MadeFill::Ordered(OrderedFill::Sorted(input([15])))),
            table(MadeFill::Ordered(OrderedFill::Match(
                input([16, 17]),
                input([18]),
            ))),
            table(MadeFill::Graph(
                GraphFill::Cycles,
                Box::new([input([19]), input([20])]),
            )),
            table(MadeFill::Constant(ConstantRows::default())),
        ],
        ..LoweredPack::default()
    }
}

/// The pipelines of a fill, in its inputs' order.
fn pipelines(fill: &MadeFill) -> Vec<&Pipeline> {
    let inputs: Vec<&FillInput> = match fill {
        MadeFill::Rows(pipelines, _) => return pipelines.iter().collect(),
        MadeFill::Group(_, input) | MadeFill::Ordered(OrderedFill::Sorted(input)) => vec![input],
        MadeFill::Ordered(OrderedFill::Match(left, right)) => vec![left, right],
        MadeFill::Graph(_, inputs) => inputs.iter().collect(),
        MadeFill::Constant(_) => Vec::new(),
    };
    inputs
        .into_iter()
        .flat_map(|input| &input.pipelines)
        .collect()
}

/// A step or stage said: the register it writes and the point it holds.
fn placed(written: Option<Register>, point: Option<PlanPoint>) -> String {
    let said =
        |number: Option<u32>| number.map_or_else(|| "-".to_owned(), |number| number.to_string());
    format!(
        "{}@{}",
        said(written.map(|register| register.0)),
        said(point.map(|point| point.0))
    )
}

/// A unit said: its name and its steps in the shared walk's order.
fn unit(name: &str, programs: &[&StepProgram]) -> String {
    let mut steps = Vec::new();
    for program in programs {
        each_step(program, &mut |step, touched| {
            steps.push(placed(touched.writes.first().copied(), step.point));
        });
    }
    format!("{name}: {}", steps.join(" "))
}

/// Every unit of a pack said, in the pack's order: its call patterns, its rules, its tables.
fn said(pack: &LoweredPack) -> String {
    let mut units = Vec::new();
    for pattern in pack.definitions.iter().flat_map(|each| &each.patterns) {
        match &pattern.form {
            PatternForm::Call(program) => units.push(unit("pattern", &[program])),
            PatternForm::Table(_) => {}
        }
    }
    for rule in &pack.rules {
        let programs: Vec<&StepProgram> = rule.branches.iter().map(|each| &each.program).collect();
        units.push(unit("rule", &programs));
    }
    for table in &pack.tables {
        let stages = pipelines(&table.fill)
            .into_iter()
            .flat_map(|pipeline| &pipeline.stages);
        let stages: Vec<String> = stages
            .map(|stage| match &stage.stage {
                FillStage::Product { out, .. } => placed(Some(*out), stage.point),
                FillStage::Scan { .. }
                | FillStage::Probe { .. }
                | FillStage::Use { .. }
                | FillStage::Latest { .. }
                | FillStage::Read { .. }
                | FillStage::Keep { .. }
                | FillStage::Call { .. }
                | FillStage::Render { .. }
                | FillStage::Walk { .. } => placed(None, stage.point),
            })
            .collect();
        units.push(format!("table: {}", stages.join(" ")));
    }
    units.join("; ")
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut pack = match scenario {
        Scenario::Empty => LoweredPack::default(),
        Scenario::Bodies => bodies(),
        Scenario::Units => units(),
        Scenario::Permuted => LoweredPack {
            rules: vec![rule(
                0,
                [program([set(1), exists([set(5), set(6)]), set(2), set(3)])],
            )],
            ..LoweredPack::default()
        },
    };
    passed(&mut pack);
    if matches!(scenario, Scenario::Permuted) {
        let level = pack
            .rules
            .first_mut()
            .and_then(|rule| rule.branches.first_mut());
        if let Some(steps) = level.and_then(|branch| branch.program.0.get_mut(1..)) {
            steps.rotate_left(1);
        }
        ordering::service::passed(&mut pack, None);
        hoisting::service::passed(&mut pack);
    }
    Ok(Cow::Owned(said(&pack)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
