//! Receive, engine and response failures stop the pipeline at the failed boundary.
use super::fixtures::{CountingContext, sealed_request};
use super::{interface::RunRyoUseCase as _, use_case::RunRyoUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::engine::types::loaded_engine_session::value::EngineExecution;
use crate::outbound::{
    engine::interface::EnginePort,
    worker::fixtures::{data::request, fake::WorkerFake},
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
#[derive(Clone, Copy, Debug)]
enum Stage {
    Complete,
    Forward,
    Retarget,
    Receive,
    Engine,
    Load,
    Configure,
    Respond,
    Sealed,
}
type Input = Stage;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Trace {
    received: usize,
    loaded: usize,
    closed: usize,
    evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount,
    responded: usize,
    published: bool,
    /// Context validations, context receipts, and whether the published report observed its context.
    seam: (usize, usize, bool),
}
type Value = Trace;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Receive(Trace),
    Engine(Trace),
    Load(Trace),
    Configure(Trace),
    Respond(Trace),
    Association(Trace),
    Unexpected,
}
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a sealed request publishes its receipt without validating the corpus"),
        word: Word::Succeeds,
        input: || Stage::Sealed,
        expect: Expect::Value(Trace {
            received: 1,
            loaded: 1,
            closed: 1,
            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
            responded: 1,
            published: true,
            seam: (0, 1, true),
        }),
    },
    Case {
        name: CaseName("genuine externally forwarded loaded handle executes the received request"),
        word: Word::Succeeds,
        input: || Stage::Forward,
        expect: Expect::Value(Trace {
            received: 1,
            loaded: 1,
            closed: 1,
            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
            responded: 1,
            published: true,
            seam: (0, 0, false),
        }),
    },
    Case {
        name: CaseName(
            "forwarding a handle loaded for another request refuses before execute or respond",
        ),
        word: Word::Fails,
        input: || Stage::Retarget,
        expect: Expect::Error(Refusal::Association(Trace {
            received: 1,
            loaded: 1,
            closed: 1,
            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
            responded: 0,
            published: false,
            seam: (0, 0, false),
        })),
    },
    Case {
        name: CaseName("load refusal prevents configuration execution and response"),
        word: Word::Fails,
        input: || Stage::Load,
        expect: Expect::Error(Refusal::Load(Trace {
            received: 1,
            loaded: 1,
            closed: 0,
            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
            responded: 0,
            published: false,
            seam: (0, 0, false),
        })),
    },
    Case {
        name: CaseName("configuration refusal closes the session without SDK execution"),
        word: Word::Fails,
        input: || Stage::Configure,
        expect: Expect::Error(Refusal::Configure(Trace {
            received: 1,
            loaded: 1,
            closed: 1,
            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
            responded: 0,
            published: false,
            seam: (0, 0, false),
        })),
    },
    Case {
        name: CaseName("receive engine respond publishes the actual report"),
        word: Word::Succeeds,
        input: || Stage::Complete,
        expect: Expect::Value(Trace {
            received: 1,
            loaded: 1,
            closed: 1,
            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
            responded: 1,
            published: true,
            seam: (0, 0, false),
        }),
    },
    Case {
        name: CaseName("receive failure prevents engine and response"),
        word: Word::Fails,
        input: || Stage::Receive,
        expect: Expect::Error(Refusal::Receive(Trace {
            received: 1,
            loaded: 0,
            closed: 0,
            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
            responded: 0,
            published: false,
            seam: (0, 0, false),
        })),
    },
    Case {
        name: CaseName("engine failure prevents response"),
        word: Word::Fails,
        input: || Stage::Engine,
        expect: Expect::Error(Refusal::Engine(Trace {
            received: 1,
            loaded: 1,
            closed: 1,
            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
            responded: 0,
            published: false,
            seam: (0, 0, false),
        })),
    },
    Case {
        name: CaseName("response failure cannot report success"),
        word: Word::Fails,
        input: || Stage::Respond,
        expect: Expect::Error(Refusal::Respond(Trace {
            received: 1,
            loaded: 1,
            closed: 1,
            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
            responded: 1,
            published: false,
            seam: (0, 0, false),
        })),
    },
];
#[derive(Debug)]
struct Engine {
    calls: AtomicUsize,
    loaded: AtomicUsize,
    closed: AtomicUsize,
    stage: Stage,
}
struct Session<'a> {
    engine: &'a Engine,
    request: &'a WorkerRequest,
}
impl Drop for Session<'_> {
    fn drop(&mut self) {
        self.engine.closed.fetch_add(1, Ordering::SeqCst);
    }
}
impl EnginePort for Engine {
    fn load<'a>(
        &'a self,
        request: &'a WorkerRequest,
    ) -> Result<
        crate::outbound::engine::types::loaded_engine_session::value::LoadedEngineSession<'a>,
        Error,
    > {
        self.loaded.fetch_add(1, Ordering::SeqCst);
        if matches!(self.stage, Stage::Load) {
            return Err(Error::Invalid("fixture load failure".into()));
        }
        let rules = if matches!(self.stage, Stage::Configure) {
            Default::default()
        } else {
            crate::outbound::engine::fixtures::fake::loaded_rules(request)?
        };
        Ok(
            crate::outbound::engine::types::loaded_engine_session::value::LoadedEngineSession::new(
                request,
                rules,
                None,
                Box::new(Session {
                    engine: self,
                    request,
                }),
            ),
        )
    }
}
impl EngineExecution for Session<'_> {
    fn configuration_files(&self) -> Result<Vec<ryo::linter::RelativePath>, Error> {
        Ok(vec![])
    }
    fn execute(
        self: Box<Self>,
        _rules: std::collections::BTreeMap<ryo::linter::RuleId, ryo::linter::Rule>,
        configured: crate::outbound::engine::types::EngineConfiguration,
    ) -> Result<crate::outbound::engine::types::EngineRun, Error> {
        assert_eq!(
            self.engine.closed.load(Ordering::SeqCst),
            0,
            "session stays live throughout SDK execution"
        );
        self.engine.calls.fetch_add(1, Ordering::SeqCst);
        if matches!(self.engine.stage, Stage::Engine) {
            return Err(Error::Invalid("fixture engine failure".into()));
        }
        assert_eq!(
            configured.selected_pairs,
            crate::outbound::engine::types::SelectedPairs(1)
        );
        assert!(configured.hosted.is_empty());
        Ok(crate::outbound::engine::types::EngineRun {
            dependencies: Vec::new(),
            report: RunReport {
                execution_metadata:
                    ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
                project_context:
                    ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
                files: self.request.files.clone(),
                skipped: Vec::new(),
                coverage:
                    ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(
                        ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                            selected: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(
                                1,
                            ),
                            skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                            unknown_or_unread:
                                ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                        },
                    ),
                diagnostics: Vec::new(),
            },
        })
    }
}
// This wrapper uses only the public EnginePort contract: it cannot inspect or mutate the handle.
struct ForwardingPort {
    inner: Arc<Engine>,
    other: WorkerRequest,
    retarget: bool,
}
impl EnginePort for ForwardingPort {
    fn load<'a>(
        &'a self,
        request: &'a WorkerRequest,
    ) -> Result<
        crate::outbound::engine::types::loaded_engine_session::value::LoadedEngineSession<'a>,
        Error,
    > {
        self.inner
            .load(if self.retarget { &self.other } else { request })
    }
}
fn subject(stage: Input) -> Result<Value, Failure> {
    let mut transport = WorkerFake::new(if matches!(stage, Stage::Sealed) {
        sealed_request()
    } else {
        request()
    });
    transport.refuse_receive = matches!(stage, Stage::Receive);
    transport.refuse_respond = matches!(stage, Stage::Respond);
    let transport = Arc::new(transport);
    let engine = Arc::new(Engine {
        calls: AtomicUsize::new(0),
        loaded: AtomicUsize::new(0),
        closed: AtomicUsize::new(0),
        stage,
    });
    let port: Arc<dyn EnginePort> = if matches!(stage, Stage::Forward | Stage::Retarget) {
        let mut other = request();
        other.files = vec![ryo::linter::RelativePath("other.txt".into())];
        Arc::new(ForwardingPort {
            inner: Arc::clone(&engine),
            other,
            retarget: matches!(stage, Stage::Retarget),
        })
    } else {
        Arc::<Engine>::clone(&engine)
    };
    let context = Arc::new(CountingContext::default());
    let result = RunRyoUseCaseImpl::new(
        port,
        Arc::<WorkerFake>::clone(&transport),
        Arc::<CountingContext>::clone(&context),
    )
    .run_ryo();
    let reports = transport.reports();
    let trace = Trace {
        received: transport.receive_calls(),
        loaded: engine.loaded.load(Ordering::SeqCst),
        closed: engine.closed.load(Ordering::SeqCst),
        evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(
            engine.calls.load(Ordering::SeqCst),
        ),
        responded: transport.respond_calls(),
        published: reports
            .iter()
            .map(|report| RunReport {
                project_context:
                    ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
                ..report.clone()
            })
            .eq([RunReport {
                execution_metadata:
                    ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
                project_context:
                    ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
                files: request().files,
                skipped: Vec::new(),
                coverage:
                    ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(
                        ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                            selected: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(
                                1,
                            ),
                            skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                            unknown_or_unread:
                                ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                        },
                    ),
                diagnostics: Vec::new(),
            }]),
        seam: (
            context.validated.load(Ordering::SeqCst),
            context.receipted.load(Ordering::SeqCst),
            matches!(reports.as_slice(), [report] if matches!(report.project_context, ryo_benchmark_contracts::domain::reports::types::ProjectContext::Observed(_))),
        ),
    };
    result.map(|()| trace).map_err(|error| match error {
        Error::Invalid(message)
            if message == "loaded engine session belongs to another received request" =>
        {
            Refusal::Association(trace)
        }
        Error::Invalid(message) if message == "fixture receive failure" => Refusal::Receive(trace),
        Error::Invalid(message) if message == "fixture engine failure" => Refusal::Engine(trace),
        Error::Invalid(message) if message == "fixture load failure" => Refusal::Load(trace),
        Error::Invalid(message)
            if message.starts_with("selected or required producer has no registered query:") =>
        {
            Refusal::Configure(trace)
        }
        Error::Io(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
            Refusal::Respond(trace)
        }
        Error::Invalid(_) | Error::Io(_) | Error::Json(_) | Error::Toml(_) => Refusal::Unexpected,
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
