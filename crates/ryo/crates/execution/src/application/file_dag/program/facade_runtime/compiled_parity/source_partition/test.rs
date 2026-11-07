//! Producer declarations and actual source scope govern ordinary admission.
use super::super::*;
use crate::application::file_dag::program::admission::{
    FileGeneration, Inputs, Partition, ProducerRoot, contributor_readiness,
};
use crate::application::file_dag::program::context::RunContext;
use crate::application::file_dag::program::kernel_session::KernelSession;
use crate::application::file_dag::program::runtime::service::{
    InvocationOutcome, ProgramExecution,
};
use crate::domain::completion::types::completion_policy::value::CompletionPolicy;
use crate::domain::coverage::types::{Lacking, Partial, Partials, RuleIndex};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::{HostedNumber, ParameterId, ParameterName, TableKey},
    query_bindings::types::{ParameterType, PlanParameter},
    stored_facts::types::PartialCause,
};
use std::{cell::Cell as Counter, rc::Rc};
use super::fixtures::{CASES, SourceCase};
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
    let project_case = matches!(
        case,
        SourceCase::Project | SourceCase::ProjectAsFile | SourceCase::ProjectInvocation
    );
    let relation = if project_case {
        Relation::ManifestDiagnostic
    } else {
        Relation::SyntaxTree
    };
    if project_case {
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
                ryo_values::domain::row_texts::types::Cell::Number(Integer(42)),
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
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
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
    let engine: Arc<dyn crate::application::file_dag::executor::PrimitiveExecutor + Send> =
        Arc::new(CommonOperatorEngine::new(
            (std::sync::Arc::new(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
            )
                as std::sync::Arc<dyn crate::outbound::operators::interface::OperatorsPort>),
        ));
    let corpus = ["a", "b"].map(|path| {
        CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath(path.into()),
            size: FileSize(SMALL_SOURCE.len() as u64),
            modified: Nanoseconds(0),
        })
    });
    let services = ProgramServices::new(Arc::clone(&engine), Arc::clone(&derived), &corpus);
    let required = required_partitions(&prepared, RuleIndex(0), &corpus, &[], &[], &[], &services)?;
    if matches!(
        case,
        SourceCase::FrontierRefusal | SourceCase::FrontierScope
    ) {
        let one_file =
            ProgramServices::new(Arc::clone(&engine), Arc::clone(&derived), &corpus[..1]);
        let actual = if matches!(case, SourceCase::FrontierRefusal) {
            required_partitions(
                &prepared,
                RuleIndex(0),
                &corpus[..1],
                &[],
                &[],
                &[],
                &one_file,
            )?
        } else {
            required.clone()
        };
        let handover = crate::application::file_dag::frontiers::types::PreparedFrontiers {
            producer: RuleIndex(0),
            sources: std::sync::Arc::new([]),
            rows: vec![].into(),
            refused: vec![].into(),
            program: Arc::clone(prepared.program()),
            required: actual,
        };
        handover.validate_scope(prepared.program(), &required)?;
        return Ok("frontier-scope");
    }
    let owner = open_program_owner(prepared.program(), Arc::clone(&derived), crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible);
    let project = Partition::Project(required);
    let scope = if project_case {
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
    let completion = if project_case {
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
    let walk_owner = if matches!(case, SourceCase::ForeignWalkGeneration) {
        open_program_owner(prepared.program(), Arc::clone(&derived), crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible)
    } else {
        Rc::clone(&owner)
    };
    let walk = super::super::super::super::native_walk::FileWalk::new(FileGeneration::from_owner(
        FileId(0),
        &walk_owner,
    ));
    let project_walk =
        crate::application::file_dag::program::native_walk::project::service::ProjectWalk::new();
    let kernels = match (&scope, case) {
        (_, SourceCase::FileAsProject) => KernelSession::project(
            engine.as_ref(),
            &prepared,
            Rc::clone(&owner),
            &read,
            &project_walk,
        ),
        (_, SourceCase::ProjectAsFile) => {
            KernelSession::file(engine.as_ref(), &prepared, Rc::clone(&owner), &read, &walk)
        }
        (Partition::File(_), _) => {
            KernelSession::file(engine.as_ref(), &prepared, Rc::clone(&owner), &read, &walk)
        }
        _ => KernelSession::project(
            engine.as_ref(),
            &prepared,
            Rc::clone(&owner),
            &read,
            &project_walk,
        ),
    };
    let mut observed = std::collections::HashMap::new();
    let mut sources =
        crate::application::file_dag::program::facade_runtime::source_admission::SourceInputs {
            execution: &execution,
            state: execution.file_state(),
            completion: completion.clone(),
            owner: Rc::clone(&owner),
            read: &read,
            observations: &mut observed,
            file_outputs: None,
            program: &prepared,
            rule: RuleIndex(0),
            project: None,
        };
    let mut run_context = RunContext::file(
        &services,
        SourceView(SMALL_SOURCE),
        &mut symbols,
        None,
        None,
    );
    let mut context = run_context.borrow(&mut symbols, &prepared, RuleIndex(0))?;
    if matches!(
        case,
        SourceCase::FileAsProject
            | SourceCase::ProjectAsFile
            | SourceCase::ForeignWalkGeneration
            | SourceCase::FileInvocation
            | SourceCase::ProjectInvocation
    ) {
        let root = prepared.program().rules()[0].emit.clone()?;
        execution.invoke(
            execution.file_state(),
            root,
            &mut sources,
            &kernels,
            &completion,
            &mut context,
        )?;
        return Ok("scope-admitted");
    }
    if matches!(case, SourceCase::Hosted | SourceCase::Parameter) {
        let root = *prepared.program().rules()[0]
            .emit
            .as_ref()
            .map_err(Clone::clone)?;
        let InvocationOutcome::Published(admitted) = execution.invoke(
            execution.file_state(),
            root,
            &mut sources,
            &kernels,
            &completion,
            &mut context,
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
    let result = sources.admit_source(
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
        crate::domain::completion::types::CoverageState::Complete => Ok("complete"),
        crate::domain::completion::types::CoverageState::Partial { files, .. } => {
            assert_eq!(files, vec![Some(FileId(0))]);
            Ok("partial")
        }
        crate::domain::completion::types::CoverageState::Inapplicable { .. } => Ok("inapplicable"),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::query_bindings::types::{BindingCell, BindingTable, ParameterBinding};

use crate::application::file_dag::program::facade_runtime::service::required_partitions;
use crate::application::file_dag::scope::open_program_owner;
use crate::domain::table_model::service::count_of;
