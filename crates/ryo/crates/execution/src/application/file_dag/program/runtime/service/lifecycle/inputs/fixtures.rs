//! Compiled program lifecycle proof with a real owner.
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::Emptiness;
use crate::application::file_dag::program::admission::Inputs;
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::program::runtime::service::ProgramExecution;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::executable_program::types::RegionId;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::Emit;
use crate::domain::operating::types::OperatorContext;
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{InputSource, PartitionRequirement, ProducerPartitions},
    table_model::types::Rows,
};
use crate::domain::table_model::types::RepresentationInput as BorrowedRows;
use ryo_relations::domain::{
    relations::{
        service::kinds,
        types::{CellKind, FileId, Flag, Integer, NodeId, Offset, Relation, Span, Value},
    },
    terms::types::Symbol,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, HostedNumber, Input, ParameterId, StepId, StepScope},
    plans::types::{Emit as PlanEmit, Operation, Plan, PlanStep},
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
        _: ryo_values::domain::plan_parts::types::ObservationBindingId,
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

pub(in super::super) struct CompiledCase {
    pub(in super::super) execution: Rc<ProgramExecution>,
    pub(in super::super) roots: [RegionId; 2],
}

pub(in super::super) fn compiled() -> CompiledCase {
    let plans = [plan(Relation::Function), plan(Relation::Identifier)];
    let semantic = crate::application::file_dag::testing::program_derived_port();
    let prepared =
        crate::application::file_dag::request::prepare(&plans, &[vec![], vec![]], &[], Arc::clone(&semantic)).unwrap();
    let graph = crate::domain::file_program::service::compile_prepared(&plans, &prepared);
    let program = Arc::new(
        crate::domain::executable_program::service::compile(
            &plans,
            graph,
            &FileProducers,
            &prepared, crate::domain::executable_program::types::RoutePolicy::default(),
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
    let owner = Rc::new(RelationOwner::new(crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible, semantic));
    let partition = Partition::File(
        crate::application::file_dag::program::admission::FileGeneration::from_owner(
            FileId(0),
            &owner,
        ),
    );
    let execution = Rc::new(ProgramExecution::open(
        program,
        partition,
        Rc::clone(&owner),
    ));
    CompiledCase { execution, roots }
}

pub(in super::super) fn value(kind: CellKind) -> Value {
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

pub(in super::super) struct TestInputs {
    owner: Rc<RelationOwner>,
    sources: HashMap<Relation, Table>,
    admitted: Vec<InputSource>,
}
impl TestInputs {
    pub(in super::super) fn new(execution: &ProgramExecution) -> Self {
        let sources = [Relation::Function, Relation::Identifier]
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
        root: crate::application::file_dag::program::admission::ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        table: Table,
    ) -> Result<AdmittedInput, OperatingError> {
        let read = crate::application::file_dag::program::admission::contributed_proof(
            partition,
            inputs,
            &self.owner,
        )?;
        let publication = self.owner.computed(table)?;
        let rows = self
            .owner
            .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
        let proof = self
            .owner
            .admit(publication.as_ref(), BorrowedRows::from(&rows))?;
        AdmittedInput::from_publication(
            Rc::clone(&self.owner),
            publication.clone(),
            &proof,
            root,
            read,
        )
    }
}
impl Inputs for TestInputs {
    fn population_scope(
        &self,
    ) -> crate::application::file_dag::relations::publication::types::ProducerScope {
        crate::application::file_dag::relations::publication::types::ProducerScope::File
    }
    fn supply(
        &mut self,
        source: InputSource,
        partition: &Partition,
        _: &mut OperatorContext<'_>,
    ) -> Result<crate::application::file_dag::program::admission::AvailableInput, OperatingError>
    {
        self.admitted.push(source);
        let InputSource::Relation(relation) = source else {
            return Err(OperatingError::NotRead);
        };
        let rows = self.read(relation)?;
        let publication = self.owner.source(relation, &rows)?;
        crate::application::file_dag::program::admission::AvailableInput::source(
            Rc::clone(&self.owner),
            publication,
            crate::application::file_dag::program::admission::ProducerRoot::Input(source),
            partition.clone(),
            crate::domain::completion::types::CoverageState::Complete,
            rows,
        )
    }
    fn admit(
        &mut self,
        input: &crate::application::file_dag::program::admission::AvailableInput,
        partition: &Partition,
        _: &mut OperatorContext<'_>,
    ) -> Result<AdmittedInput, OperatingError> {
        input.admit(&self.owner, partition)
    }
    fn is_empty(
        &self,
        input: &AdmittedInput,
        representation: crate::domain::operator::contract::types::EmptyRepresentation,
    ) -> Result<Emptiness, OperatingError> {
        let publication = input.publication(&self.owner)?;
        let rows = self
            .owner
            .resolve(publication.as_ref(), &|relation| self.read(relation))?;
        Ok(
            if crate::domain::table_model::service::count_of(&rows).0 == 0
                && (representation
                    == crate::domain::operator::contract::types::EmptyRepresentation::Any
                    || matches!(rows, Rows::Computed(_)))
            {
                Emptiness::Empty
            } else {
                Emptiness::Populated
            },
        )
    }
    fn publish(
        &mut self,
        root: crate::application::file_dag::program::admission::ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        representation: crate::application::file_dag::program::admission::Representation,
    ) -> Result<AdmittedInput, OperatingError> {
        use crate::application::file_dag::program::admission::Representation;
        let publication = match representation {
            Representation::Computed(rows) => return self.computed(root, partition, inputs, rows),
            Representation::Minted(publication) => publication,
        };
        let read = crate::application::file_dag::program::admission::contributed_proof(
            partition,
            inputs,
            &self.owner,
        )?;
        let rows = self
            .owner
            .resolve(publication.as_ref(), &|relation| self.read(relation))?;
        let proof = self
            .owner
            .admit(publication.as_ref(), BorrowedRows::from(&rows))?;
        AdmittedInput::from_publication(
            Rc::clone(&self.owner),
            publication.clone(),
            &proof,
            root,
            read,
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
