# outbound/scheduler
STATUS: drafted (28 Nov 2026; not reviewed, not agreed). No line names a table type, so nothing here waits on the table spike.

Bead: ryo-fmx3.1.13.23.4.17. Revisions: MAIN = f4ae3bd802, TIP = 2a207464c, parked port branch = d2502dca4 (one commit
off the tip, `claude/wayback-u1-scheduler-port`). All reads are `git -C /workspaces/idiomatic-public-benchmark show`.

## 1. Owns
Running a set of numbered jobs on workers. One thing. The worker count is decided once at the edge and handed to the
factory; how jobs are cut (by file, by shard, by weight) is the caller's; what a worker keeps between jobs is the
worker's own closure.

## 2. On main (f4ae3bd802)
**Where.** `crates/ryo/crates/core/src/outbound/scheduler/`: 207 Rust lines without tests and fixtures
(interface 20, types 14, errors 15, factory 27, adapters 4, rayon 6 + 90, sequential 6 + 25), plus 144 lines of
fixtures behind the `fixtures` feature (contract 73, fake 67, mod 4).

**The contract, verbatim.**
```
pub trait SchedulerPort: Send + Sync + std::fmt::Debug {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/interface.rs:8
workers: &mut [Box<dyn FnMut(JobNumber) + Send + '_>],  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/interface.rs:17
count: JobCount,  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/interface.rs:18
) -> Result<(), SchedulerError>;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/interface.rs:19
pub struct JobCount(pub u32);  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/types.rs:6
pub struct JobNumber(pub u32);  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/types.rs:10
pub struct WorkerCount(pub u32);  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/types.rs:14
pub enum SchedulerError {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/errors.rs:5
Unavailable {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/errors.rs:8
NoWorkers,  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/errors.rs:14
pub fn rayon(workers: WorkerCount) -> impl crate::outbound::scheduler::interface::SchedulerPort {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/factory.rs:11
pub fn rayon_over<P: AsRef<rayon::ThreadPool> + Send + Sync + 'static>(  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/factory.rs:17
pub fn sequential() -> impl crate::outbound::scheduler::interface::SchedulerPort {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/factory.rs:25
```
The doc of `run`: "A worker is called by one thread at a time and keeps what it owns between jobs." Each worker is
its own boxed closure, so per-worker state (buffers, tables, a store) is owned by that closure and no job takes a lock.

**Public items: 9** (`SchedulerPort`, `run`, `JobCount`, `JobNumber`, `WorkerCount`, `SchedulerError`, `rayon`,
`rayon_over`, `sequential`), plus 4 behind `fixtures` (`contract::CASES`, `contract::check`, `fake::SchedulerFake`,
`fake::CountingSchedulerFake`).

**Who implements it.** `SchedulerRayonAdapter` (every worker drains one atomic queue inside a `pool.scope`):
```
pub(crate) struct SchedulerRayonAdapter {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/adapters/rayon/adapter.rs:22
pool.scope(|scope| {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/adapters/rayon/adapter.rs:61
let job = next.fetch_add(1, Ordering::Relaxed);  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/adapters/rayon/adapter.rs:66
pub(crate) struct SchedulerSequentialAdapter;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/adapters/sequential/adapter.rs:9
pub struct SchedulerFake {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/fixtures/fake.rs:11
pub struct CountingSchedulerFake {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/scheduler/fixtures/fake.rs:39
```

**Who calls it (production files).** core 8 (`file_analysis/streaming/service.rs`, `file_analysis/project/service.rs`,
`file_analysis/service.rs`, `storing/service.rs`, `storing/errors.rs`, `cross_file/errors.rs`,
`inbound/use_cases/lint/use_case.rs`, `lint/errors.rs`); SDK 3 (`domain/capacity/types.rs`, `lint/composition.rs`,
`lint/types.rs`); command line 1 (`query/run/composition.rs`). The three run sites, each once per set:
```
self.scheduler.run(&mut jobs, count)?;  @f4ae3bd802:crates/ryo/crates/core/src/application/file_analysis/streaming/service.rs:130
self.scheduler.run(&mut workers, count)?;  @f4ae3bd802:crates/ryo/crates/core/src/application/storing/service.rs:108
.run(std::slice::from_mut(&mut job), JobCount(1))?;  @f4ae3bd802:crates/ryo/crates/core/src/application/file_analysis/project/service.rs:235
```
The first gives one job per file (`count` = the run's files); the third runs one job on the pool so that a language's
own parallel work (Rust's `JobSchedulingPort`) runs on the edge's pool, never a pool of its own. Main's `storing` also
writes the ordered-collect helper every caller needs (decision 2):
```
fn spread<T>(  @f4ae3bd802:crates/ryo/crates/core/src/application/storing/service.rs:81
```
Built at the edges and handed in:
```
Arc::new(scheduler::factory::rayon(workers))  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:187
Self::scheduler(Arc::new(scheduler::factory::rayon_over(pool)), workers)  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:268
pub struct WorkerPool(Arc<dyn SchedulerPort>, WorkerCount);  @f4ae3bd802:crates/ryo/crates/sdk/src/domain/capacity/types.rs:11
Arc::new(scheduler::factory::rayon(started)),  @f4ae3bd802:crates/ryo/cli/src/resources/query/run/composition.rs:84
scheduler: Arc<dyn SchedulerPort>,  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/use_case.rs:121
```
Pool sizing lives in `values` (decision 3): `workers_to_start(asked, machine)` and its three count types, same lines
on both sides (quoted in section 3).

**Proof.** 7 test functions (rayon `succeeds`, `refuses`; sequential `succeeds`, `refuses`; fake `succeeds`,
`refuses`, `fails`), 9 case rows in them (3 + 2 + 4), and the 2-row contract (`every_job_runs_once`,
`no_jobs_run_no_job`; it also asserts no more jobs at once than workers) run against both adapters. Plus the core
bench workload `benches/performance/scheduler/bench.rs` (section 8). Callers' tests stand the port in with
`SchedulerFake` (core lint fixtures: `given.rs`, `lookups.rs`, `runs.rs`, `worlds.rs`, `measures.rs`, `rules/test.rs`)
and `CountingSchedulerFake` (lint `measures.rs`: heap blocks allocated per file job).

**Tastemaker entries.** No `crate-scheduler` or `nu-hex-scheduler` entry on either side. The shared rule
`port-takes-no-settings` holds the port's main property, with a scenario named for it:
```
id = 'scheduler-tells-its-worker-count'  @f4ae3bd802:tastemaker/rules/port-takes-no-settings/0.1.0/rule.toml:78
```

## 3. On the perf branch (2a207464c)
**Where.** Same folder, 200 lines without tests and fixtures (interface 26, types 14, errors 15, factory 22,
adapters 4, rayon 6 + 108, sequential 4 + 1); fixtures unchanged (144).

**The contract actually called.** `run` unchanged; one default method added:
```
pub trait SchedulerPort: Send + Sync + std::fmt::Debug {  @2a207464c:crates/ryo/crates/core/src/outbound/scheduler/interface.rs:8
fn on_owner(&self, job: &mut (dyn FnMut() + Send)) -> Result<(), SchedulerError> {  @2a207464c:crates/ryo/crates/core/src/outbound/scheduler/interface.rs:11
self.run(&mut [Box::new(|_| job())], JobCount(1))  @2a207464c:crates/ryo/crates/core/src/outbound/scheduler/interface.rs:12
fn on_owner(&self, job: &mut (dyn FnMut() + Send)) -> Result<(), SchedulerError> {  @2a207464c:crates/ryo/crates/core/src/outbound/scheduler/adapters/rayon/adapter.rs:50
if pool.current_thread_index() == Some(0) {  @2a207464c:crates/ryo/crates/core/src/outbound/scheduler/adapters/rayon/adapter.rs:53
pool.broadcast(|worker| {  @2a207464c:crates/ryo/crates/core/src/outbound/scheduler/adapters/rayon/adapter.rs:59
```
Its doc: "Runs a session operation on the same worker for every invocation on this pool." The rayon adapter runs the job
on pool thread 0 (directly when already there, by `install` on a one-thread pool, by `broadcast` otherwise; the
broadcast path `expect`s a mutex). The sequential adapter is an empty file:
```
//! The scheduler port, implemented in order on the calling thread, on one worker.  @2a207464c:crates/ryo/crates/core/src/outbound/scheduler/adapters/sequential/adapter.rs:1
```
**Public items: 9** (main's minus `sequential`, plus `on_owner`), plus the same 4 behind `fixtures`.

**Who calls it.** Only the SDK (5 files: `capacity/types.rs`, `lint/composition.rs`, `lint/types.rs`, `lint/entry.rs`,
`lint/rows.rs`) and core's `lint/errors.rs` (the error type); the command line no longer names it. Nothing calls `run`
in production. `on_owner` is called three times:
```
match Arc::clone(&engine.scheduler).on_owner(&mut job) {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:62
drop(Arc::clone(&self.engine().scheduler).on_owner(&mut || close(session)));  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:86
let report = match Arc::clone(&engine.scheduler).on_owner(&mut job) {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/rows.rs:62
Arc::new(scheduler::factory::rayon(workers))  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:35
Self::scheduler(Arc::new(scheduler::factory::rayon_over(pool)), workers)  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:73
pub scheduler: Arc<dyn SchedulerPort>,  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:135
use crate::outbound::scheduler::errors::SchedulerError;  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/lint/errors.rs:8
```
Why `on_owner` exists: the lint session is kept per thread, and the batch it holds is not `Send` (12 `Rc<` lines in
`batch.rs`), so every round must land on the thread that holds it:
```
static SESSIONS: std::cell::RefCell<HashMap<usize, Live>> = Default::default();  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:4
pub struct Batch {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:102
```
**The proof of concept bypasses the port.** It spreads its jobs with rayon directly, on whatever pool it is running
in; it lands on the edge's pool only because `on_owner` put the session on that pool's thread 0:
```
use rayon::prelude::*;  @2a207464c:crates/ryo/crates/executor/src/check_session/jobs.rs:3
pub fn run<I, T, F>(jobs: I, work: F) -> (Vec<T>, f64)  @2a207464c:crates/ryo/crates/executor/src/check_session/jobs.rs:5
let results: Vec<_> = jobs.into_par_iter().map(|job| {  @2a207464c:crates/ryo/crates/executor/src/check_session/jobs.rs:12
```
Four callers, each once per set: file shards, cross-file shards, rule admission chunks, report chunks:
```
let (results, children) = jobs::run(&shards, |shard| {  @2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:193
let _budget = finish::PoolBudget::for_workers(shards.len());  @2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:194
let (parts, cpu) = session::jobs::run(0..count.min(programs.len()), run_shard);  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/closed_workers.rs:312
let (_, cpu) = ryo_executor::check_session::jobs::run(chunks, |chunk| {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:614
let (parts, child_cpu) = ryo_executor::check_session::jobs::run(chunks, render_chunk);  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:742
```
The `f64` is CPU on threads other than the session's, summed into the bench images' printed `worker_cpu`:
```
last.worker_cpu += self.child_cpu() - previous_children;  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:344
```
**Pool sizing**, in `values`, unchanged from main:
```
pub fn workers_to_start(asked: WorkersAsked, machine: MachineWorkers) -> StartedWorkers {  @2a207464c:crates/ryo/crates/values/src/domain/runs/service.rs:7
pub struct WorkersAsked(pub usize);  @2a207464c:crates/ryo/crates/values/src/domain/runs/types.rs:23
pub struct MachineWorkers(pub u32);  @2a207464c:crates/ryo/crates/values/src/domain/runs/types.rs:117
pub struct StartedWorkers(pub u32);  @2a207464c:crates/ryo/crates/values/src/domain/runs/types.rs:121
pub(crate) fn machine_workers() -> ryo_values::domain::runs::types::MachineWorkers {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/connections.rs:4
```
**Proof.** 5 test functions (rayon 2, fake 3), 7 case rows; the contract runs against the rayon adapter only. Nothing
tests `on_owner` on the tip. The core bench workload still imports `sequential`, so it does not build on the tip
(section 8).

**Two more scheduling contracts exist, on both sides unchanged** (decision 5): the benchmark crate's own port, a thread
per task, and the Rust language's crate-internal port over the caller's pool:
```
fn run_all(&self, tasks: Vec<Box<dyn FnOnce() + Send + '_>>);  @2a207464c:crates/ryo/crates/benchmark/src/outbound/scheduler/interface.rs:8
fn run(&self, jobs: &mut [Box<dyn FnMut() + Send + '_>]);  @2a207464c:crates/ryo/languages/rust/src/outbound/job_scheduling/interface.rs:8
```

**The parked branch d2502dca4** puts the four callers behind the port: `jobs::run` takes the scheduler and makes one
`run` call per set, from inside `on_owner`; the executor drops rayon; the SDK hands `engine.scheduler` down as
`Batch.scheduler`. Port unchanged.
```
pub fn run<I, T, F>(scheduler: &dyn SchedulerPort, workers: usize, jobs: Vec<I>, work: F) -> (Vec<T>, f64)  @d2502dca4:crates/ryo/crates/executor/src/check_session/jobs.rs:11
```
Bead notes (ryo-fmx3.1.13.23.1, 27 Nov 14:25 UTC): short-check PASS, receipts identical on four workspaces; quiet
pairs against landing-139 on the Tastemaker check: one worker 21.79 -> 21.71 s CPU, ten workers 32.24 -> 31.95 s, wall
12.13 -> 11.78 s, peaks within 1.3%. Within noise: going through the port costs nothing measurable.

## 4. What changed between them
| Change | Commit | Why | Lost that should not have been? |
|---|---|---|---|
| `on_owner` added (default method + rayon override) | 753bddbb90, 24 Nov, "Schedule all shard work on the supplied pool and keep session state on its owning worker" | the same commit replaced `std::thread::scope` spawns in the shard, cross-file and admission code (then under `engine/benches/vec`) with `jobs::run` over rayon's current pool; `on_owner` makes that pool the edge's and pins the thread-local session | no measurement recorded for it; no test until d2502dca4 |
| sequential adapter, its test file and `factory::sequential` deleted | 89d89c5c6c, 27 Nov (stage 2 "never-used items") | nothing on the tip called it | **yes**: a designed adapter dropped because nothing called it (owner, 27 Nov: contracts are not dead code); 2 test functions and 2 case rows went; the core bench workload broke |
| the adapter body emptied to its doc line | 551980284c, 27 Nov | follow-on clean-up | as above |
| every production `run` call gone | core's `file_analysis`, `storing`, `cross_file` deleted in stages 1 and 2 | the proof of concept replaced them and calls rayon itself | the port's one-place guarantee ("no other crate spreads jobs") is gone; the edge's pool is reached only by the implicit current-pool rule |

## 5. The drafted public interface        (explains every line of api.txt that is changed or new)
The listing is `outbound/scheduler/api.txt`: 13 public items, plus 2 behind `fixtures`. On main 9 (+4), on the tip 9
(+4). Lines tagged `[main]` are main's verbatim at a new path; this part explains the rest.

**The claim it starts from.** Appendix A 3.15 (`PLAN.md:1404-1413`), "[exists, unchanged]", with `run` and
`on_owner`, and the executor's `jobs::run` going behind it as on d2502dca4. Checked: `run` is unchanged since main;
`on_owner` is the tip's (not main's); the claim misses that the sequential adapter is gone on the tip, that the
`jobs::run` callers need an ordered collect (written three times: main's `storing::spread`, the tip's `jobs::run`,
d2502dca4's `jobs::run`), that pool sizing sits in `values` with no home, and that two other scheduling contracts
exist. Corrected below.

- `SchedulerPort::run`: main verbatim. The port stays object-safe so the SDK can keep one `Arc<dyn SchedulerPort>`
  in `WorkerPool`; callers in the tree may still bind it as a generic (`W: SchedulerPort`, as the project sheet does).
  Its doc gains one sentence, held by a new contract row: **within one `run`, every call of one worker is on one
  thread.** Both adapters already do this (a rayon worker is one `scope.spawn` that loops; sequential is the caller's
  thread). The store needs it: its pools are thread-local and a store is made on the worker's thread
  (outbound/store sheet, section 5 and decision 3).
- `SchedulerPort::on_owner`: the tip verbatim, `[perf]` (decision 1). Kept because the lint session is thread-local
  today; the contract gains a row that two `on_owner` calls land on the same thread, and d2502dca4's three rows
  (`run` from inside `on_owner` at 1, 2 and 10 workers) move from the rayon test into the contract so every adapter
  runs them. The rayon override's `expect("session owner")` becomes a `SchedulerError::Unavailable` (a library does
  not panic on a poisoned lock).
- `sequential()`: restored from main with its adapter and its 2 test functions; the core bench workload needs it and
  it is the designed one-thread adapter (owner, 27 Nov: a designed contract is never dropped because nothing calls it).
- `spread`: `[changed]`, decision 2. One `run` call for a set of owned jobs, answers in job order, at most
  `WorkerCount` workers (clamped to the number of jobs, at least 1). It is d2502dca4's `jobs::run` without the CPU
  clock and main's `storing::spread` made public. The clock is the caller's (its `work` closure measures itself,
  as `jobs::run` does today with `thread_cpu`), because timing is the profile's concern, not the scheduler's.
- `WorkersAsked`, `workers_to_start`: `[changed]`, moved from `values` (which dissolves), decision 3.
  `MachineWorkers` and `StartedWorkers` fold into `WorkerCount`: three newtypes of one `u32` worker count become one.
  The edge reads the machine (`std::thread::available_parallelism`, the SDK's `machine_workers`) and passes it in;
  the scheduler does no I/O.
- Fixtures: `SchedulerFake` and `CountingSchedulerFake` stay public behind the feature (other crates' tests use
  them). `contract::{CASES, check}` become `#[cfg(test)]` private: on both sides only the crate's own adapter tests
  call them.
- Placement inside the crate (nu-hex): `src/outbound/scheduler/{interface, types, errors, factory, adapters/{rayon,
  sequential}, fixtures}` as on main; `spread` in `src/application/spread`; `workers_to_start` and `WorkersAsked` in
  `src/domain/workers`. `lib.rs` re-exports flat (`ryo_scheduler::SchedulerPort`), the form the store and project
  sheets already name.

**Types other sheets name from here.** `ryo_scheduler::WorkerCount` (outbound/store: `Store::for_workers`),
`ryo_scheduler::SchedulerPort` (application/project: `CrossFileUseCaseImpl<L, W: SchedulerPort>`), `SchedulerError`
(project's `CrossFileError::WorkersFailed` wraps it; lint's errors). Not redefined anywhere else in the drafts:
`check-types.py` over the folder is the check.

## 6. Proof it brings
| Test | Source | Fate |
|---|---|---|
| rayon `succeeds`, `refuses` (3 rows) | main and tip `adapters/rayon/adapter_test.rs` | carried |
| sequential `succeeds`, `refuses` (2 rows) | main `adapters/sequential/adapter_test.rs` | carried (restored-main) |
| fake `succeeds`, `refuses`, `fails` (4 rows) | main and tip `fixtures/fake_test.rs` | carried |
| contract (2 rows, peak <= workers) against rayon and sequential | main `fixtures/contract.rs` | carried |
| `run_from_inside_on_owner_runs_every_job_once_over_{one,two,ten}_workers` | d2502dca4 rayon test | restated as contract rows |
| `a_set_of_jobs_is_one_port_call_and_answers_in_admission_order`, `an_empty_set_is_one_call_with_no_jobs` | d2502dca4 `jobs.rs` | restated as `spread` tests |
| a worker stays on one thread for the whole run | none | **missing**: new contract row |
| two `on_owner` calls land on one thread | none | **missing**: new contract row |
| no crate but this one names `rayon::` to spread jobs | none | **missing**: a check (section 9) |

Planted bugs the rows catch: a job run twice or skipped (every_job_runs_once); more jobs at once than workers (peak);
a worker moved between threads mid-run (new row); `on_owner` on a different thread on the second call (new row);
a deadlock when `run` is called from inside `on_owner` (d2502dca4 rows); `spread` answers out of order (restated).

## 7. Perf gains it carries
Rows of `perf-gains.tsv` whose crate column names the scheduler:

| Row | Gain | State | What in the draft keeps it |
|---|---|---|---|
| 54 | per-worker memory budget `PoolBudget::for_workers`, dagster peak 1,257.5 -> 480.1 MiB (joined with row 7) | live | `WorkerCount` is the one count type, given by the edge to both `rayon(WorkerCount)` and the store's `Store::for_workers(WorkerCount)`; the port never tells it (rule `port-takes-no-settings`). The tip passes the active shard count (`shards.len()`, `worker.rs:194`); under the draft the caller passes the number of worker closures it hands to `run` (what `spread` clamps to). The new "one thread per worker for the run" row keeps the thread-local share valid. |
| 13 | many workers sharing a frozen prepared set: ten cores fastapi 3.619 -> 1.380 CPU-s, 838 -> 387 MiB | parked:89d13a19e | `run`'s closures borrow (`+ '_`), so every worker can read one frozen `Programs`/`Screens`/`FrozenSymbols` by reference with no `Arc` or copy; jobs are `Send`, shared state must be `Sync`. The gain itself lands in programs, symbols and screen; the scheduler only must not force owned or `'static` jobs. |

Related, not in the crate column: row 37 (pools for columns and tables, -14 to -17%) relies on the same
thread-per-worker property, now a contract row. d2502dca4 (port in the path) measured within noise (section 3), so
the port has no cost to defend; the scheduler's own cost at ten workers was never isolated (review-performance,
"the scheduler's own cost: not measured").

**Would have landed inside this crate alone under the draft:** d2502dca4's whole change (the executor's jobs behind
the port) reduces to callers calling `spread`, and any later change to how a set is handed out (the two mutexes per
job, a chunked queue, slot layout) is inside `spread` or an adapter. 753bddbb90's first half (stop spawning threads
per shard; use the edge's pool) would have been unnecessary: callers already spread on the edge's pool.
**Would still have crossed:** 753bddbb90's second half (the session pinned to a thread) crosses into lint while the
session is thread-local (decision 1); row 54 touches the store and the edge (the scheduler only supplies the type);
row 13 crosses programs, symbols and screen (frozen shared state), not the scheduler. The ten-worker wall bar is
decided before the first file is read (PLAN 10.5): programs', not this crate's.

## 8. Benchmarks that cover it
| Benchmark | Today (tip) | After the draft |
|---|---|---|
| core `performance` bench, scheduler workload: builds a pool and runs 64 jobs on `rayon`, `rayon_over`, `sequential` | imports the three factories (quoted below the table) — **does not build on the tip** (`sequential` gone; also found by bead ryo-fmx3.1.13.23.1) | moves with the crate (`benches/performance/scheduler/bench.rs`, path only), imports `ryo_scheduler::{rayon, rayon_over, sequential}`; builds again because `sequential` is restored |
| Ruff, Clippy, Oxlint linter benchmarks (`languages/*/benchmarks/linters/*/composition/measure.rs`) | build the engine through `benchmark`'s `EngineLocalAdapter::pool()`, a rayon pool sized by `RAYON_NUM_THREADS`, handed to `WorkerPool::from(Arc<rayon::ThreadPool>)`, which calls `rayon_over`; their verification shards run on the benchmark crate's own port (`benchmark::outbound::scheduler::factory::threads()`) | unchanged: the SDK's composition calls `ryo_scheduler::rayon_over`; ten-worker figures still come from the pool the benchmark builds. The benchmark's own port is the benchmarks sheet's (decision 5) |
| Tastemaker check, one and ten workers (the bar row; `gates.tsv` ten-worker columns) | the session on thread 0 by `on_owner`; shards by `jobs::run` over rayon; `worker_cpu` printed from `jobs::run`'s `f64` | `on_owner` kept; shards by `spread`; `worker_cpu` kept by the caller's `work` closure timing itself (non-owner threads only), as `jobs::run` does now. d2502dca4 is the measured proof this route holds the bar cells |

The workload's import, the same line on both sides:
```
use ryo_core::outbound::scheduler::factory::{rayon, rayon_over, sequential};  @2a207464c:crates/ryo/crates/core/benches/performance/scheduler/bench.rs:8
use ryo_core::outbound::scheduler::factory::{rayon, rayon_over, sequential};  @f4ae3bd802:crates/ryo/crates/core/benches/performance/scheduler/bench.rs:8
Ok(WorkerPool::from(std::sync::Arc::new(  @2a207464c:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/adapter.rs:82
Arc::new(benchmark::outbound::scheduler::factory::threads()),  @2a207464c:crates/ryo/languages/python/benchmarks/linters/ruff/composition/measure.rs:168
```
No benchmark names `JobCount`, `JobNumber` or `SchedulerError` outside the core workload.

## 9. Checks
IMPORTS: none
CROSSED: per file
- **Directions.** Imports no Ryo crate (outbound may import domain and outbound; it needs neither). Outside crates:
  `rayon` (adapter), `thiserror` (error); `dhat` only behind `fixtures` (`CountingSchedulerFake`). Imported by:
  outbound/store (`WorkerCount`), application/project (`SchedulerPort`), inbound/lint, the SDK and the command line.
- **How often.** `run` (and `spread`) once per set of jobs: per request phase. The job closure is called once per job:
  per file on main (one job per file), per shard on the tip; never per row or cell. That one indirect call per job is
  the only `dyn` crossing (decision 4).
- **Layout words.** None in any signature.
- **Settings.** The port takes and gives no setting: `port-takes-no-settings` and its scenario
  `scheduler-tells-its-worker-count` hold it as written. The count goes to the factory.
- **One place spreads jobs.** No check holds it today (the tip's executor imports `rayon::prelude`). Cheapest
  enforcement: Cargo, by `rayon` appearing only in this crate's manifest among Ryo's engine crates (the corpus
  adapter's `par_iter` at `core/src/outbound/corpus/adapters/ignore/adapter.rs:163` and the Rust language's
  `JobSchedulingPort` adapter are the two other `rayon` users; both run on the caller's pool, inside a scheduler job).
- **Tastemaker.** New: `crate-scheduler`, `nu-hex-scheduler` entries (per Appendix A section 2, point 5).

## 10. Decisions for the owner              (each: the example from the code; why; what we get; my recommendation)
1. **Keep `on_owner`?** Example: `SESSIONS` is a `thread_local!` (`session/entry.rs:4`) holding a `Batch` with `Rc`s,
   and three SDK calls run every round through `on_owner` (`entry.rs:62`, `:86`, `rows.rs:62`). Why: without it a
   round could land on a thread that does not hold the session. Main had no such method; it ran single jobs through
   `run(.., JobCount(1))` (`project/service.rs:235`) with no thread promise. What we get by keeping: the live
   behaviour, now tested for every adapter. What we get by dropping: one method less, if lint's session becomes an
   owned `Send` value (a lint-sheet question). **My pick: keep it in the first build; drop it when the lint session no
   longer lives in a thread-local.**
2. **`spread` in the scheduler, public?** Example: the same 20-line ordered collect exists as main's
   `storing::spread` (`service.rs:81`), the tip's `jobs::run` and d2502dca4's `jobs::run`, called from four places
   (file shards, cross-file shards, admission chunks, report chunks). Why: one definition per concept, and a change
   to how a set is handed out then lands here alone (aim 3). Cost: one more public item and a bare `Vec` in a public
   signature (scoper section 2 asks for nominal types in interface modules; `spread` is a service function, not a
   port method). **My pick: yes, public, in `src/application/spread`.**
3. **Pool sizing moves here from `values`?** Example: `workers_to_start(asked: WorkersAsked, machine:
   MachineWorkers) -> StartedWorkers` (`values/src/domain/runs/service.rs:7`), called by the SDK's composition and
   the command line; `MachineWorkers`, `StartedWorkers` and the scheduler's `WorkerCount` are three `u32` newtypes for
   one count. Why: `values` dissolves and every item needs a home by concept; how many workers run jobs is this
   crate's concept. What we get: one count type; the edge's sizing in the crate whose factory takes it. **My pick:
   move; fold both into `WorkerCount`.**
4. **Jobs stay `Box<dyn FnMut(JobNumber)>`?** Example: `run`'s first parameter on both sides (`interface.rs:17`).
   The owner's rule asks hot paths to fix the implementation's type at compile time. A generic `run<W: FnMut(JobNumber)
   + Send>(&self, workers: &mut [W], ..)` makes the port not object-safe, so the SDK's public `WorkerPool(Arc<dyn
   SchedulerPort>, WorkerCount)` would need a type parameter or an enum of adapters. The `dyn` here is one indirect
   call per job (per file at most), and d2502dca4 measured the whole route within noise. **My pick: keep main's
   signature; callers bind the port as a generic where they can.**
5. **The two other scheduling contracts.** Example: the benchmark crate's `SchedulerPort::run_all` (a thread per task,
   because "one that waits for another's result must be given its own worker", `benchmark/.../interface.rs:5`) and
   the Rust language's `pub(crate) JobSchedulingPort` (runs on the caller's pool, `job_scheduling/interface.rs:8`).
   Why: three definitions of "run these jobs" look like one concept. They are not the same contract: `run_all` must
   give each task its own thread (a bounded pool would deadlock its waiting tasks), and a language may import only
   `outbound/language`, so it cannot name this crate. **My pick: leave both where they are; the benchmarks sheet and
   the languages sheet carry them; the ledger records them as not this sheet's.**

## 11. Not checked
- Nothing was compiled or run. `spread`'s signature (`&(impl SchedulerPort + ?Sized)`, `Vec<J>`) is not type-checked,
  and `tm check` was not run on a scratch skeleton for the nominal-type rule.
- The thread-per-worker property for the rayon adapter is read from the code (one `scope.spawn` per worker that loops),
  not tested; whether rayon's `scope` may run a spawned closure on the calling thread when called from outside the
  pool was not checked.
- That the `CountingSchedulerFake` has a caller after the build: on the tip its only caller (`lint/fixtures/measures.rs`)
  is gone; kept as main's designed proof, not verified as needed.
- The CPU split printed by the bench images (`worker_cpu`): that moving the clock into the callers' closures keeps the
  printed figure identical was not checked; d2502dca4 kept it in `jobs.rs`.
- Where the case harness `cases::Case` (copied per crate in core, the command line, benchmark, benchmark_contracts)
  lives for this crate's private contract rows.
- 753bddbb90's perf effect (thread spawns per shard against the pool) was never measured; I found no record.
- Whether the corpus adapter's `par_iter` runs inside a scheduler job on the tip (that is, on the edge's pool) or on
  rayon's global pool.
- The SDK's public `From<Arc<rayon::ThreadPool>> for WorkerPool` (`sdk/api.txt:2171` on the tip) puts rayon on the
  SDK's surface; left to the SDK sheet.
