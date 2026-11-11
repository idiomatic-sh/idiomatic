//! Each operation's presence declaration rests on its kernel: an input computed empty inside a region that runs gives
//! no row.
use super::super::super::fixtures as owner_fixtures;
use super::super::conformance::fixtures::ObservedDelivery;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::SemanticLookup;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::expectations::SOURCE;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::values::KnownSyntax;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::*;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;
use std::sync::Arc;

/// Delivers an operation whose listed inputs have no rows while the region that computes it runs. Each such input is
/// the difference of its authored rows with themselves: its presence is that of the authored rows, which have some, so
/// the operation's kernel is evaluated over an empty input instead of being found absent.
fn compare_emptied(
    operation: &Operation,
    inputs: &[Table],
    emptied: &[usize],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    source: &[u8],
    symbols: &mut Symbols,
    semantic: Option<&dyn SemanticLookup>,
) -> Result<ObservedDelivery, OperatingError> {
    use ryo_values::domain::plan_parts::types::{Input, StepId, StepScope};
    use ryo_values::domain::plans::types::PlanStep;
    let schema = super::super::conformance::test::schema(operation, inputs, bindings);
    let mut authored =
        super::super::conformance::test::author(operation, inputs, bindings, symbols);
    let mut last = authored
        .plan
        .steps
        .pop()
        .expect("the operation under proof is the authored plan's last step");
    let mut sources = Vec::new();
    ryo_values::domain::plans::service::each_input(&mut last.operation, &mut |input| {
        sources.push(*input)
    });
    let mut replaced = Vec::new();
    for (at, source) in sources.iter().enumerate() {
        if emptied.contains(&at) {
            replaced.push(Input::Step(StepId(
                u32::try_from(authored.plan.steps.len()).expect("a step number fits in u32"),
            )));
            authored.plan.steps.push(PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Difference {
                    left: *source,
                    right: *source,
                },
            });
        } else {
            replaced.push(*source);
        }
    }
    let mut at = 0;
    ryo_values::domain::plans::service::each_input(&mut last.operation, &mut |input| {
        *input = replaced[at];
        at += 1;
    });
    authored.plan.steps.push(last);
    authored.plan.emit.step =
        StepId(u32::try_from(authored.plan.steps.len() - 1).expect("a step number fits in u32"));
    super::super::conformance::fixtures::deliver(
        authored,
        schema,
        bindings,
        source,
        symbols,
        None,
        semantic,
        Arc::new(owner_fixtures::operators()),
    )
}

/// Each operation's presence declaration rests on its kernel: a required input with no rows, the others as authored, gives
/// no row; a union of two empty inputs gives none; and a reach join over no edge finds no pair, so closure is not
/// reflexive. Each emptied input is computed empty inside a region that runs, so the kernel answers, not the presence
/// decision.
fn every_required_input_empty_answers_no_rows() -> Result<Value, Failure> {
    use ryo_execution_domain::domain::operator::contract::presence::types::PresenceMask;
    let semantic = KnownSyntax;
    let probe =
        |kind: Option<OperationKind>, case: &mut expectations::Fixture, emptied: Vec<usize>| {
            compare_emptied(
                &case.operation,
                &case.inputs,
                &emptied,
                &case.bindings,
                SOURCE,
                &mut case.symbols,
                Some(&semantic),
            )
            .map(|observed| Probe {
                kind,
                emptied: emptied.into(),
                collected: observed.collected.as_ref().map_or(0, |table| table.count.0),
                refusal: observed.collected.err(),
                streamed: observed.rows.len(),
            })
        };
    let mut probes = vec![];
    let mut lacking = vec![];
    for kind in OperationKind::ALL {
        let presence =
            ryo_execution_domain::domain::operator::contract::service::semantics(*kind).presence;
        let width = fixture(*kind).inputs.len();
        let emptied: Vec<Vec<usize>> = match presence {
            PresenceMask::AllOf(mask) => (0..width)
                .filter(|at| mask.0 & (1 << at) != 0)
                .map(|at| vec![at])
                .collect(),
            PresenceMask::AnyOf(mask) => {
                vec![(0..width).filter(|at| mask.0 & (1 << at) != 0).collect()]
            }
            PresenceMask::Always => continue,
        };
        if emptied.is_empty() {
            lacking.push(*kind);
        }
        for inputs in emptied {
            probes.push(probe(Some(*kind), &mut fixture(*kind), inputs)?);
        }
    }
    let mut reach = fixture(OperationKind::Join);
    if let Operation::Join { reach: lookup, .. } = &mut reach.operation {
        *lookup = Some(ryo_values::domain::plan_parts::types::ReachLookup {
            source: ryo_values::domain::plan_parts::types::ColumnIndex(0),
            destination: ryo_values::domain::plan_parts::types::ColumnIndex(1),
            keys: vec![],
            columns: vec![
                ryo_values::domain::plan_parts::types::ColumnIndex(0),
                ryo_values::domain::plan_parts::types::ColumnIndex(1),
            ],
        });
    }
    probes.push(probe(None, &mut reach, vec![1])?);
    Ok(Emptied {
        probes: probes.into(),
        lacking: lacking.into(),
    })
}
/// One delivery with some inputs computed empty: the kind (`None` for a reach join over no edge), the emptied input
/// positions, and what was collected, refused and streamed.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Probe {
    kind: Option<OperationKind>,
    emptied: Cow<'static, [usize]>,
    collected: usize,
    refusal: Option<OperatingError>,
    streamed: usize,
}
/// Every probe, and the kinds that declare a required input their fixture lacks.
#[derive(Debug, PartialEq, Eq)]
struct Emptied {
    probes: Cow<'static, [Probe]>,
    lacking: Cow<'static, [OperationKind]>,
}
/// A probe that collected and streamed no row.
const fn none(kind: Option<OperationKind>, emptied: &'static [usize]) -> Probe {
    Probe {
        kind,
        emptied: Cow::Borrowed(emptied),
        collected: 0,
        refusal: None,
        streamed: 0,
    }
}
type Input = fn() -> Result<Value, Failure>;
type Value = Emptied;
type Failure = OperatingError;
const PROBES: &[Probe] = {
    use OperationKind::*;
    &[
        none(Some(Replace), &[0]),
        none(Some(OrderedMatch), &[0]),
        none(Some(OrderedMatch), &[1]),
        none(Some(Select), &[0]),
        none(Some(Project), &[0]),
        none(Some(Extend), &[0]),
        none(Some(Join), &[0]),
        none(Some(Join), &[1]),
        none(Some(Union), &[0, 1]),
        none(Some(Intersection), &[0]),
        none(Some(Intersection), &[1]),
        none(Some(Difference), &[0]),
        none(Some(Antijoin), &[0]),
        none(Some(GroupCount), &[0]),
        none(Some(GroupDigest), &[0]),
        none(Some(Capture), &[0]),
        none(Some(Rank), &[0]),
        none(Some(Words), &[0]),
        none(Some(Axis), &[0]),
        none(Some(Closure), &[0]),
        none(Some(Cycles), &[0]),
        none(Some(SpanBetween), &[0]),
        none(Some(SourceMatches), &[0]),
        none(Some(RenderSyntax), &[0]),
        none(Some(FitsRendered), &[0]),
        none(Some(SpanBounds), &[0]),
        none(Some(OrderedDfs), &[0]),
        none(None, &[1]),
    ]
};
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName(stringify!(every_required_input_empty_answers_no_rows)),
    word: Word::Succeeds,
    input: || every_required_input_empty_answers_no_rows,
    expect: Expect::Value(Emptied {
        probes: Cow::Borrowed(PROBES),
        lacking: Cow::Borrowed(&[]),
    }),
}];
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
