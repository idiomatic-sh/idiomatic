// ignore: idiomatic.arch/idiomatic-rust/module-cycles - the steps feed on the steps they run: the `Feed` impl of `Stepper` reads an input by running its step, which is the service's recursion (`run`, `operate`, `deliver`), so the impl and the service it calls form one ring by design
//! The types of the `pipeline` concept: the steps of a plan being run for a file, and what they read their inputs through.

use ryo_language::domain::relations::types::{CellKind, Relation, Value};

use crate::domain::coverage::types::Partial;
use crate::domain::lowering::types::Counting;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room, Supply};
use crate::domain::operator::types::{PlanLayout, RowsFed, StepPlace};
use crate::domain::pipeline::service::{deliver, feed, input, keep};
use crate::domain::positions::types::LoweredRoom;
use crate::domain::replay::types::Pipe;
use crate::domain::table_cells::types::TreeRows;
use crate::domain::table_model::service::count_of;
use crate::domain::table_model::types::{RowCount, Rows, Table};
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::types::Plan;

pub(crate) mod compact;

/// The tree of a file, when the steps that read only it run over positions.
#[derive(Clone, Copy, Debug)]
pub(crate) enum LoweredTree<'f> {
    /// No step runs over positions.
    Missing,
    /// The tree the lowered steps run over.
    Tree(TreeRows<'f>),
}

/// The steps of a plan being run for a file: the ones that keep their rows are before the one being made.
pub(crate) struct Stepper<'a, 'f> {
    pub kernel_strategy: crate::domain::column_kernel::types::KernelStrategy,
    pub kernel: Option<&'a crate::domain::column_kernel::types::KernelPlan>,
    pub kernel_counts: &'a mut crate::domain::column_kernel::types::KernelCounts,
    pub executor: &'a dyn crate::domain::operator::contract::executor::PrimitiveExecutor,
    pub operators:
        &'a [std::sync::Arc<crate::domain::operator::contract::types::OperatorContract>],
    /// The plan.
    pub plan: &'a Plan,
    /// The rows of a relation, where the store keeps them.
    pub read: &'a dyn Reading<'f>,
    /// The tables of the steps before the one being made.
    pub done: &'a [Table],
    /// Compact kept outputs alongside the materialized table slots.
    pub selected:
        &'a [Option<std::sync::Arc<crate::domain::table_model::types::StoredSelection>>],
    /// Successful internal cuts from earlier active rules of this same physical file.
    pub shared: &'a [Option<std::sync::Arc<Table>>],
    /// Application-owned schedule, borrowed for this residual invocation only.
    pub dag: Option<&'a dyn crate::domain::file_program::types::DagExecution>,
    /// Invocation leases pin a demanded graph handle without copying its cells.
    pub dag_shared: &'a [std::cell::OnceCell<std::sync::Arc<Table>>],
    /// Ordered terminal identity; graph computation itself is shared across rules.
    pub rule: usize,
    /// Steps made at a global DAG cut even when this rule has one local reader.
    pub graph_kept: &'a [bool],
    /// Prepared stored programs for the current request's plan.
    pub selectors: &'a [Option<std::sync::Arc<crate::domain::stored_selection::types::Selector>>],
    /// What each step keeps for itself.
    pub rooms: &'a mut [Option<Box<Room>>],
    /// Rows being read one at a time from a relation, one buffer for each scan at once.
    pub spare: &'a mut Vec<Vec<Value>>,
    /// How the plan is laid out.
    pub layout: &'a PlanLayout,
    /// The tree of the file, when the steps that read only it run over positions.
    pub lowered: LoweredTree<'f>,
    /// The rows the lowered steps made.
    pub made: &'a mut LoweredRoom,
    /// Immutable selected positions retained only for this physical file.
    pub classes: &'a [crate::domain::positions::types::Slots],
    /// The step whose inputs are being asked for.
    pub current: StepPlace,
    /// The relations the file is read in part.
    pub partial: &'a [Partial],
    /// How many rows each step of the plan has been fed.
    pub fed: &'a mut Vec<RowsFed>,
    /// Whether they are counted.
    pub counting: Counting,
}

/// What gives the rows of a relation, where the store keeps them.
pub(crate) trait Reading<'f> {
    /// The rows of the relation.
    ///
    /// # Errors
    ///
    /// `NotRead` when nothing provides them.
    fn rows(&self, relation: Relation) -> Result<Rows<'f>, OperatingError>;
}

impl<'f, F: Fn(Relation) -> Result<Rows<'f>, OperatingError>> Reading<'f> for F {
    fn rows(&self, relation: Relation) -> Result<Rows<'f>, OperatingError> {
        self(relation)
    }
}

impl<'a> Feed<'a> for Stepper<'a, '_> {
    fn kinds(&self, which: InputPlace) -> Result<&[CellKind], OperatingError> {
        Ok(match input(self, which)? {
            Input::Relation(relation) => match self.read.rows(relation)? {
                Rows::Stored(view) => view.kinds,
                Rows::Tree(tree) => {
                    ryo_language::domain::relations::service::kinds(match tree.form {
                        crate::domain::table_cells::types::TreeForm::Elements => {
                            Relation::SyntaxTree
                        }
                        crate::domain::table_cells::types::TreeForm::Edges => Relation::SyntaxChild,
                    })
                }
                Rows::Computed(table) => &table.kinds,
                Rows::Selected(_) => return Err(OperatingError::WrongOperation),
            },
            Input::Step(id) => self
                .layout
                .kinds
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .ok_or(OperatingError::NotRun)?,
        })
    }

    fn found(&mut self, which: InputPlace, rows: RowCount) {
        if self.counting == Counting::Counts {
            let consumer = self.current;
            feed(self, consumer, which, rows);
        }
    }

    fn streams(&self, which: InputPlace) -> Supply {
        if matches!(input(self, which), Ok(Input::Step(id)) if self.layout.kept.0.get(usize::try_from(id.0).unwrap_or(usize::MAX)) != Some(&true) && self.graph_kept.get(usize::try_from(id.0).unwrap_or(usize::MAX)) != Some(&true))
        {
            Supply::Streamed
        } else {
            Supply::AtHand
        }
    }

    fn stream(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        if self.counting == Counting::DoesNotCount {
            return deliver(self, which, context, emit);
        }
        let consumer = self.current;
        let mut delivered = 0_usize;
        let outcome = {
            let mut counting = Pipe(|piped: &mut OperatorContext<'_>, row: &[Value]| {
                delivered += 1;
                emit.row(piped, row)
            });
            deliver(self, which, context, &mut counting)
        };
        feed(self, consumer, which, RowCount(delivered));
        outcome
    }

    fn hold<'t>(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        into: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'a: 't,
    {
        if self.counting == Counting::DoesNotCount {
            return keep(self, which, context, into);
        }
        let consumer = self.current;
        let held = keep(self, which, context, into)?;
        // A tree is looked up, not read: what it was fed is what was found in it.
        if !matches!(held, Rows::Tree(_)) {
            let rows = count_of(&held);
            feed(self, consumer, which, rows);
        }
        Ok(held)
    }
}
