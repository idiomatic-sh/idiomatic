//! The `driver` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::super::driver::types::LeaseDriver;
use super::super::failure::{types::FailureLatch, types::LatchState};
use super::super::primitive::types::PreparedInputs;
use super::super::types::{FoundRows, PrimitiveLease, RowSink};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::domain::contract::types::PortableOperatorFault as Fault;
use ryo_operators::domain::contract::types::{
    FirstOccurrence, GroupSummary, PredicateOutcome, RenderKind, RightColumns, RowRead, TextFound,
    TextScratch, VertexPlace,
};
use ryo_operators::domain::evaluation::types::{EvaluationInput, EvaluationViews};
use ryo_relations::domain::relations::types::{Count, Integer, NodeId, Value};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::{RowCount, RowIndex};
/// Keeps the first engine error, and answers every later call as interrupted.
pub(crate) fn hold<'a, 'f, T>(
    driver: &mut LeaseDriver<'a, 'f>,
    result: Result<T, OperatingError>,
) -> Result<T, Fault> {
    result.map_err(|error| {
        crate::domain::operator::contract::failure::service::record(&mut driver.failure, error);
        Fault::Interrupted
    })
}

/// Whether an earlier engine error interrupted the evaluation: the latch is `Failed` once one was kept.
pub(crate) fn interrupted<'a, 'f>(driver: &LeaseDriver<'a, 'f>) -> LatchState {
    crate::domain::operator::contract::failure::service::failed(&driver.failure)
}

/// The scheduled left row of a row-mode evaluation.
pub(crate) fn left<'a, 'f>(driver: &LeaseDriver<'a, 'f>) -> &'a [Value] {
    match driver.inputs {
        PreparedInputs::Row { row, .. } => row,
        PreparedInputs::Views(_) | PreparedInputs::RequestSource(_) => &[],
    }
}

/// One population of a population-mode evaluation.
pub(crate) fn borrowed<'a, 'f>(
    driver: &LeaseDriver<'a, 'f>,
    input: InputNumber,
) -> Option<crate::domain::table_model::types::RepresentationInput<'a>> {
    match driver.inputs {
        PreparedInputs::Views(views) => views.get(input.0).copied(),
        PreparedInputs::RequestSource(source) if input.0 == 0 => Some(*source),
        PreparedInputs::RequestSource(_) | PreparedInputs::Row { .. } => None,
    }
}

/// Evaluates one operation through the operators' one meaning of its kind: the engine lends its views, its
/// scratch and this driver, and keeps the first engine error ahead of any portable answer.
pub(crate) fn evaluate(
    operation: &Operation,
    inputs: PreparedInputs<'_>,
    lease: &mut PrimitiveLease<'_>,
    sink: &mut dyn RowSink,
) -> Result<(), OperatingError> {
    let semantics = crate::domain::operator::contract::lease::service::semantics(lease);
    let resources = lease.views.resources;
    let views = EvaluationViews {
        source: ryo_operators::domain::source_matches::types::LineBytes(lease.views.source.0),
        bindings: lease.views.bindings,
        regex: lease.views.regex.into(),
        checked: lease
            .views
            .validated_source
            .map(
                |checked| ryo_operators::domain::source_matches::types::CheckedSource {
                    base: checked.base,
                    text: checked.text,
                    line: checked.line,
                },
            )
            .into(),
        appended: resources.appended().into(),
        layers: resources.rank_layers().into(),
        cached_reach: lease
            .views
            .cached_reach
            .map(
                |reach| ryo_operators::domain::equality_join::types::CachedReach {
                    request: reach.request,
                    destinations: reach.destinations,
                },
            )
            .into(),
        cached_axis: lease.views.cached_axis.map(|axis| axis.destinations).into(),
    };
    let input = match inputs {
        PreparedInputs::Row { row, .. } => EvaluationInput::Row(row),
        PreparedInputs::Views(_) | PreparedInputs::RequestSource(_) => EvaluationInput::Populations,
    };
    let mut scratch = std::mem::take(&mut lease.workspace.evaluation);
    let positions = std::mem::take(&mut lease.workspace.positions);
    let mut driver = LeaseDriver {
        lease,
        sink,
        operation,
        inputs,
        resources,
        failure: FailureLatch::default(),
        join: crate::domain::operator::contract::candidates::service::join_cursor(
            resources, operation, positions,
        ),
        axis: super::super::views::types::AxisCursor::default(),
        graph: super::super::views::types::GraphCursor::default(),
        rendered: Vec::new(),
        left_buffer: Vec::new(),
        right_buffer: Vec::new(),
    };
    let answer = semantics.operators(
        ryo_operators::inbound::use_cases::operators::types::OperatorCall::Evaluate {
            operation,
            input,
            views,
            driver: &mut driver,
            scratch: &mut scratch,
        },
    );
    let latched = crate::domain::operator::contract::failure::service::take(&mut driver.failure);
    let refusal = match answer {
        Err(Fault::MissingCapture) if latched.is_none() => missing_capture(operation),
        Err(Fault::UnsupportedRender) if latched.is_none() => {
            Some(unsupported_render(&driver.rendered))
        }
        _ => None,
    };
    let LeaseDriver { lease, join, .. } = driver;
    lease.workspace.positions =
        crate::domain::operator::contract::candidates::service::give_back(join);
    lease.workspace.evaluation = scratch;
    if let Some(refusal) = refusal {
        return Err(refusal);
    }
    crate::domain::operating::service::written_result(answer, latched)
}

/// A named capture the matcher does not have is an invalid regular expression of the operation.
pub(super) fn missing_capture(operation: &Operation) -> Option<OperatingError> {
    let Operation::SourceMatches { capture, .. } = operation else {
        return None;
    };
    Some(OperatingError::InvalidRegex {
        reason: format!(
            "missing named capture `{}`",
            capture.as_ref().map(|name| &*name.0).unwrap_or_default()
        ),
    })
}

/// A provider's unsupported answer invalidates the consuming rule, with the provider's reason.
pub(super) fn unsupported_render(
    answers: &[ryo_language::domain::semantics::types::RenderOutcome],
) -> OperatingError {
    use ryo_language::domain::semantics::types::RenderOutcome;
    let reason = answers.iter().find_map(|answer| match answer {
        RenderOutcome::Unsupported(reason) => Some(reason.0),
        RenderOutcome::Known(_) | RenderOutcome::NotApplicable => None,
    });
    OperatingError::SemanticIncomplete {
        reason: format!(
            "selected language cannot render this syntax (reason {})",
            reason.map_or_else(String::new, |reason| reason.to_string())
        ),
    }
}

pub(crate) fn emit(driver: &mut LeaseDriver<'_, '_>, row: &[Value]) -> Result<(), Fault> {
    if interrupted(driver) == LatchState::Failed {
        return Err(Fault::Interrupted);
    }
    let result = driver.sink.row(driver.lease, row);
    hold(driver, result)
}

pub(crate) fn holds(
    driver: &mut LeaseDriver<'_, '_>,
    row: &[Value],
) -> Result<PredicateOutcome, Fault> {
    if interrupted(driver) == LatchState::Failed {
        return Err(Fault::Interrupted);
    }
    let verdict = crate::domain::operator::contract::lease::service::holds(driver.lease, row);
    hold(driver, verdict).map(|verdict| {
        if verdict == Verdict::Fails {
            PredicateOutcome::Fails
        } else {
            PredicateOutcome::Holds
        }
    })
}

pub(crate) fn first_time(
    driver: &mut LeaseDriver<'_, '_>,
    row: &[Value],
) -> Result<FirstOccurrence, Fault> {
    if interrupted(driver) == LatchState::Failed {
        return Err(Fault::Interrupted);
    }
    let added =
        crate::domain::operator::contract::lease::service::distinct(driver.lease).map(|state| {
            crate::domain::table_dedup::service::first_time(&mut state.rows, &mut state.seen, row)
                == crate::domain::table_model::types::Addition::Added
        });
    hold(driver, added).map(FirstOccurrence::from)
}

pub(crate) fn found(driver: &mut LeaseDriver<'_, '_>, input: InputNumber, rows: Count) {
    crate::domain::operator::contract::lease::service::found(
        driver.lease,
        input,
        FoundRows(u64::try_from(rows.0).unwrap_or(u64::MAX)),
    );
}

pub(crate) fn text(
    driver: &mut LeaseDriver<'_, '_>,
    value: Value,
    target: &mut TextScratch,
) -> TextFound {
    let texts = Texts {
        symbols: driver.lease.derived.as_ref(),
        source: driver.lease.views.source,
        paths: driver.lease.views.paths,
        arenas: driver.lease.views.arenas,
    };
    match datum(value, &texts) {
        Datum::Text(text) => {
            target.0.push_str(text.0);
            TextFound::Text
        }
        Datum::Unlocated
        | Datum::Number(_)
        | Datum::Flag(_)
        | Datum::Numbers(_)
        | Datum::Texts(_) => TextFound::NotText,
    }
}

pub(crate) fn mint(driver: &mut LeaseDriver<'_, '_>, text: CellText<'_>) -> Value {
    let made = query_intern_derived(
        &mut driver.lease.derived,
        ByteSlice(driver.lease.views.source.0),
        NameText(text.0),
    );
    crate::domain::operating::service::minted(
        driver.lease.derived.as_ref(),
        made,
        driver.lease.views.phase,
    )
}

pub(crate) fn population(
    driver: &mut LeaseDriver<'_, '_>,
    input: InputNumber,
) -> Result<RowCount, Fault> {
    let population = borrowed(driver, input)
        .map(|rows| rows.count())
        .ok_or(OperatingError::NotRun);
    hold(driver, population)
}

pub(crate) fn population_row(
    driver: &mut LeaseDriver<'_, '_>,
    input: InputNumber,
    at: RowIndex,
    row: &mut Vec<Value>,
) -> RowRead {
    let Some(rows) = borrowed(driver, input) else {
        return RowRead::Missing;
    };
    match rows.read(at, &mut driver.left_buffer) {
        Some(values) => {
            row.clear();
            row.extend_from_slice(values);
            RowRead::Read
        }
        None => RowRead::Missing,
    }
}

pub(crate) fn groups(driver: &mut LeaseDriver<'_, '_>) -> Result<Count, Fault> {
    let groups = driver
        .resources
        .groups()
        .map(|groups| Count(groups.len()))
        .ok_or(OperatingError::NotRun);
    hold(driver, groups)
}

pub(crate) fn group(driver: &LeaseDriver<'_, '_>, at: VertexPlace) -> Option<GroupSummary> {
    driver
        .resources
        .groups()?
        .get(at.0)
        .map(|group| GroupSummary {
            first: group.first,
            counted: Count(group.counted.0),
            digest: Integer(group.digest.0),
        })
}

pub(crate) fn render(driver: &mut LeaseDriver<'_, '_>, nodes: &[NodeId]) -> Result<(), Fault> {
    let Operation::RenderSyntax { context, .. } = driver.operation else {
        return hold(driver, Err(OperatingError::WrongOperation));
    };
    let requests: Vec<ryo_language::domain::semantics::types::RenderRequest> = nodes
        .iter()
        .map(
            |node| ryo_language::domain::semantics::types::RenderRequest {
                node: *node,
                context: *context,
            },
        )
        .collect();
    let answers = driver
        .lease
        .views
        .semantic
        .ok_or_else(|| OperatingError::SemanticIncomplete {
            reason: crate::domain::operator::constants::NO_SEMANTIC_SESSION.into(),
        })
        .and_then(|lookup| lookup.render_batch(&requests, driver.lease.views.source))
        .and_then(|answers| {
            if answers.len() == requests.len() {
                Ok(answers)
            } else {
                Err(OperatingError::SemanticBatchShape)
            }
        });
    driver.rendered = hold(driver, answers)?;
    Ok(())
}

pub(crate) fn rendered(
    driver: &LeaseDriver<'_, '_>,
    at: RowIndex,
    text: &mut TextScratch,
) -> RenderKind {
    use ryo_language::domain::semantics::types::RenderOutcome;
    match driver.rendered.get(at.0) {
        Some(RenderOutcome::Known(known)) => {
            text.0.push_str(&known.0);
            RenderKind::Known
        }
        Some(RenderOutcome::NotApplicable) | None => RenderKind::NotApplicable,
        Some(RenderOutcome::Unsupported(_)) => RenderKind::Unsupported,
    }
}

pub(crate) fn right_columns(driver: &LeaseDriver<'_, '_>) -> RightColumns {
    if crate::domain::operator::contract::candidates::service::native(&driver.join).is_some() {
        RightColumns::Physical
    } else {
        RightColumns::Logical
    }
}

pub(crate) fn intern(driver: &mut LeaseDriver<'_, '_>, text: CellText<'_>) -> Value {
    Value::Symbol(query_intern_derived(
        &mut driver.lease.derived,
        ByteSlice(driver.lease.views.source.0),
        NameText(text.0),
    ))
}
