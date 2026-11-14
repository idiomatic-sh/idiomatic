//! The subject of the source partition cases: one project phase run over the partitioned sources.
use super::*;
use std::borrow::Cow;
/// How a source was admitted: the admission's name, and where the case reads them, the admitted rows, the provider
/// reads made, and the files a partial admission covers.
#[derive(Debug, PartialEq)]
pub(crate) struct Admission {
    pub(crate) how: &'static str,
    pub(crate) rows: Option<usize>,
    pub(crate) reads: Option<usize>,
    pub(crate) partial: Cow<'static, [Option<FileId>]>,
}
/// A refused admission, and the provider reads made before a refused project was answered.
#[derive(Debug, PartialEq)]
pub(crate) struct Refused {
    pub(crate) error: OperatingError,
    pub(crate) reads_before: Option<usize>,
}
/// An admission named `how` with nothing else read.
const fn named(how: &'static str) -> Admission {
    Admission {
        how,
        rows: None,
        reads: None,
        partial: Cow::Borrowed(&[]),
    }
}
/// Runs the case's project phase over the partitioned sources and answers how it was admitted.
pub(crate) fn admit(case: SourceCase) -> Result<Admission, Refused> {
    let mut reads_before = None;
    admitted(case, &mut reads_before).map_err(|error| Refused {
        error,
        reads_before,
    })
}
/// The admission, recording in `reads_before` the provider reads a refused project made before its answer.
fn admitted(
    case: SourceCase,
    reads_before: &mut Option<usize>,
) -> Result<Admission, OperatingError> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let mut source_plan = plan(ProgramShape::Words);
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
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: std::iter::IntoIterator::into_iter(vec![source_plan])
                .map(std::sync::Arc::new)
                .collect(),
            bindings: vec![bindings],
        },
        &[rule()],
        &[Digest([0; 16])],
        &hosted,
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let operators: Arc<dyn crate::outbound::operators::interface::OperatorsPort> =
        Arc::new(crate::application::file_dag::fixtures::operators());
    let engine: Arc<dyn crate::application::file_dag::executor::service::PrimitiveExecutor + Send> =
        Arc::new(CommonOperatorEngine::new(operators));
    let corpus = ["a", "b"].map(|path| {
        CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath(path.into()),
            size: FileSize(SMALL_SOURCE.len() as u64),
            modified: Nanoseconds(0),
        })
    });
    let services = ProgramServices::new(Arc::clone(&engine), Arc::clone(&derived), &corpus);
    let required = required_partitions(
        &prepared,
        RuleIndex(0),
        &corpus,
        &[],
        &[],
        &[],
        &[],
        &services,
    )?;
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
                &[],
                &one_file,
            )?
        } else {
            required.clone()
        };
        let handover = crate::application::file_dag::frontiers::fixtures::prepared_frontiers(
            RuleIndex(0),
            Arc::new([]),
            vec![].into(),
            Arc::clone(prepared.program()),
            actual,
            vec![].into(),
        );
        handover.validate_scope(prepared.program(), &required)?;
        return Ok(named("frontier-scope"));
    }
    let owner = open_program_owner(
        ryo_execution_store::application::file_dag::relations::types::ProducerCapacity(
            prepared.program().regions().len(),
        ),
        Arc::clone(&derived),
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        ryo_execution_store::application::file_dag::route_counts::profile::service::RouteProfile::default(),
    );
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
        at: ryo_execution_domain::domain::coverage::types::PartialPlace::File(FileId(0)),
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
        open_program_owner(ryo_execution_store::application::file_dag::relations::types::ProducerCapacity(prepared.program().regions().len()), Arc::clone(&derived), ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible, ryo_execution_store::application::file_dag::route_counts::profile::service::RouteProfile::default())
    } else {
        Rc::clone(&owner)
    };
    let walk = super::super::super::super::super::native_walk::service::FileWalk::new(
        FileGeneration::from_owner(FileId(0), &walk_owner),
    );
    let project_walk =
        crate::application::file_dag::program::native_walk::service::project::service::ProjectWalk::new();
    // The project stage admits only the heads of a rule its cohort declared.
    project_walk.owe(
        &prepared,
        RuleIndex(0),
        crate::application::file_dag::program::native_walk::types::Deliveries(1),
    );
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
    let mut observed = rustc_hash::FxHashMap::default();
    let mut sources =
        crate::application::file_dag::program::facade_runtime::source_admission::service::SourceInputs {
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
        return Ok(named("scope-admitted"));
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
        contributed_proof(&project, &[admitted.clone()], &owner)?;
        return Ok(Admission {
            rows: Some(count_of(&admitted.rows(&owner, &read)?).0),
            reads: Some(reads.get()),
            ..named("invariant")
        });
    }
    let requested = if matches!(case, SourceCase::ProjectRefusal) {
        &project
    } else {
        &scope
    };
    let result = sources.admit_source(
        ryo_execution_domain::domain::executable_program::types::InputSource::Relation(relation),
        requested,
        &mut context,
    );
    if matches!(case, SourceCase::ProjectRefusal) {
        *reads_before = Some(reads.get());
    }
    let admitted = result?;
    let reads = Some(reads.get());
    Ok(match admitted.coverage().clone() {
        ryo_execution_domain::domain::completion::types::CoverageState::Complete => Admission {
            reads,
            ..named("complete")
        },
        ryo_execution_domain::domain::completion::types::CoverageState::Partial {
            files, ..
        } => Admission {
            reads,
            partial: files.into(),
            ..named("partial")
        },
        ryo_execution_domain::domain::completion::types::CoverageState::Inapplicable { .. } => {
            Admission {
                reads,
                ..named("inapplicable")
            }
        }
    })
}
