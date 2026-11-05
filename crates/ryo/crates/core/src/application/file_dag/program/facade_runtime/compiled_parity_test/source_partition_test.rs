//! Producer declarations and actual source scope govern ordinary admission.
use super::*;
use crate::application::file_dag::program::{
    admission::{FileGeneration, Inputs, Partition, ProducerRoot, contributor_readiness},
    context::RunContext,
    kernel_session::KernelSession,
    runtime::{Delivery, InvocationOutcome, ProgramExecution},
};
use crate::domain::{
    coverage::types::{Lacking, Partial, Partials, RuleIndex},
    table_model::types::{BindingCell, BindingTable, ParameterBinding},
};
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::{HostedNumber, ParameterId, ParameterName, TableKey},
    plans::types::{ParameterType, PlanParameter},
    stored_facts::types::PartialCause,
};
use std::{cell::Cell as Counter, rc::Rc};
#[derive(Clone, Copy)]
enum SourceCase {
    File,
    ProjectRefusal,
    Project,
    Partial,
    Hosted,
    Parameter,
}
type Input = SourceCase;
type Value = &'static str;
type Failure = OperatingError;
fn subject(case: Input) -> Result<Value, Failure> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let mut source_plan = plan(Scenario::Words);
    let mut bindings = vec![];
    let mut hosted = vec![];
    let relation = if matches!(case, SourceCase::Project) {
        Relation::ManifestDiagnostic
    } else {
        Relation::SyntaxTree
    };
    if matches!(case, SourceCase::Project) {
        source_plan.steps[0].scope = StepScope::CrossFile;
        source_plan.steps[0].operation = Operation::Select {
            from: PlanInput::Relation(relation),
            comparisons: vec![],
            kept: vec![],
        };
    }
    if matches!(case, SourceCase::Hosted) {
        source_plan.steps[0].operation = Operation::Hosted {
            relation: HostedNumber(0),
        };
        hosted.push(HostRelation {
            name: HostName("argument".into()),
            columns: vec![HostColumn {
                name: HostColumnName("value".into()),
                kind: HostKind::Number,
            }],
            rows: vec![HostRow(vec![
                ryo_values::domain::records::types::Cell::Number(Integer(42)),
            ])],
        });
    }
    if matches!(case, SourceCase::Parameter) {
        source_plan.parameters = vec![PlanParameter {
            name: ParameterName("argument".into()),
            kind: ParameterType::Tables,
        }];
        source_plan.steps[0].operation = Operation::Rows {
            parameter: ParameterId(0),
            keys: vec![TableKey("value".into())],
        };
        bindings.push(ParameterBinding::Tables(vec![BindingTable(vec![
            BindingCell {
                key: LiteralText("value".into()),
                value: LiteralText("answer".into()),
            },
        ])]));
    }
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = facade::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![source_plan],
            bindings: vec![bindings],
        },
        &[rule()],
        &[Digest([0; 16])],
        &hosted,
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let engine = CommonOperatorEngine::new(crate::outbound::operators::factory::test_port());
    let corpus = ["a", "b"].map(|path| {
        CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath(path.into()),
            size: FileSize(SMALL_SOURCE.len() as u64),
            modified: Nanoseconds(0),
        })
    });
    let services = ProgramServices::new(&engine, Arc::clone(&derived), &corpus);
    let required = required_partitions(&prepared, RuleIndex(0), &corpus, &[], &[], &services)?;
    let owner = open_program_owner(prepared.program(), derived);
    let project = Partition::Project(required);
    let scope = if matches!(case, SourceCase::Project) {
        project.clone()
    } else {
        Partition::File(FileGeneration::from_owner(FileId(0), &owner))
    };
    let execution = ProgramExecution::open(
        Arc::clone(prepared.program()),
        scope.clone(),
        Rc::clone(&owner),
    );
    let partials = [Partial {
        relation,
        cause: PartialCause("missing source rows".into()),
        lacking: Lacking::Any,
        at: Some(FileId(0)),
    }];
    let project_partials = Partials::default();
    let completion = if matches!(case, SourceCase::Project) {
        CompletionPolicy::project_inputs(RuleIndex(0), &project_partials, &project_partials)
    } else {
        CompletionPolicy::file_inputs(
            RuleIndex(0),
            FileId(0),
            &[],
            if matches!(case, SourceCase::Partial) {
                &partials
            } else {
                &[]
            },
        )
    };
    let table = Table {
        kinds: kinds(Relation::ManifestDiagnostic).to_vec(),
        ..Table::default()
    };
    let reads = Counter::new(0);
    let read = |input| {
        reads.set(reads.get() + 1);
        match input {
            Relation::SyntaxTree => Ok(Rows::Tree(rows_over(&elements))),
            Relation::ManifestDiagnostic => Ok(Rows::Computed(&table)),
            _ => Err(OperatingError::NotRead),
        }
    };
    let kernels = KernelSession::new(&engine, &prepared, Rc::clone(&owner), &read, None);
    let mut observed = std::collections::HashMap::new();
    let mut sources = super::super::SourceInputs {
        execution: &execution,
        completion: completion.clone(),
        owner: Rc::clone(&owner),
        read: &read,
        kernels: &kernels,
        observations: &mut observed,
        file_outputs: None,
        program: &prepared,
        rule: RuleIndex(0),
        project: None,
    };
    let mut run_context = RunContext::file(&services, SMALL_SOURCE, &mut symbols, None, None);
    let mut context = run_context.borrow(&mut symbols, &prepared, RuleIndex(0))?;
    if matches!(case, SourceCase::Hosted | SourceCase::Parameter) {
        let root = *prepared.program().rules()[0]
            .emit
            .as_ref()
            .map_err(Clone::clone)?;
        let InvocationOutcome::Published(admitted) = execution.invoke(
            root,
            &mut sources,
            &kernels,
            &completion,
            &mut context,
            Delivery::Retain,
        )?
        else {
            return Err(OperatingError::NotRun);
        };
        contributor_readiness(&project, &[admitted.clone()], &owner)?;
        assert_eq!(count_of(&admitted.rows(&owner, &read)?).0, 1);
        assert_eq!(reads.get(), 0);
        return Ok("invariant");
    }
    let requested = if matches!(case, SourceCase::ProjectRefusal) {
        &project
    } else {
        &scope
    };
    let result = sources.admit(
        crate::domain::executable_program::types::InputSource::Relation(relation),
        requested,
        &mut context,
    );
    if matches!(case, SourceCase::ProjectRefusal) {
        assert_eq!(reads.get(), 0);
    }
    let admitted = result?;
    assert_eq!(reads.get(), 1);
    match admitted.coverage()? {
        crate::application::file_dag::completion::CoverageState::Complete => Ok("complete"),
        crate::application::file_dag::completion::CoverageState::Partial { files } => {
            assert_eq!(files, vec![Some(FileId(0))]);
            Ok("partial")
        }
    }
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("exact_file_source_is_admitted"),
        word: Word::Succeeds,
        input: || SourceCase::File,
        expect: Expect::Value("complete"),
    },
    Case {
        name: CaseName("one_file_source_cannot_be_retagged_as_two_file_project"),
        word: Word::Refuses,
        input: || SourceCase::ProjectRefusal,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("declared_project_source_uses_actual_request"),
        word: Word::Succeeds,
        input: || SourceCase::Project,
        expect: Expect::Value("complete"),
    },
    Case {
        name: CaseName("physical_partial_source_never_becomes_complete"),
        word: Word::Succeeds,
        input: || SourceCase::Partial,
        expect: Expect::Value("partial"),
    },
    Case {
        name: CaseName("hosted_output_broadcasts_from_request_owned_population"),
        word: Word::Succeeds,
        input: || SourceCase::Hosted,
        expect: Expect::Value("invariant"),
    },
    Case {
        name: CaseName("rows_output_broadcasts_from_request_owned_parameter"),
        word: Word::Succeeds,
        input: || SourceCase::Parameter,
        expect: Expect::Value("invariant"),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
