//! The `reading` service: free functions named by verb over the visit of one file, with no IO and no ports.

use super::settings::VisitSetting;
use crate::domain::coverage::service::partial_of;
use crate::domain::coverage::types::Recovery;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::finish::service::escape;
use crate::domain::finish::types::{Arena, Leaves, Leaving, Retained, RuleNumber};
use crate::domain::operating::types::{OperatorContext, SemanticLookup};
use crate::domain::operator::service::{emitted, pool_for};

use crate::domain::operator::types::RuleWorkspace;
use crate::domain::ordering::service::sort_findings;
use crate::domain::reading::types::{EngineAsked, EngineRows};
use crate::domain::render::service::{render_rows, rendered_limit, shown_by};
use crate::domain::render::types::{Answering, FindingRule, RenderInput};
use crate::domain::rules::errors::RulesError;
use crate::domain::rules::service::{failed, id_at};
use crate::domain::table_cells::types::{TreeForm, TreeRows};

use crate::domain::table_texts::types::SourceView;
use crate::domain::table_words::service::{encode_table, pack_onto};
use crate::domain::table_words::types::WordSlice;
use crate::domain::value_cells::types::Scene;
use crate::domain::workers::types::Tables;
use ryo_language::domain::elements::service::clear_elements;
use ryo_relations::domain::relations::types::{ColumnsView, FileId, Relation};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::Kept;
use ryo_values::domain::records::types::PlacedRule;
use ryo_values::domain::stored_facts::types::{ArenaBase, Bucket, KeptWords, ReadInPart};

/// Runs the per-file steps of a rule on the rows of the file the store holds, and renders what the rule emits.
///
/// # Errors
///
/// `Failed` when an operator refuses the plan or a finding cannot be rendered.
#[cfg(test)]
pub(crate) fn evaluate_rule<'f>(
    tables: &'f mut Tables,
    setting: &VisitSetting<'_>,
    rule: RuleIndex,
    file: FileId,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    classes: &crate::domain::shared_walk::types::SharedClasses,
) -> Result<Vec<Table>, RulesError> {
    evaluate_rule_with_graph(tables, setting, rule, file, relations, classes, None)
}

/// Evaluate one rule with optional successful cut handles from the request graph.
#[cfg(test)]
pub(crate) fn evaluate_rule_with_graph<'f>(
    tables: &'f mut Tables,
    setting: &VisitSetting<'_>,
    rule: RuleIndex,
    file: FileId,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    classes: &crate::domain::shared_walk::types::SharedClasses,
) -> Result<Vec<Table>, RulesError> {
    evaluate_rule_with_dag(tables, setting, rule, file, relations, classes, None, None)
}

pub(crate) fn evaluate_rule_with_dag<'f>(
    tables: &'f mut Tables,
    setting: &VisitSetting<'_>,
    rule: RuleIndex,
    file: FileId,
    relations: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    classes: &crate::domain::shared_walk::types::SharedClasses,

    file_dag: &dyn crate::application::file_dag::interface::DagExecution,
    executor: &super::executor::ScopedExecutor<'_>,
    semantic: Option<&dyn SemanticLookup>,
) -> Result<Vec<Table>, RulesError> {
    let _route_rule = super::route_counts::rule(
        rule.0,
        setting
            .ids
            .get(rule.0)
            .map_or("<unknown>", |id| id.0.as_ref()),
    );
    let (Some(plan), Some(bound)) = (setting.plans.get(rule.0), setting.bindings.get(rule.0))
    else {
        return Ok(Vec::new());
    };
    let mut workspace = RuleWorkspace {
        pool: pool_for(std::slice::from_ref(plan)),
        ..RuleWorkspace::default()
    };
    workspace.pool.partial = if matches!(setting.answering, Answering::Rows(_)) {
        Vec::new()
    } else {
        tables.partials.clone()
    };
    let refuse = |reason: &dyn std::fmt::Display| failed(&id_at(setting.ids, rule), reason);
    let number = RuleNumber(u32::try_from(rule.0).unwrap_or(u32::MAX));
    let pool = &mut workspace.pool;
    let diagnostic = tracing::enabled!(
        target: "ryo::diagnostic::rule_step",
        tracing::Level::TRACE
    );
    let rule_span = if diagnostic {
        let rule_id = setting
            .ids
            .get(rule.0)
            .map_or("<unknown>", |id| id.0.as_ref());
        let file_path = usize::try_from(file.0)
            .ok()
            .and_then(|index| setting.paths.get(index))
            .map_or("<unknown>", |path| path.0.as_ref());
        tracing::trace_span!(
            target: "ryo::diagnostic::rule_step",
            "rule_file",
            rule = rule_id,
            file = file_path,
        )
    } else {
        tracing::Span::none()
    };
    let _entered = rule_span.enter();
    {
        let mut context = OperatorContext {
            semantic,
            prepared_kinds: classes.kind_vocabulary.as_deref(),
            prepared_matchers: classes.matchers.as_deref(),
            bindings: bound,
            symbols: (&mut tables.symbols).into(),
            source: SourceView(&tables.bytes.0),
            paths: setting.paths,
            arenas: &[],
            memo: &mut workspace.memo,
            scratch: &mut workspace.scratch,
        };
        let evaluated =
            crate::application::file_dag::pipeline::service::evaluate_file_with_executor(
                plan,
                relations,
                Some(TreeRows {
                    elements: &tables.elements,
                    file,
                    form: TreeForm::Edges,
                }),
                &mut context,
                pool,
                &mut tables.pool,
                &classes.classes,
                rule.0,
                file_dag,
                executor,
                setting
                    .operators
                    .rules
                    .get(rule.0)
                    .ok_or(crate::domain::operating::errors::OperatingError::NotRun)
                    .map_err(|error| refuse(&error))?,
            )
            .map_err(|error| match error {
                crate::domain::operating::errors::OperatingError::ObservationIncomplete {
                    reason,
                } => RulesError::ObservationIncomplete {
                    id: id_at(setting.ids, rule),
                    reason,
                },
                crate::domain::operating::errors::OperatingError::SemanticIncomplete { reason } => {
                    RulesError::SemanticIncomplete {
                        id: id_at(setting.ids, rule),
                        reason,
                    }
                }
                other => refuse(&other),
            });
        evaluated?;
    }
    if let Some(table) = emitted(plan, pool) {
        let scene = Scene {
            file,
            symbols: &tables.symbols,
            elements: &tables.elements,
            source: SourceView(&tables.bytes.0),
            paths: setting.paths,
            unknown: setting.unknown,
        };
        let id = FindingRule(setting.ids.get(rule.0).map(|id| &*id.0).unwrap_or_default());
        let rendered = render_rows(
            RenderInput {
                rule: number,
                table,
                emit: &plan.emit,
                layout: shown_by(setting.answering, number, id),
                scene,
                limit: rendered_limit(setting.answering),
            },
            &mut tables.starts,
            &mut tables.rendered,
        )
        .map_err(|error| refuse(&error));
        rendered?;
    }
    // All other workspace payloads drop here, including every error exit above.
    Ok(std::mem::take(&mut pool.frontier))
}

use ryo_values::domain::rows::types::Table;
