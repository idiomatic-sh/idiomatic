//! Compiled program lifecycle proof with a real owner.
use super::super::*;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{InputSource, PartitionRequirement, ProducerPartitions},
    operator::contract::{types::RequestOperators, views::BorrowedRows},
    table_model::types::Rows,
};
use ryo_relations::domain::{
    relations::{
        service::kinds,
        types::{CellKind, FileId, Flag, Integer, NodeId, Offset, Relation, Span, Value},
    },
    terms::types::Symbol,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, HostedNumber, Input, ParameterId, StepId},
    plans::types::{Emit as PlanEmit, Operation, Plan, PlanStep, StepScope},
    rows::types::{RowCount, Table},
};
use std::{collections::HashMap, rc::Rc, sync::Arc};

struct FileProducers;
impl ProducerPartitions for FileProducers {
    fn hosted(
        &self,
        _: RuleIndex,
        _: HostedNumber,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::NotRead)
    }
    fn parameter(
        &self,
        _: RuleIndex,
        _: ParameterId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::Unbound)
    }
    fn relation(&self, _: Relation) -> PartitionRequirement {
        PartitionRequirement::CurrentFile
    }
    fn observation(
        &self,
        _: RuleIndex,
        _: ryo_values::domain::observations::types::ObservationBindingId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::NotRead)
    }
}

fn plan(relation: Relation) -> Plan {
    Plan {
        observation_inputs: vec![],
        parameters: vec![],
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::GroupCount {
                from: Input::Relation(relation),
                keys: vec![ColumnIndex(0)],
                distinct: None,
            },
        }],
        comparison_origins: vec![],
        observations: vec![],
        frontier: vec![],
        emit: PlanEmit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
}

pub(super) struct CompiledCase {
    pub(super) execution: Rc<ProgramExecution>,
    pub(super) roots: [RegionId; 2],
}

pub(super) fn compiled() -> CompiledCase {
    let plans = [plan(Relation::Function), plan(Relation::Class)];
    let semantic = crate::application::file_dag::testing::program_derived_port();
    let prepared =
        RequestOperators::prepare(&plans, &[vec![], vec![]], &[], Arc::clone(&semantic)).unwrap();
    let graph = crate::domain::file_program::service::compile_prepared(&plans, &prepared);
    let program = Arc::new(
        crate::domain::executable_program::service::compile(
            &plans,
            graph,
            &FileProducers,
            &prepared,
        )
        .unwrap(),
    );
    let roots = [
        program.rules()[0].emit.as_ref().copied().unwrap(),
        program.rules()[1].emit.as_ref().copied().unwrap(),
    ];
    assert_ne!(
        roots[0], roots[1],
        "distinct source roots must remain distinct"
    );
    let owner = Rc::new(RelationOwner::new(true, semantic));
    let partition = Partition::File(super::super::super::admission::FileGeneration::from_owner(
        FileId(0),
        &owner,
    ));
    let execution = Rc::new(ProgramExecution::open(
        program,
        partition,
        Rc::clone(&owner),
    ));
    CompiledCase { execution, roots }
}

pub(super) fn value(kind: CellKind) -> Value {
    match kind {
        CellKind::Node => Value::Node(NodeId(0)),
        CellKind::Symbol => Value::Symbol(Symbol(0)),
        CellKind::File => Value::File(FileId(0)),
        CellKind::Span => Value::Span(Span {
            start: Offset(0),
            end: Offset(0),
        }),
        CellKind::Number => Value::Number(Integer(1)),
        CellKind::Flag => Value::Flag(Flag::Lowered),
    }
}

pub(super) struct TestInputs {
    owner: Rc<RelationOwner>,
    sources: HashMap<Relation, Table>,
    admitted: Vec<InputSource>,
}
impl TestInputs {
    pub(super) fn new(execution: &ProgramExecution) -> Self {
        let sources = [Relation::Function, Relation::Class]
            .into_iter()
            .map(|relation| {
                let kinds = kinds(relation).to_vec();
                let cells = kinds.iter().copied().map(value).collect();
                (
                    relation,
                    Table {
                        kinds,
                        cells,
                        count: RowCount(1),
                    },
                )
            })
            .collect();
        Self {
            owner: Rc::clone(&execution.owner),
            sources,
            admitted: vec![],
        }
    }
    fn read(&self, relation: Relation) -> Result<Rows<'_>, OperatingError> {
        self.sources
            .get(&relation)
            .map(Rows::Computed)
            .ok_or(OperatingError::NotRead)
    }
    fn computed(
        &self,
        root: super::super::super::admission::ProducerRoot,
        partition: &Partition,
        table: Table,
    ) -> Result<AdmittedInput, OperatingError> {
        let publication = self.owner.computed(table)?;
        let rows = self
            .owner
            .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
        let proof = self
            .owner
            .admit(publication.as_ref(), BorrowedRows::from(&rows))?;
        AdmittedInput::from_publication(
            Rc::clone(&self.owner),
            publication,
            &proof,
            root,
            partition.clone(),
            super::super::super::admission::Readiness::Complete,
        )
    }
}
impl Inputs for TestInputs {
    fn admit(
        &mut self,
        source: InputSource,
        partition: &Partition,
        _: &mut OperatorContext<'_>,
    ) -> Result<AdmittedInput, OperatingError> {
        self.admitted.push(source);
        let InputSource::Relation(relation) = source else {
            return Err(OperatingError::NotRead);
        };
        let rows = self.read(relation)?;
        let publication = self.owner.source(relation, &rows)?;
        let proof = self
            .owner
            .admit(publication.as_ref(), BorrowedRows::from(&rows))?;
        AdmittedInput::from_publication(
            Rc::clone(&self.owner),
            publication,
            &proof,
            super::super::super::admission::ProducerRoot::Input(source),
            partition.clone(),
            super::super::super::admission::Readiness::Complete,
        )
    }
    fn is_empty(&self, input: &AdmittedInput) -> Result<bool, OperatingError> {
        let publication = input.publication(&self.owner)?;
        let rows = self
            .owner
            .resolve(publication.as_ref(), &|relation| self.read(relation))?;
        Ok(matches!(rows, Rows::Computed(table) if table.count.0 == 0))
    }
    fn publish(
        &mut self,
        root: super::super::super::admission::ProducerRoot,
        partition: &Partition,
        _: &[AdmittedInput],
        rows: Table,
    ) -> Result<AdmittedInput, OperatingError> {
        self.computed(root, partition, rows)
    }
    fn publish_selected(
        &mut self,
        _: super::super::super::admission::ProducerRoot,
        _: &Partition,
        _: &[AdmittedInput],
        _: super::super::super::selection::ProducedSelection,
    ) -> Result<AdmittedInput, OperatingError> {
        Err(OperatingError::WrongOperation)
    }
    fn publish_empty(
        &mut self,
        root: super::super::super::admission::ProducerRoot,
        partition: &Partition,
        _: &[AdmittedInput],
        schema: Result<Vec<CellKind>, OperatingError>,
    ) -> Result<AdmittedInput, OperatingError> {
        self.computed(
            root,
            partition,
            Table {
                kinds: schema?,
                cells: vec![],
                count: RowCount(0),
            },
        )
    }
    fn replay(
        &self,
        input: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        output: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let publication = input.publication(&self.owner)?;
        let rows = self
            .owner
            .resolve(publication.as_ref(), &|relation| self.read(relation))?;
        let Rows::Computed(table) = rows else {
            return Err(OperatingError::WrongOperation);
        };
        for row in table.cells.chunks(table.kinds.len()) {
            output.row(context, row)?;
        }
        Ok(())
    }
}
