//! The in-process implementation runs a worker request through the language's worker and refuses every other invocation:
//! the shared contract's child invocations are the process implementation's, which the composition chooses for them.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::execution::interface::{ExecutionPort as _, InProcessWorker};
use crate::outbound::execution::types::{RuleCompileTimes, StoredRequest};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::constants::{REQUEST_FILE, RUN_RYO};
/// Which invocation runs, and whether the worker completes it.
#[derive(Clone, Copy, Debug)]
enum Input {
    /// A worker request the worker completes.
    Completed,
    /// A worker request the worker refuses.
    Refused,
    /// A child invocation that is no worker request.
    Child,
}
/// The exit status and the captured stdout.
type Value = (i32, Vec<u8>);
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a completed worker request exits 0 with its response as stdout"),
        word: Word::Succeeds,
        input: || Input::Completed,
        expect: Expect::Value((0, b"response".to_vec())),
    },
    Case {
        name: CaseName("a refused worker request exits 1 with no response"),
        word: Word::Succeeds,
        input: || Input::Refused,
        expect: Expect::Value((1, Vec::new())),
    },
    Case {
        name: CaseName("an invocation that is no worker request is refused"),
        word: Word::Refuses,
        input: || Input::Child,
        expect: Expect::Error(()),
    },
];
/// A worker that answers or refuses each request as its input says.
#[derive(Debug)]
struct Worker(Input);
impl InProcessWorker for Worker {
    fn run(
        &self,
        _request: &StoredRequest,
        mut response: Box<dyn std::io::Write + Send>,
    ) -> Result<(), ReportsError> {
        match self.0 {
            Input::Refused => Err(ReportsError::Invalid("refused".into())),
            Input::Completed | Input::Child => Ok(response.write_all(b"response")?),
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
        Input::Child => vec!["-c".into(), "true".into()],
        Input::Completed | Input::Refused => {
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
        .map(|output| (output.status.0, output.stdout.0))
        .map_err(|_refused| ())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
