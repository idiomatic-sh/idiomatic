//! The cases of the `observing` part: a relation the host declares is a constant table a step after the files may join the
//! file's rows with; a row of it that names a file is a row of that file alone; a step after the files that meets two of
//! the file's row sets on no file column, or whose rows name no file, is refused; and a rule that emits a constant table is
//! refused at its emit. Every expected text is what a run printed.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::fixtures::digests_of;
use crate::domain::lowering::flattening::types::FlatAtom;
use crate::domain::lowering::nodes::service::stored;
use crate::domain::lowering::nodes::types::AcceptedPlace;
use crate::domain::lowering::tables::arms::service::rule_flattened;
use crate::domain::lowering::tables::service::admitted_in;
use crate::domain::lowering::tables::types::RuleFlattening;
use crate::domain::lowering::types::{LoadGiven, LoadRefusal, PlanNode, SelectedRule};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Heading, HostedNumber, Input, JoinColumn, Side, StepId, StepScope,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::query_plans::types::{Emit, Plan};
use ryo_values::domain::row_texts::types::Cell;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// What a row asks: a plan, admitted and flattened with the host's relations declared or not.
#[derive(Debug)]
struct Asked {
    /// The plan.
    plan: Plan,
    /// Whether the host declares its relations.
    declared: bool,
}

type Given = Asked;
type Value = Cow<'static, str>;
type Failure = LoadRefusal;

/// A step of `operation` at `scope`.
const fn at(operation: Operation, scope: StepScope) -> PlanStep {
    PlanStep { operation, scope }
}

/// A join of two inputs on the columns `on` names of each, answering `output`.
fn joined(left: Input, right: Input, on: (&[u32], &[u32]), output: &[(Side, u32)], scope: StepScope) -> PlanStep {
    let columns = |places: &[u32]| places.iter().copied().map(ColumnIndex).collect();
    let output = output
        .iter()
        .map(|(side, column)| JoinColumn {
            side: *side,
            column: ColumnIndex(*column),
        })
        .collect();
    at(
        Operation::Join {
            left,
            right,
            left_on: columns(on.0),
            right_on: columns(on.1),
            output,
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        },
        scope,
    )
}

/// The words of a file: parent, child, ordinal, text, file (step 0).
fn words() -> PlanStep {
    joined(
        Input::Relation(Relation::SyntaxChild),
        Input::Relation(Relation::SyntaxTree),
        (&[1], &[0]),
        &[(Side::Left, 0), (Side::Left, 1), (Side::Left, 2), (Side::Right, 2), (Side::Left, 4)],
        StepScope::PerFile,
    )
}

/// A plan of `steps` emitting the last one's rows.
fn plan(steps: Vec<PlanStep>) -> Plan {
    let last = u32::try_from(steps.len().saturating_sub(1)).unwrap_or(u32::MAX);
    Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        steps,
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        emit: Emit {
            step: StepId(last),
            subject: ColumnIndex(0),
            key: vec![ColumnIndex(0)],
            headings: vec![Heading("node".into())],
        },
        frontier: Vec::new(),
    }
}

/// The host's rows read after the files, joined with the words on their text (and their file, for a relation that names
/// one): a word whose text the host names.
fn words_the_host_names(relation: u32) -> Plan {
    let on: (&[u32], &[u32]) = if relation == 0 {
        (&[3], &[0])
    } else {
        (&[3, 4], &[1, 0])
    };
    plan(vec![
        words(),
        at(
            Operation::Hosted {
                relation: HostedNumber(relation),
            },
            StepScope::CrossFile,
        ),
        joined(
            Input::Step(StepId(0)),
            Input::Step(StepId(1)),
            on,
            &[(Side::Left, 1), (Side::Left, 4)],
            StepScope::CrossFile,
        ),
    ])
}

/// The words beside the words, met after the files on `on`, answering both children and the left file.
fn words_met(on: (&[u32], &[u32])) -> Plan {
    plan(vec![
        words(),
        joined(
            Input::Step(StepId(0)),
            Input::Step(StepId(0)),
            on,
            &[(Side::Left, 1), (Side::Right, 1), (Side::Left, 4)],
            StepScope::CrossFile,
        ),
    ])
}

/// The relations the host declares: codes (a text), and codes of a file (a file, a text).
fn hosted() -> Vec<HostRelation> {
    let column = |name: &str, kind| HostColumn {
        name: HostColumnName(name.into()),
        kind,
    };
    let text = |written: &str| Cell::Text(written.into());
    vec![
        HostRelation {
            name: HostName("codes".into()),
            columns: vec![column("code", HostKind::Text)],
            rows: vec![HostRow(vec![text("a")]), HostRow(vec![text("b")])],
        },
        HostRelation {
            name: HostName("codes of a file".into()),
            columns: vec![column("file", HostKind::File), column("code", HostKind::Text)],
            rows: vec![HostRow(vec![Cell::File("x/y.walk".into()), text("a")])],
        },
    ]
}

const fn said(name: &'static str, input: fn() -> Given, value: &'static str) -> Case<Given, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Succeeds,
        input,
        expect: Expect::Value(Cow::Borrowed(value)),
    }
}

const fn refused(name: &'static str, input: fn() -> Given, refusal: LoadRefusal) -> Case<Given, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Refuses,
        input,
        expect: Expect::Error(refusal),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Given, Value, Failure>] = &[
    said(
        "a_relation_the_host_declares_is_a_constant_table_a_step_after_the_files_joins_the_files_words_with",
        || Asked {
            plan: words_the_host_names(0),
            declared: true,
        },
        "locals [Some(File), Some(Constant), Some(File)]\nbranches [Rows Rows Test, Rows Rows Test]\ndefinitions 0",
    ),
    said(
        "a_row_of_a_declared_relation_that_names_a_file_is_a_row_of_that_file_alone",
        || Asked {
            plan: words_the_host_names(1),
            declared: true,
        },
        "locals [Some(File), Some(File), Some(File)]\nbranches [Rows Rows Test Test]\ndefinitions 0",
    ),
    refused(
        "a_relation_the_host_does_not_declare_cannot_be_evaluated",
        || Asked {
            plan: words_the_host_names(0),
            declared: false,
        },
        LoadRefusal::Unlowerable(PlanNode(1)),
    ),
    refused(
        "a_step_after_the_files_that_meets_the_files_words_on_no_file_column_is_refused",
        || Asked {
            plan: words_met((&[3], &[3])),
            declared: true,
        },
        LoadRefusal::Unlowerable(PlanNode(1)),
    ),
    said(
        "a_step_after_the_files_that_meets_the_files_words_keyed_by_the_file_is_a_step_of_the_file",
        || Asked {
            plan: words_met((&[3, 4], &[3, 4])),
            declared: true,
        },
        "locals [Some(File), Some(File)]\nbranches [Rows Rows]\ndefinitions 1",
    ),
    said(
        "a_step_after_the_files_that_meets_the_files_words_keyed_by_a_node_is_a_step_of_the_file",
        || Asked {
            plan: words_met((&[1], &[1])),
            declared: true,
        },
        "locals [Some(File), Some(File)]\nbranches [Rows Rows]\ndefinitions 1",
    ),
    said(
        "a_step_after_the_files_whose_rows_name_no_file_over_the_files_rows_and_a_constant_table_is_a_step_of_the_file",
        || Asked {
            plan: {
                let mut named = words_the_host_names(0);
                if let Some(PlanStep {
                    operation: Operation::Join { output, .. },
                    ..
                }) = named.steps.get_mut(2)
                {
                    output.truncate(1);
                    if let Some(first) = output.first_mut() {
                        first.column = ColumnIndex(3);
                    }
                }
                named
            },
            declared: true,
        },
        "locals [Some(File), Some(Constant), Some(File)]\nbranches [Rows Rows Test, Rows Rows Test]\ndefinitions 0",
    ),
    refused(
        "a_rule_that_emits_a_constant_table_is_refused_at_its_emit",
        || Asked {
            plan: plan(vec![at(
                Operation::Hosted {
                    relation: HostedNumber(0),
                },
                StepScope::CrossFile,
            )]),
            declared: true,
        },
        LoadRefusal::Unlowerable(PlanNode(0)),
    ),
];

/// The plan admitted and flattened: whose rows each step holds, and each branch of the emitted rows by its atoms.
fn subject(asked: Given) -> Result<Value, Failure> {
    let declared = hosted();
    let given = LoadGiven {
        hosted: if asked.declared { &declared } else { &[] },
        observed: &[],
    };
    let digests = digests_of(&asked.plan, RuleBindings(&[]));
    let rule = SelectedRule {
        rule: SelectedRuleOrdinal(0),
        plan: &asked.plan,
        bindings: RuleBindings(&[]),
        digests: &digests,
    };
    let rules = [admitted_in(rule, given)?];
    let store = stored(&rules);
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
        fields: Vec::new(),
        fixed: Vec::new(),
    };
    let mut drafts = Drafts::default();
    let mut state = RuleFlattening {
        rules: &rules,
        store: &store,
        kinds: &kinds,
        place: AcceptedPlace(0),
        flat: Vec::new(),
    };
    let locals = rules.first().map(|rule| format!("{:?}", rule.locals)).unwrap_or_default();
    let flattened = rule_flattened(&mut state, &mut drafts).map_err(|step| {
        LoadRefusal::Unlowerable(PlanNode(step.0))
    })?;
    let branches: Vec<String> = flattened
        .branches
        .iter()
        .map(|branch| {
            let atoms: Vec<&str> = branch
                .atoms
                .iter()
                .map(|atom| match atom {
                    FlatAtom::Rows { .. } => "Rows",
                    FlatAtom::Latest { .. } => "Latest",
                    FlatAtom::Decided { .. } => "Decided",
                    FlatAtom::Walk { .. } => "Walk",
                    FlatAtom::Nearest { .. } => "Nearest",
                    FlatAtom::Closure { .. } => "Closure",
                    FlatAtom::Called { .. } => "Called",
                    FlatAtom::Test { .. } => "Test",
                })
                .collect();
            atoms.join(" ")
        })
        .collect();
    Ok(Cow::Owned(format!(
        "locals {locals}\nbranches [{}]\ndefinitions {}",
        branches.join(", "),
        drafts.definitions.len()
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// What a row of the lowered cases asks: rules lowered together, the host's relations declared, and the bound observation
/// of the last rule fed by the first rule's step named.
#[derive(Debug)]
struct Lowered {
    /// The plans, in rule order.
    plans: Vec<Plan>,
    /// The first rule's step the last rule's binding 0 reads, when it reads one.
    fed: Option<u32>,
    /// How many rows the host's codes relation holds.
    codes: usize,
}

/// The words the host names, counted by child and file after the files, emitted after the files.
fn counted_names() -> Plan {
    let mut named = words_the_host_names(0);
    named.steps.push(at(
        Operation::GroupCount {
            from: Input::Step(StepId(2)),
            keys: vec![ColumnIndex(0), ColumnIndex(1)],
            distinct: None,
        },
        StepScope::CrossFile,
    ));
    named.steps.push(at(
        Operation::Project {
            from: Input::Step(StepId(3)),
            columns: vec![ColumnIndex(0), ColumnIndex(1)],
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
        },
        StepScope::CrossFile,
    ));
    named.emit.step = StepId(4);
    named
}

/// A producer: the words written `a`, with their child, their file and the code `a` (step 1), observed; it emits the words.
fn producer() -> Plan {
    let mut produced = plan(vec![
        words(),
        at(
            Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(1), ColumnIndex(4), ColumnIndex(3)],
                distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
            },
            StepScope::PerFile,
        ),
    ]);
    produced.emit.step = StepId(0);
    produced
}

/// A consumer: the observation's rows whose code the host names, after the files, emitted.
fn consumer() -> Plan {
    use ryo_values::domain::observations::types::{
        BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName,
    };
    use ryo_relations::domain::relations::types::CellKind;
    let column = |name: &str, kind| ObservationColumn {
        name: ObservationColumnName(name.into()),
        kind,
    };
    let mut consuming = plan(vec![
        at(
            Operation::Hosted {
                relation: HostedNumber(0),
            },
            StepScope::CrossFile,
        ),
        joined(
            Input::BoundObservation(ryo_values::domain::plan_parts::types::ObservationBindingId(0)),
            Input::Step(StepId(0)),
            (&[2], &[0]),
            &[(Side::Left, 0), (Side::Left, 1), (Side::Left, 2)],
            StepScope::CrossFile,
        ),
    ]);
    consuming.observation_inputs = vec![BoundObservationInput {
        name: ObservationBindingName("matches".into()),
        columns: vec![
            column("node", CellKind::Node),
            column("file", CellKind::File),
            column("code", CellKind::Symbol),
        ],
        scope: StepScope::CrossFile,
    }];
    consuming
}

const fn lowered_said(name: &'static str, input: fn() -> Lowered, value: &'static str) -> Case<Lowered, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Succeeds,
        input,
        expect: Expect::Value(Cow::Borrowed(value)),
    }
}

/// The rows of the lowered cases.
const LOWERED: &[Case<Lowered, Value, Failure>] = &[
    lowered_said(
        "a_count_after_the_files_of_the_words_a_small_constant_table_names_lowers_and_encodes",
        || Lowered {
            plans: vec![counted_names()],
            fed: None,
            codes: 2,
        },
        "definition 0: 2 columns, emptiness All([All([Unknown, Unknown]), All([])])\ntable 0: Group(Count), 3 columns, indexes []\ntable 1: Rows, 2 columns, indexes []\nrule 0: 1 branches\nencoded\nload: 1 accepted, refused []",
    ),
    lowered_said(
        "a_count_after_the_files_of_the_words_a_large_constant_table_names_lowers_and_encodes",
        || Lowered {
            plans: vec![counted_names()],
            fed: None,
            codes: 500,
        },
        "definition 0: 1 columns, emptiness All([])\ndefinition 1: 2 columns, emptiness All([All([All([Unknown, Unknown]), Definition(DefinitionId(0))]), Any([All([Definition(DefinitionId(0))])])])\ntable 0: Group(Count), 3 columns, indexes []\ntable 1: Rows, 2 columns, indexes []\nrule 0: 1 branches\nencoded\nload: 1 accepted, refused []",
    ),
    lowered_said(
        "a_bound_observation_is_one_definition_of_its_producers_observed_steps",
        || Lowered {
            plans: vec![producer(), consumer()],
            fed: Some(1),
            codes: 2,
        },
        "definition 0: 5 columns, emptiness All([Unknown, Unknown])\ndefinition 1: 3 columns, emptiness All([Definition(DefinitionId(0)), Any([All([Definition(DefinitionId(0))])])])\ndefinition 2: 3 columns, emptiness All([Any([Definition(DefinitionId(1))]), Any([All([Definition(DefinitionId(1))])])])\ntable 0: Rows, 5 columns, indexes [[3]]\nrule 0: 1 branches\nrule 1: 3 branches\nencoded\nload: 2 accepted, refused []",
    ),
];

/// The rules lowered and encoded: each refused rule with why, and the pack's lines.
fn lowered_subject(asked: Lowered) -> Result<Value, Failure> {
    use crate::domain::encoding::service::encoded;
    use crate::domain::lowering::fixtures::described::described;
    use crate::domain::lowering::service::lowered;
    use crate::domain::lowering::types::{BoundProducer, ObservedBinding, SelectedRules};
    use ryo_values::domain::plan_parts::types::ObservationBindingId;
    use ryo_values::domain::query_plans::types::NodeDigest;
    use crate::domain::programs::service::accepted;
    use crate::domain::statistics::types::StatisticsBytes;
    use crate::inbound::use_cases::load::fixtures::kinds as load_kinds;
    use crate::inbound::use_cases::load::interface::LoadUseCase as _;
    use crate::inbound::use_cases::load::types::LoadSource;
    use crate::inbound::use_cases::load::use_case::LoadUseCaseImpl;
    let mut declared = hosted();
    if let Some(codes) = declared.first_mut() {
        codes.rows = (0..asked.codes)
            .map(|code| HostRow(vec![Cell::Text(format!("c{code}").into())]))
            .chain([HostRow(vec![Cell::Text("a".into())])])
            .collect();
    }
    let last = u32::try_from(asked.plans.len().saturating_sub(1)).unwrap_or(u32::MAX);
    let observed: Vec<ObservedBinding> = asked
        .fed
        .map(|step| ObservedBinding {
            consumer: SelectedRuleOrdinal(last),
            binding: ObservationBindingId(0),
            node: NodeDigest([9; 16]),
            producers: vec![BoundProducer {
                rule: SelectedRuleOrdinal(0),
                step: StepId(step),
            }],
        })
        .into_iter()
        .collect();
    let given = LoadGiven {
        hosted: &declared,
        observed: &observed,
    };
    let digests: Vec<_> = asked
        .plans
        .iter()
        .map(|plan| digests_of(plan, RuleBindings(&[])))
        .collect();
    let rules: Vec<SelectedRule<'_>> = asked
        .plans
        .iter()
        .zip(&digests)
        .zip(0..)
        .map(|((plan, digests), rule)| SelectedRule {
            rule: SelectedRuleOrdinal(rule),
            plan,
            bindings: RuleBindings(&[]),
            digests,
        })
        .collect();
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
        fields: Vec::new(),
        fixed: Vec::new(),
    };
    let lowered = lowered(SelectedRules(&rules), given, None, &kinds);
    let mut lines: Vec<String> = lowered
        .refused
        .iter()
        .map(|refused| format!("refused rule {}: {}", refused.rule.0, refused.refusal))
        .collect();
    lines.extend(described(&lowered.pack).into_iter().filter(|line| !line.starts_with("  ")));
    let written = encoded(&lowered.pack).map_or_else(|error| format!("encoding refused: {error:?}"), |_| "encoded".to_owned());
    lines.push(written);
    // The load's own sequence: lowered, passed, encoded, and every unit asked whether the executor runs it.
    let source = LoadSource::Plans(SelectedRules(&rules), StatisticsBytes(&[]), given);
    let loaded = LoadUseCaseImpl.load(source, &load_kinds());
    lines.push(match loaded {
        Ok(loaded) => format!(
            "load: {} accepted, refused {:?}",
            accepted(&loaded.programs).len(),
            loaded.refused.iter().map(|refused| (refused.rule.0, refused.refusal.to_string())).collect::<Vec<_>>()
        ),
        Err(error) => format!("load error: {error:?}"),
    });
    Ok(Cow::Owned(lines.join("\n")))
}

#[test]
fn lowered_succeeds() {
    run(LOWERED, Word::Succeeds, lowered_subject);
}

/// The words joined after the files with written rows of `codes` texts, `a` among them, on their text: each word `a`.
fn words_written_among(codes: usize) -> Plan {
    use ryo_values::domain::plan_parts::types::{LiteralText, TableKey};
    use ryo_values::domain::written_rows::types::{RowsSource, WrittenRow, WrittenRows};
    let mut written: Vec<WrittenRow> = (0..codes)
        .map(|code| WrittenRow(vec![LiteralText(format!("c{code}").into())]))
        .collect();
    written.push(WrittenRow(vec![LiteralText("a".into())]));
    plan(vec![
        words(),
        at(
            Operation::Rows {
                source: RowsSource::Written(WrittenRows(written)),
                keys: vec![TableKey("code".into())],
            },
            StepScope::CrossFile,
        ),
        joined(
            Input::Step(StepId(0)),
            Input::Step(StepId(1)),
            (&[3], &[0]),
            &[(Side::Left, 1), (Side::Left, 4)],
            StepScope::CrossFile,
        ),
    ])
}

/// The plan loaded and its file `a b a` scanned: the rows the rule finds, and the pack's lines.
fn scanned_subject(plans: Vec<Plan>) -> Result<Value, Failure> {
    use crate::domain::lowering::fixtures::described::described;
    use crate::domain::lowering::types::SelectedRules;
    use crate::domain::statistics::types::StatisticsBytes;
    use crate::inbound::use_cases::load::fixtures::kinds as load_kinds;
    use crate::inbound::use_cases::load::interface::LoadUseCase as _;
    use crate::inbound::use_cases::load::types::LoadSource;
    use crate::inbound::use_cases::load::use_case::LoadUseCaseImpl;
    use crate::inbound::use_cases::scan::fixtures::{rule_rows, scanned};
    let digests: Vec<_> = plans.iter().map(|plan| digests_of(plan, RuleBindings(&[]))).collect();
    let rules: Vec<SelectedRule<'_>> = plans
        .iter()
        .zip(&digests)
        .zip(0..)
        .map(|((plan, digests), rule)| SelectedRule {
            rule: SelectedRuleOrdinal(rule),
            plan,
            bindings: RuleBindings(&[]),
            digests,
        })
        .collect();
    let last = u32::try_from(plans.len().saturating_sub(1)).unwrap_or(u32::MAX);
    let kinds = load_kinds();
    let pack = crate::domain::lowering::service::lowered(SelectedRules(&rules), LoadGiven::default(), None, &kinds);
    let source = LoadSource::Plans(SelectedRules(&rules), StatisticsBytes(&[]), LoadGiven::default());
    let loaded = LoadUseCaseImpl
        .load(source, &kinds)
        .map_err(|_| LoadRefusal::Unlowerable(PlanNode(u32::MAX)))?;
    let scan = scanned(&loaded.programs, "a b a").ok().flatten();
    let rows = scan.as_ref().map(|scan| rule_rows(scan, last));
    let mut lines = vec![format!("refused {:?}, rows {rows:?}", loaded.refused)];
    lines.extend(described(&pack.pack));
    Ok(Cow::Owned(lines.join("\n")))
}

const fn scanned_said(name: &'static str, input: fn() -> Vec<Plan>, value: &'static str) -> Case<Vec<Plan>, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Succeeds,
        input,
        expect: Expect::Value(Cow::Borrowed(value)),
    }
}

/// The rows of the scanned cases.
const SCANNED: &[Case<Vec<Plan>, Value, Failure>] = &[
    scanned_said(
        "the_files_words_joined_with_a_few_written_rows_find_each_word_a_row_names",
        || vec![words_written_among(3)],
        "refused [], rows Some(2)\nrule 0: 4 branches\n  branch: EachMatch:File Exists:Constant Emit:Constant\n  branch: EachMatch:File Exists:Constant Emit:Constant\n  branch: EachMatch:File Exists:Constant Emit:Constant\n  branch: EachMatch:File Exists:Constant Emit:Constant",
    ),
    scanned_said(
        "the_files_words_joined_with_many_written_rows_find_each_word_a_row_names",
        || vec![words_written_among(40)],
        "refused [], rows Some(2)\ndefinition 0: 1 columns, emptiness All([])\n  pattern bound [0] collected []: Call: Fork:Constant{Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant | Test:Constant Yield:Constant}\nrule 0: 1 branches\n  branch: EachRow:File EachMatch:Fanout Read:Constant Exists:Fanout Emit:Constant",
    ),
    scanned_said(
        "beside_another_rule_the_files_words_joined_with_many_written_rows_find_each_word_a_row_names",
        || vec![plan(vec![words()]), words_written_among(40)],
        "refused [], rows Some(2)\ndefinition 0: 5 columns, emptiness All([Unknown, Unknown])\n  pattern bound [] collected []: Table 0\n  pattern bound [3] collected [1]: Table 0\ndefinition 1: 1 columns, emptiness All([])\n  pattern bound [] collected []: Table 1\ntable 0: Rows, 5 columns, indexes [[3]]\n  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\ntable 1: Constant, 1 columns, indexes []\nrule 0: 1 branches\n  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\nrule 1: 1 branches\n  branch: EachRow:File Read:Constant Or:Fanout Emit:Constant",
    ),
];

#[test]
fn scanned_succeeds() {
    run(SCANNED, Word::Succeeds, scanned_subject);
}
