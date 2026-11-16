//! The in-process implementation runs a worker request through the language's worker and refuses every other invocation:
//! the shared contract's child invocations are the process implementation's, which the composition chooses for them.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::execution::interface::ExecutionPort as _;
use crate::outbound::execution::types::{InProcessWorker, RuleCompileTimes, StoredRequest};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::constants::{REQUEST_FILE, RUN_RYO};
/// Which invocation runs, and whether the worker completes it.
#[derive(Clone, Copy, Debug)]
enum Worked {
    /// A worker request the worker completes.
    Completed,
    /// A worker request the worker refuses.
    Refused,
    /// A child invocation that is no worker request.
    Child,
}
type Input = Worked;
/// The exit status, and whether stdout holds exactly the worker's response.
type Value = (i32, bool);
type Failure = ();
/// What the worker answers a request it completes with.
const RESPONSE: &[u8] = b"response";
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a completed worker request exits 0 with its response as stdout"),
        word: Word::Succeeds,
        input: || Worked::Completed,
        expect: Expect::Value((0, true)),
    },
    Case {
        name: CaseName("a refused worker request exits 1 with no response"),
        word: Word::Succeeds,
        input: || Worked::Refused,
        expect: Expect::Value((1, false)),
    },
    Case {
        name: CaseName("an invocation that is no worker request is refused"),
        word: Word::Refuses,
        input: || Worked::Child,
        expect: Expect::Error(()),
    },
];
/// A worker that answers or refuses each request as its input says.
#[derive(Debug)]
struct Worker(Worked);
impl InProcessWorker for Worker {
    fn run(
        &self,
        _request: &StoredRequest,
        mut response: Box<dyn std::io::Write + Send>,
    ) -> Result<(), ReportsError> {
        match self.0 {
            Worked::Refused => Err(ReportsError::Invalid("refused".into())),
            Worked::Completed | Worked::Child => Ok(response.write_all(RESPONSE)?),
        }
    }
    fn compile(&self, _request: &StoredRequest) -> Result<RuleCompileTimes, ReportsError> {
        Ok(RuleCompileTimes::default())
    }
}
fn subject(input: Input) -> Result<Value, Failure> {
    let worker = Worker(input);
    let adapter = crate::outbound::execution::factory::in_process(&worker);
    let arguments = match input {
        Worked::Child => vec!["-c".into(), "true".into()],
        Worked::Completed | Worked::Refused => {
            vec![RUN_RYO.into(), REQUEST_FILE.into(), "request.json".into()]
        }
    };
    adapter
        .run(Invocation {
            executable: "/bin/sh".into(),
            arguments,
            environment: std::collections::BTreeMap::new(),
            directory: std::env::temp_dir(),
        })
        .map(|output| (output.status.0, output.stdout.0 == RESPONSE))
        .map_err(|_refused| ())
}
#[test]
fn succeeds() {
    let worker = Worker(Worked::Completed);
    crate::outbound::execution::fixtures::contract::check_with(
        &crate::outbound::execution::factory::in_process(&worker),
        &Invocation {
            executable: "/bin/sh".into(),
            arguments: vec!["-c".into(), "true".into()],
            environment: std::collections::BTreeMap::new(),
            directory: std::env::temp_dir(),
        },
    );
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
