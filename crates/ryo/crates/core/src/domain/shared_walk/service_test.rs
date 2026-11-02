//! The `shared_walk` service's cases, over the small tree: the pass selects the elements each step would select on its own.

use super::service::{install, walk};
use super::types::Walking;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::lowering::types::Counting;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::types::TreeSteps;
use crate::domain::placement::types::{PlacedScopes, ScopePlacement};
use crate::domain::positions::service::selected;
use crate::domain::positions::service_test::{SMALL_SOURCE, SmallTree, small_tree};
use crate::domain::positions::types::Slots;
use crate::domain::render::types::Answering;
use crate::domain::selections::service::select_tree;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::types::ParameterBinding;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use crate::domain::visit::types::{Noting, RoleRead};
use crate::domain::workers::service::tables_for;
use crate::domain::workers::types::{Replaying, Tables, VisitSetting};
use crate::outbound::matching::fixtures::fake::MatchingFake;
use crate::outbound::matching::interface::MatchingPort as _;
use ryo_language::domain::relations::types::{FileId, Relation, WorkerNumber};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input as PlanInput, LiteralText, Operand, QueryLiteral, StepId, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};
use ryo_values::domain::runs::types::MemoryBytes;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Writes which elements the pass selected for each class.
    Selected,
    /// Writes which elements each rule's step takes from the pass, then which the step selects on its own.
    Alike,
    /// Runs the pass on steps that are run as operators.
    Unlowered,
}

/// What a row of the cases does.
type Input = Scenario;
/// What the case shows, as text.
type Value = &'static str;
type Failure = ();

static NOTHING_STORED: std::sync::LazyLock<ryo_values::domain::stores::types::Stored> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::Stored::default);
static NOTHING_HANDED: std::sync::LazyLock<ryo_values::domain::stores::types::StoredHanded> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::StoredHanded::default);
static NOTHING_WANTED: ryo_values::domain::stores::types::BucketsWanted =
    ryo_values::domain::stores::types::BucketsWanted::These(Vec::new());

fn text_is(text: &str) -> Comparison {
    Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: None,
    }
}

fn kind_is(name: &str) -> Comparison {
    Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(1)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(name.into()))),
        origin: None,
    }
}

/// A plan whose one step selects the elements of the kind `name` from the tree, or, when `name` is empty, those with no text.
fn plan(name: &str) -> Plan {
    Plan {
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::SyntaxTree),
                comparisons: vec![if name.is_empty() {
                    text_is("")
                } else {
                    kind_is(name)
                }],
                kept: Vec::new(),
            },
            scope: StepScope::PerFile,
        }],
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

fn setting<'a>(
    placed: &'a PlacedScopes,
    plans: &'a [Plan],
    bindings: &'a [Vec<ParameterBinding>],
) -> VisitSetting<'a> {
    VisitSetting {
        unknown: &[],
        plans,
        bindings,
        wanted: &[],
        paths: &[],
        first_bucket: &[],
        ids: &[],
        applicability: &[],
        applicability_can_gate: true,
        placed,
        records: &[],
        replaying: Replaying::Findings,
        stored: &NOTHING_STORED,
        handed: &NOTHING_HANDED,
        wanted_buckets: &NOTHING_WANTED,
        budget: MemoryBytes(0),
        noted: Noting::Unwanted,
        roled: RoleRead::Skips,
        needs: &[],
        answering: &Answering::Findings,
        keeping: crate::domain::visit::types::Keeping::Keeps,
    }
}

fn describe(elements: &[u32]) -> String {
    elements
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let plans = [plan("word"), plan("other"), plan("word"), plan("")];
    let bindings = vec![Vec::new(); plans.len()];
    let placed = PlacedScopes {
        placement: ScopePlacement::default(),
        matcher: MatchingFake.compile(&[]).map_err(drop)?,
    };
    let unlowered = if let Scenario::Unlowered = scenario {
        TreeSteps::AsOperators
    } else {
        TreeSteps::Lowered
    };
    let mut tables = tables_for(WorkerNumber(0), &plans, unlowered, Counting::DoesNotCount);
    let SmallTree {
        elements: small_elements,
        symbols: small_symbols,
    } = small_tree();
    tables.elements = small_elements;
    tables.symbols = small_symbols;
    tables.bytes.0 = SMALL_SOURCE.to_vec();
    tables.mask.0 = vec![u64::MAX];
    let setting = setting(&placed, &plans, &bindings);
    let mut classes = super::types::SharedClasses::default();
    {
        let mut scratch = OperatorScratch::default();
        let Tables {
            mask,
            elements,
            symbols,
            bytes,
            shared,
            pool,
            ..
        } = &mut tables;
        walk(
            shared,
            Walking {
                plans: setting.plans,
                bindings: setting.bindings,
                paths: setting.paths,
                mask,
                file: FileId(0),
                elements,
                symbols,
                source: SourceView(&bytes.0),
                scratch: &mut scratch,
                room: &mut classes,
                steps: pool.unlowered,
            },
        );
    }
    let shown = match scenario {
        Scenario::Selected | Scenario::Unlowered => classes
            .classes
            .iter()
            .map(|each| describe(&each.0))
            .collect::<Vec<_>>()
            .join(" | "),
        Scenario::Alike => {
            let mut shown = Vec::new();
            for (rule, each) in plans.iter().enumerate() {
                let mut made = crate::domain::positions::types::LoweredRoom::default();
                install(&tables.shared, &classes, &mut made, RuleIndex(rule));
                let taken =
                    selected(&made, &classes.classes, StepId(0)).map(|chosen| describe(&chosen.0));
                let Operation::Select { comparisons, .. } = &each.steps[0].operation else {
                    return Err(());
                };
                let tree = TreeRows {
                    elements: &tables.elements,
                    file: FileId(0),
                    form: TreeForm::Elements,
                };
                let mut scratch = OperatorScratch::default();
                let mut memo = Vec::new();
                let mut context = OperatorContext {
                    stored_indexes: None,
                    bindings: &[],
                    symbols: &mut tables.symbols,
                    source: SourceView(SMALL_SOURCE),
                    paths: &[],
                    arenas: &[],
                    memo: &mut memo,
                    scratch: &mut scratch,
                };
                let mut alone = Slots::default();
                select_tree(&tree, comparisons, &mut context, &mut alone).map_err(drop)?;
                shown.push(format!(
                    "{} / {}",
                    taken.unwrap_or_else(|| "none".to_owned()),
                    describe(&alone.0)
                ));
            }
            shown.join(" ; ")
        }
    };
    Ok(Box::leak(shown.into_boxed_str()))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_pass_selects_the_elements_of_each_class_in_the_order_of_the_tree"),
        word: Word::Succeeds,
        input: || Scenario::Selected,
        expect: Expect::Value("1,3 |  | "),
    },
    Case {
        name: CaseName("a_step_takes_what_it_would_select_on_its_own"),
        word: Word::Succeeds,
        input: || Scenario::Alike,
        expect: Expect::Value("1,3 / 1,3 ; none / 2 ; 1,3 / 1,3 ; none / 0"),
    },
    Case {
        name: CaseName("steps_run_as_operators_are_left_to_their_operators"),
        word: Word::Succeeds,
        input: || Scenario::Unlowered,
        expect: Expect::Value(""),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn native_file_outputs_drop_before_an_empty_next_generation() {
    use crate::domain::positions::types::Slots;
    use crate::domain::shared_walk::types::{MergedWalk, SharedClasses};
    let mut shared = MergedWalk::default();
    let mut classes = SharedClasses {
        classes: vec![Slots(vec![0; 4096])],
        merged: vec![super::types::Merged::Together],
    };
    let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mask = crate::domain::placement::types::Mask::default();
    let mut scratch = OperatorScratch::default();
    walk(
        &mut shared,
        Walking {
            plans: &[],
            bindings: &[],
            paths: &[],
            mask: &mask,
            file: FileId(1),
            elements: &elements,
            symbols: &mut symbols,
            source: SourceView(&[]),
            scratch: &mut scratch,
            room: &mut classes,
            steps: TreeSteps::Lowered,
        },
    );
    assert!(classes.classes.is_empty());
    assert_eq!(classes.classes.capacity(), 0);
    assert!(classes.merged.is_empty());
}
