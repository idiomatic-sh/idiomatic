//! A compiled program answers the declared partition and phase of its sources, and refuses a source it does not hold.
use crate::domain::operator::types::ReaderMultiplicity;

use super::value::InterpretedProgram;
use crate::domain::{
    coverage::types::RuleIndex,
    interpreted_program::types::{
        FileReaderGraph, InputSource, ObservationPlace, RegionId, SourcePartition,
        region::value::Region,
    },
    operating::errors::OperatingError,
    source_partitions::types::PartitionRequirement,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::StepScope;

fn program() -> InterpretedProgram {
    let mut region = Region::new(
        RuleIndex(0),
        Ok(Box::new([])),
        StepScope::PerFile,
        Box::new([]),
        Box::new([InputSource::Relation(Relation::Function)]),
        ReaderMultiplicity(1),
    );
    region.set_source_partitions(Box::new([SourcePartition {
        source: InputSource::Relation(Relation::Function),
        requirement: PartitionRequirement::Project,
    }]));
    InterpretedProgram::new(
        Box::new([region]),
        FileReaderGraph::default(),
        Box::new([]),
        Box::new([RegionId(0)]),
        Box::new([Ok(RegionId(0)), Err(OperatingError::NotRead)]),
    )
}

#[test]
fn a_declared_relation_answers_its_partition_and_phase() {
    let program = program();
    let source = InputSource::Relation(Relation::Function);
    assert_eq!(
        program.source_partition(source),
        Ok(PartitionRequirement::Project)
    );
    assert_eq!(program.frontier_phase(source), Ok(StepScope::CrossFile));
    assert_eq!(
        program.frontier_phase(InputSource::Region(RegionId(0))),
        Ok(StepScope::PerFile)
    );
    assert_eq!(
        program.frontier_phase(InputSource::Observation(ObservationPlace(0))),
        Ok(StepScope::PerFile)
    );
    assert_eq!(program.regions().len(), 1);
    assert_eq!(program.logical_regions(), [RegionId(0)]);
}

#[test]
fn a_source_the_program_does_not_hold_is_refused() {
    let program = program();
    assert_eq!(
        program.source_partition(InputSource::Relation(Relation::Defines)),
        Err(OperatingError::NotRead)
    );
    assert_eq!(
        program.frontier_phase(InputSource::Region(RegionId(9))),
        Err(OperatingError::NotRun)
    );
    assert_eq!(
        program.frontier_phase(InputSource::Observation(ObservationPlace(1))),
        Err(OperatingError::NotRead)
    );
}
