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
    /// Same predicates have unrelated bindings, including an unused invalid pattern.
    UnrelatedBindings,
    /// The first class representative is inactive; two other equivalent rules are active.
    FirstInactive,
    /// A single active member leaves the class to that rule's own execution.
    OneActive,
    /// A shared dynamic-pattern error leaves both members to their own operators.
    SharedError,
    /// Missing referenced slots keep independent rule-specific failure attribution.
    MissingReferenced,
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
static NOTHING_HANDED: std::sync::LazyLock<ryo_values::domain::stores::types::ReadHanded> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::ReadHanded::default);
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
    let mut plans = [plan("word"), plan("other"), plan("word"), plan("")];
    let mut bindings = vec![Vec::new(); plans.len()];
    if matches!(
        scenario,
        Scenario::UnrelatedBindings
            | Scenario::FirstInactive
            | Scenario::SharedError
            | Scenario::MissingReferenced
    ) {
        bindings[0] = vec![ParameterBinding::Text(LiteralText("[".into()))];
        bindings[2] = vec![
            ParameterBinding::Text(LiteralText("[".into())),
            ParameterBinding::Numbers(Vec::new()),
        ];
    }
    if matches!(scenario, Scenario::FirstInactive) {
        plans[1] = plan("word");
    }
    if matches!(
        scenario,
        Scenario::SharedError | Scenario::MissingReferenced
    ) {
        for rule in [0, 2] {
            let Operation::Select { comparisons, .. } = &mut plans[rule].steps[0].operation else {
                return Err(());
            };
            comparisons.push(Comparison {
                test: Test::Matches,
                left: Operand::Column(ColumnIndex(2)),
                right: Operand::Parameter(ryo_values::domain::plan_parts::types::ParameterId(
                    if matches!(scenario, Scenario::MissingReferenced) {
                        3
                    } else {
                        0
                    },
                )),
                origin: None,
            });
        }
    }
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
    if matches!(scenario, Scenario::FirstInactive) {
        tables.mask.0 = vec![0b0110];
    }
    if matches!(scenario, Scenario::OneActive) {
        tables.mask.0 = vec![1];
    }
    let ids = ["first", "other", "third", "last"]
        .map(|id| ryo_values::domain::runs::types::RuleId(id.into()));
    let mut setting = setting(&placed, &plans, &bindings);
    setting.ids = &ids;
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
        Scenario::Selected
        | Scenario::Unlowered
        | Scenario::FirstInactive
        | Scenario::OneActive => classes
            .classes
            .iter()
            .map(|each| describe(&each.0))
            .collect::<Vec<_>>()
            .join(" | "),
        Scenario::Alike
        | Scenario::UnrelatedBindings
        | Scenario::SharedError
        | Scenario::MissingReferenced => {
            let mut shown = Vec::new();
            for (rule, each) in plans.iter().enumerate() {
                let mut made = crate::domain::positions::types::LoweredRoom::default();
                install(&tables.shared, &classes, &mut made, RuleIndex(rule));
                let taken =
                    selected(&made, &classes.classes, StepId(0)).map(|chosen| describe(&chosen.0));
                let Operation::Select { comparisons, .. } = &each.steps[0].operation else {
                    return Err(());
                };
                if matches!(scenario, Scenario::MissingReferenced) && matches!(rule, 0 | 2) {
                    let result = crate::domain::reading::service::evaluate_rule(
                        &mut tables,
                        &setting,
                        RuleIndex(rule),
                        FileId(0),
                        None,
                        &|_| None,
                        &classes,
                    );
                    let error =
                        result.expect_err("the member's referenced parameter remains missing");
                    shown.push(format!(
                        "{} / {error}",
                        taken.unwrap_or_else(|| "none".to_owned())
                    ));
                    continue;
                }
                let tree = TreeRows {
                    elements: &tables.elements,
                    file: FileId(0),
                    form: TreeForm::Elements,
                };
                let mut scratch = OperatorScratch::default();
                let mut memo = Vec::new();
                let mut context = OperatorContext {
                    prepared_kinds: None,
                    prepared_matchers: None,
                    stored_indexes: None,
                    bindings: &bindings[rule],
                    symbols: &mut tables.symbols,
                    source: SourceView(SMALL_SOURCE),
                    paths: &[],
                    arenas: &[],
                    memo: &mut memo,
                    scratch: &mut scratch,
                };
                let mut alone = Slots::default();
                let result = select_tree(&tree, comparisons, &mut context, &mut alone);
                let ordinary = if matches!(scenario, Scenario::SharedError) && matches!(rule, 0 | 2)
                {
                    assert!(
                        result.is_err(),
                        "shared failure must not hide the member's own error"
                    );
                    "error".to_owned()
                } else {
                    result.map_err(drop)?;
                    describe(&alone.0)
                };
                shown.push(format!(
                    "{} / {}",
                    taken.unwrap_or_else(|| "none".to_owned()),
                    ordinary
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
        name: CaseName(
            "unused_parameters_share_without_changing_tree_order_or_reading_invalid_patterns",
        ),
        word: Word::Succeeds,
        input: || Scenario::UnrelatedBindings,
        expect: Expect::Value("1,3 / 1,3 ; none / 2 ; 1,3 / 1,3 ; none / 0"),
    },
    Case {
        name: CaseName("inactive_first_representative_does_not_change_shared_predicate_results"),
        word: Word::Succeeds,
        input: || Scenario::FirstInactive,
        expect: Expect::Value("1,3 | "),
    },
    Case {
        name: CaseName("only_one_active_member_keeps_its_original_execution"),
        word: Word::Succeeds,
        input: || Scenario::OneActive,
        expect: Expect::Value(" |  | "),
    },
    Case {
        name: CaseName("a_shared_error_preserves_each_rules_normal_error_path"),
        word: Word::Succeeds,
        input: || Scenario::SharedError,
        expect: Expect::Value("none / error ; none / 2 ; none / error ; none / 0"),
    },
    Case {
        name: CaseName("missing_referenced_bindings_keep_the_owning_rule_in_each_error"),
        word: Word::Succeeds,
        input: || Scenario::MissingReferenced,
        expect: Expect::Value(
            "none / rule `first`: the parameter has no value ; none / 2 ; none / rule `third`: the parameter has no value ; none / 0",
        ),
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
        matchers: None,
        kind_vocabulary: None,
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
