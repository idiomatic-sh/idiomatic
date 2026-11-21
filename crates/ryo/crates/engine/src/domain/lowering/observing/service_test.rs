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
