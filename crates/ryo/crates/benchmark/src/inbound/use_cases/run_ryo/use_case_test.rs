//! Receive, engine and response failures stop the pipeline at the failed boundary.
use super::fixtures::{CountingContext, Engine, Failing, ForwardingPort, report, sealed_request};
use super::{interface::RunRyoUseCase as _, use_case::RunRyoUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::execution::types::{InProcessWorker, StoredRequest};
use crate::outbound::worker::interface::WorkerPort as _;
use crate::outbound::{
    engine::interface::EnginePort,
    worker::fixtures::{data::request, fake::WorkerFake},
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
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
    /// The request run in the bench's own process: the use case as the in-process worker.
    InProcess,
    /// The request compiled ahead in the bench's own process, the engine refusing it.
    Uncompiled,
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
    Compile(Trace),
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
    Case {
        name: CaseName("run in the bench's own process the request answers the encoded report"),
        word: Word::Succeeds,
        input: || Stage::InProcess,
        expect: Expect::Value(Trace {
            received: 0,
            loaded: 1,
            closed: 1,
            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
            responded: 0,
            published: true,
            seam: (0, 0, false),
        }),
    },
    Case {
        name: CaseName(
            "an engine that refuses to compile a request ahead refuses the in-process compile",
        ),
        word: Word::Fails,
        input: || Stage::Uncompiled,
        expect: Expect::Error(Refusal::Compile(Trace {
            received: 0,
            loaded: 0,
            closed: 0,
            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
            responded: 0,
            published: false,
            seam: (0, 0, false),
        })),
    },
];
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
        failing: match stage {
            Stage::Load => Failing::Load,
            Stage::Configure => Failing::Configure,
            Stage::Engine => Failing::Execute,
            Stage::Uncompiled => Failing::Compile,
            Stage::Complete
            | Stage::Forward
            | Stage::Retarget
            | Stage::Receive
            | Stage::Respond
            | Stage::Sealed
            | Stage::InProcess => Failing::Nothing,
        },
    });
    let port: Arc<dyn EnginePort + Send + Sync> =
        if matches!(stage, Stage::Forward | Stage::Retarget) {
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
    let use_case = RunRyoUseCaseImpl::new(
        port,
        Arc::<WorkerFake>::clone(&transport),
        Arc::<CountingContext>::clone(&context),
    );
    let stored = StoredRequest(std::path::PathBuf::from("request.json"));
    // In the bench's own process the report is answered as bytes, not published: the bytes are the fake's encoding of the report.
    let mut answered = None;
    let result = match stage {
        Stage::InProcess => InProcessWorker::run(&use_case, &stored).map(|response| {
            answered = Some(response.0);
        }),
        Stage::Uncompiled => InProcessWorker::compile(&use_case, &stored).map(|_times| ()),
        Stage::Complete
        | Stage::Forward
        | Stage::Retarget
        | Stage::Receive
        | Stage::Engine
        | Stage::Load
        | Stage::Configure
        | Stage::Respond
        | Stage::Sealed => use_case.run_ryo(),
    };
    let reports = transport.reports();
    let expected = report(request().files);
    let trace = Trace {
        received: transport.receive_calls(),
        loaded: engine.loaded.load(Ordering::SeqCst),
        closed: engine.closed.load(Ordering::SeqCst),
        evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(
            engine.calls.load(Ordering::SeqCst),
        ),
        responded: transport.respond_calls(),
        published: answered.map_or_else(
            || {
                reports
                    .iter()
                    .map(|report| RunReport {
                        project_context:
                            ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
                        ..report.clone()
                    })
                    .eq([expected.clone()])
            },
            |bytes| {
                transport
                    .encoded(&expected)
                    .is_ok_and(|encoded| encoded.0 == bytes)
            },
        ),
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
        Error::Invalid(message) if message == "fixture compile failure" => Refusal::Compile(trace),
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
