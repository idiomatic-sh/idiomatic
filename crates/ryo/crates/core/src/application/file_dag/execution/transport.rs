//! The common engine alone interprets typed demand, lazy gates and publication order.
use super::super::{
    aggregation::Aggregation, invocation::InvocationResources, workspace::InvocationWorkspace,
};
use super::{CommonOperatorEngine, EMPTY, lease};
use crate::application::file_dag::feed::Feed;
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, InputPlace, OperatorContext, Room, Supply},
};
use crate::domain::operator::contract::{types::*, views::BorrowedRows};
use crate::domain::replay::{
    service::scan,
    types::{Collect, Pipe},
};
use crate::domain::table_model::{
    service::{count_of, reset},
    types::Rows,
};
use crate::domain::table_texts::types::Texts;
use ryo_relations::domain::relations::types::Value;

pub(super) fn stream(
    engine: &CommonOperatorEngine,
    contract: &OperatorContract,
    feed: &mut dyn Feed<'_>,
    context: &mut OperatorContext<'_>,
    room: &mut Room,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let mut workspace = InvocationWorkspace::new(std::sync::Arc::clone(&engine.derived));
    workspace.predicates = std::mem::take(&mut room.scratch.select_scratch);
    workspace.predicate_memo = std::mem::take(&mut room.memo);
    workspace.downstream_scratch = std::mem::take(&mut room.scratch);
    let result = if contract.kind == ryo_values::domain::plans::types::OperationKind::Join {
        let mut output = super::output::State::take(
            &mut workspace.downstream_scratch,
            &mut workspace.downstream_memo,
        );
        let result = output.run(emit, |emit| {
            stream_scoped(engine, contract, feed, context, room, emit, &mut workspace)
        });
        output.restore(
            &mut workspace.downstream_scratch,
            &mut workspace.downstream_memo,
        );
        result
    } else {
        stream_scoped(engine, contract, feed, context, room, emit, &mut workspace)
    };
    #[cfg(test)]
    let mut observed = workspace.measured();
    room.scratch = std::mem::take(&mut workspace.downstream_scratch);
    room.scratch.select_scratch = std::mem::take(&mut workspace.predicates);
    room.memo = std::mem::take(&mut workspace.predicate_memo);
    drop(workspace);
    #[cfg(test)]
    {
        observed.local_resources_released = true;
        *room.scratch.observation = observed;
    }
    result
}

fn stream_scoped(
    engine: &CommonOperatorEngine,
    contract: &OperatorContract,
    feed: &mut dyn Feed<'_>,
    context: &mut OperatorContext<'_>,
    room: &mut Room,
    emit: &mut dyn Emit,
    workspace: &mut InvocationWorkspace,
) -> Result<(), OperatingError> {
    let mode = contract.streaming;
    if let Some(program) = &contract.aggregation {
        let kinds = feed.kinds(InputPlace(0))?;
        let mut aggregate = Aggregation::new(program, kinds)?;
        feed.stream(
            InputPlace(0),
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                let texts = Texts {
                    symbols: each.symbols.as_ref(),
                    source: each.source,
                    paths: each.paths,
                    arenas: each.arenas,
                };
                aggregate.push(row, &texts)
            }),
        )?;
        let (first, groups) = aggregate.finish();
        let mut resources = InvocationResources::with_groups(groups);
        let views = [BorrowedRows::Computed(&first)];
        return lease::evaluate(
            engine.operators,
            contract,
            PreparedInputs::Views(&views),
            &mut resources,
            workspace,
            context,
            emit,
        );
    }
    if mode.schema_boundary == SchemaBoundary::BeforeDemand {
        contract.schema.as_ref().map_err(Clone::clone)?;
    }
    reset(&mut workspace.distinct, &contract.set_kinds);
    let mut resources = None;
    if mode.shape == InputShape::RequestSource {
        if contract.external_source {
            let source = feed.hold(InputPlace(0), context, &mut room.held[0])?;
            workspace.capture(feed, 1)?;
            let originals = [source];
            let views = originals.iter().map(BorrowedRows::from).collect::<Vec<_>>();
            let mut resources = None;
            return engine.run(
                contract,
                mode,
                &views,
                &originals,
                &mut resources,
                workspace,
                context,
                emit,
            );
        }
        return engine.run(
            contract,
            mode,
            &[],
            &[],
            &mut resources,
            workspace,
            context,
            emit,
        );
    }
    if mode.publication == Publication::AfterSuccess || mode.shape == InputShape::Views {
        let views = hold(feed, context, &mut room.held, contract.inputs.used.0, false)?;
        workspace.capture(feed, views.len())?;
        return held(
            engine,
            contract,
            mode,
            &views,
            true,
            workspace,
            context,
            &mut room.out,
            emit,
        );
    }
    match mode.demand {
        DemandOrder::Unary | DemandOrder::LeftThenRight => {
            for place in 0..contract.inputs.used.0 {
                if mode.row_inputs.0 & (1 << place) == 0 {
                    continue;
                }
                feed.stream(
                    InputPlace(place),
                    context,
                    &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                        engine.row(
                            contract,
                            mode,
                            InputPlace(place),
                            row,
                            &[],
                            &[],
                            &mut resources,
                            workspace,
                            each,
                            emit,
                        )
                    }),
                )?;
            }
            Ok(())
        }
        DemandOrder::JoinTransport => {
            if feed.streams(InputPlace(0)) == Supply::AtHand
                || feed.streams(InputPlace(1)) == Supply::Streamed
            {
                let views = hold(feed, context, &mut room.held, 2, true)?;
                workspace.capture(feed, views.len())?;
                if views.len() == 1 {
                    return Ok(());
                }
                held(
                    engine,
                    contract,
                    mode,
                    &views,
                    false,
                    workspace,
                    context,
                    &mut room.out,
                    emit,
                )
            } else {
                let held_right = feed.hold(InputPlace(1), context, &mut room.held[1])?;
                workspace.capture(feed, 2)?;
                let fallback =
                    super::physical_fallbacks(std::slice::from_ref(&held_right), workspace, 1)?;
                let right = fallback[0].as_ref().map_or_else(
                    || held_right.clone(),
                    |table| Rows::Computed(table.as_ref()),
                );
                let originals = [Rows::Computed(&EMPTY), right];
                let sides = originals.iter().map(BorrowedRows::from).collect::<Vec<_>>();
                let mut resources = None;
                let mut found = 0;
                let mut stats = super::super::optimized::join::StreamJoinStats::begin(
                    workspace.owner().derived.index_observation(),
                );
                let native = matches!(&originals[1], Rows::Tree(_));
                let result = feed.stream(
                    InputPlace(0),
                    context,
                    &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                        stats.probe();
                        let mut counted =
                            Pipe(|own: &mut OperatorContext<'_>, output: &[Value]| {
                                stats.hit();
                                found += 1;
                                emit.row(own, output)
                            });
                        engine.row(
                            contract,
                            mode,
                            InputPlace(0),
                            row,
                            &sides,
                            &originals,
                            &mut resources,
                            workspace,
                            each,
                            &mut counted,
                        )
                    }),
                );
                stats.finish(contract, &originals[1], workspace.owner().derived.index_observation(), ryo_values::domain::plans::service::named(&contract.inputs).get(1).is_some_and(|input|matches!(input, ryo_values::domain::plan_parts::types::Input::Relation(relation) if workspace.owner().derived.relation_eligible(*relation))), result.is_ok());
                if native {
                    feed.found(
                        InputPlace(1),
                        ryo_values::domain::rows::types::RowCount(found),
                    );
                }
                result
            }
        }
        DemandOrder::AxisTransport => {
            let streaming = feed.streams(InputPlace(0)) == Supply::Streamed
                && feed.streams(InputPlace(1)) == Supply::AtHand
                && feed.streams(InputPlace(2)) == Supply::AtHand;
            if !streaming {
                let views = hold(feed, context, &mut room.held, 3, true)?;
                workspace.capture(feed, views.len())?;
                if views.len() == 1 {
                    return Ok(());
                }
                held(
                    engine,
                    contract,
                    mode,
                    &views,
                    mode.held_publication == Publication::AfterSuccess,
                    workspace,
                    context,
                    &mut room.out,
                    emit,
                )
            } else {
                let [_, edge_table, domain_table] = &mut room.held;
                let edges = feed.hold(InputPlace(1), context, edge_table)?;
                let domain = feed.hold(InputPlace(2), context, domain_table)?;
                workspace.capture(feed, 3)?;
                let originals = [Rows::Computed(&EMPTY), edges, domain];
                let sides = originals.iter().map(BorrowedRows::from).collect::<Vec<_>>();
                let mut resources = None;
                feed.stream(
                    InputPlace(0),
                    context,
                    &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                        engine.row(
                            contract,
                            mode,
                            InputPlace(0),
                            row,
                            &sides,
                            &originals,
                            &mut resources,
                            workspace,
                            each,
                            emit,
                        )
                    }),
                )
            }
        }
        DemandOrder::LeftThenRightUnlessEmptyLeft => {
            let views = hold(feed, context, &mut room.held, 2, true)?;
            workspace.capture(feed, views.len())?;
            if views.len() == 1 {
                return Ok(());
            }
            held(
                engine,
                contract,
                mode,
                &views,
                false,
                workspace,
                context,
                &mut room.out,
                emit,
            )
        }
        DemandOrder::None => Err(OperatingError::WrongOperation),
    }
}
fn hold<'a, 'f>(
    feed: &mut dyn Feed<'f>,
    context: &mut OperatorContext<'_>,
    tables: &'a mut [Table; 3],
    count: usize,
    empty_gate: bool,
) -> Result<Vec<Rows<'a>>, OperatingError>
where
    'f: 'a,
{
    if count > tables.len() {
        return Err(OperatingError::WrongOperation);
    }
    let [first, second, third] = tables;
    let mut rows = Vec::with_capacity(count);
    if count > 0 {
        rows.push(feed.hold(InputPlace(0), context, first)?);
    }
    if empty_gate && rows.first().is_some_and(|rows| count_of(rows).0 == 0) {
        return Ok(rows);
    }
    if count > 1 {
        rows.push(feed.hold(InputPlace(1), context, second)?);
    }
    if count > 2 {
        rows.push(feed.hold(InputPlace(2), context, third)?);
    }
    Ok(rows)
}
fn held<'f>(
    engine: &CommonOperatorEngine,
    contract: &OperatorContract,
    mode: ModeContract,
    rows: &[Rows<'f>],
    buffer: bool,
    workspace: &mut InvocationWorkspace,
    context: &mut OperatorContext<'_>,
    out: &mut Table,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let logical = ryo_values::domain::plans::service::named(&contract.inputs);
    if super::super::optimized::join::supports(&contract.resources, logical, rows) {
        // Physical fallback is permissive; authoritative computed populations are not.
        for (place, rows) in rows.iter().enumerate() {
            super::selected_schema(rows)?;
            if matches!(rows, Rows::Computed(_)) {
                workspace.validate(place, BorrowedRows::from(rows))?;
            }
        }
        let normalized = super::physical_fallbacks(rows, workspace, 0)?;
        let normalized_rows = rows
            .iter()
            .zip(&normalized)
            .map(|(rows, fallback)| {
                fallback
                    .as_ref()
                    .map_or_else(|| rows.clone(), |table| Rows::Computed(table.as_ref()))
            })
            .collect::<Vec<_>>();
        let rows = normalized_rows.as_slice();
        if buffer {
            reset(
                out,
                contract.collected_kinds.as_ref().map_err(Clone::clone)?,
            );
            if let Some(result) = super::super::optimized::join::evaluate(
                &super::PrimitiveSemanticsBridge {
                    port: engine.operators,
                },
                contract,
                mode.evaluation,
                &contract.resources,
                logical,
                rows,
                workspace,
                context,
                &mut Collect(out),
            ) {
                result?;
                return scan(&Rows::Computed(out), &mut Vec::new(), context, emit);
            }
        } else if let Some(result) = super::super::optimized::join::evaluate(
            &super::PrimitiveSemanticsBridge {
                port: engine.operators,
            },
            contract,
            mode.evaluation,
            &contract.resources,
            logical,
            rows,
            workspace,
            context,
            emit,
        ) {
            return result;
        }
    }
    let normalized = super::physical_fallbacks(rows, workspace, 0)?;
    let originals = rows
        .iter()
        .zip(&normalized)
        .map(|(input, fallback)| {
            fallback
                .as_ref()
                .map_or_else(|| input.clone(), |table| Rows::Computed(table.as_ref()))
        })
        .collect::<Vec<_>>();
    let rows = originals.as_slice();
    let views = rows.iter().map(BorrowedRows::from).collect::<Vec<_>>();
    let mut resources = None;
    if mode.acquisition == AcquisitionBoundary::BeforeRows {
        resources = Some(InvocationResources::prepare(
            contract,
            &views,
            context,
            mode.evaluation,
            rows,
            workspace,
        )?);
    }
    if buffer {
        reset(
            out,
            contract.collected_kinds.as_ref().map_err(Clone::clone)?,
        );
        engine.run(
            contract,
            mode,
            &views,
            rows,
            &mut resources,
            workspace,
            context,
            &mut Collect(out),
        )?;
        let mut row = Vec::new();
        scan(&Rows::Computed(out), &mut row, context, emit)
    } else {
        engine.run(
            contract,
            mode,
            &views,
            rows,
            &mut resources,
            workspace,
            context,
            emit,
        )
    }
}

use ryo_values::domain::rows::types::Table;
