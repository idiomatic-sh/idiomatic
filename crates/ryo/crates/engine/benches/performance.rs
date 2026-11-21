//! The engine crate's reference bench (scope-engine3 2.12): the cost per row of each stage loop, of the index build and of
//! the table fills, on synthetic columns of 10^4, 10^5 and 10^6 rows. Under `cargo bench` it prints one Bencher Metric
//! Format document, a measure `ns_per_row` for each loop and size; run as a test it runs each loop once on a small input
//! and checks that it gave rows.

use ryo_engine::domain::stages::fixtures::run_kernel;
use ryo_engine::domain::stages::fixtures::workloads::{
    KERNELS, KernelWorkload, ReferenceKernel, kernel_name, workload,
};
use std::io::Write as _;
use std::time::{Duration, Instant};

/// The sizes each loop is timed at.
const SIZES: [usize; 3] = [10_000, 100_000, 1_000_000];

/// The size each loop is run at once, as a test.
const SMOKE_ROWS: usize = 10_000;

/// How long one loop is timed for at one size, at least.
const TIMED: Duration = Duration::from_millis(300);

/// The fewest runs a time is taken from.
const LEAST_RUNS: usize = 7;

/// The lowest, the middle and the highest time of one run, in nanoseconds.
struct RunTimes {
    /// The fastest run.
    lowest: f64,
    /// The middle run.
    median: f64,
    /// The slowest run.
    highest: f64,
}

/// Times `kernel` over its workload: one run to warm, then runs for `TIMED`, at least `LEAST_RUNS`.
fn timed(kernel: ReferenceKernel, work: &mut KernelWorkload) -> RunTimes {
    let _warm: usize = std::hint::black_box(run_kernel(kernel, work));
    let mut times: Vec<f64> = Vec::new();
    let started = Instant::now();
    while times.len() < LEAST_RUNS || started.elapsed() < TIMED {
        let run = Instant::now();
        let _given: usize = std::hint::black_box(run_kernel(kernel, std::hint::black_box(work)));
        times.push(run.elapsed().as_secs_f64() * 1e9);
    }
    times.sort_by(f64::total_cmp);
    RunTimes {
        lowest: times.first().copied().unwrap_or(0.0),
        median: times.get(times.len() / 2).copied().unwrap_or(0.0),
        highest: times.last().copied().unwrap_or(0.0),
    }
}

/// The rows a loop's cost is divided by: the rows it read, or for a product the pairs it gave.
fn divisor(kernel: ReferenceKernel, rows: usize) -> f64 {
    let counted = match kernel {
        ReferenceKernel::Product => rows.isqrt().saturating_mul(rows.isqrt()),
        ReferenceKernel::Scan
        | ReferenceKernel::Read
        | ReferenceKernel::KeepConstant
        | ReferenceKernel::KeepColumns
        | ReferenceKernel::ProbeRows
        | ReferenceKernel::ProbeIds
        | ReferenceKernel::ProbePresent
        | ReferenceKernel::ProbeAbsent
        | ReferenceKernel::Latest
        | ReferenceKernel::IndexOne
        | ReferenceKernel::IndexIds
        | ReferenceKernel::IndexTwo
        | ReferenceKernel::IndexOrdered
        | ReferenceKernel::Write
        | ReferenceKernel::WriteNew
        | ReferenceKernel::FillRows
        | ReferenceKernel::FillGroupCount
        | ReferenceKernel::FillSorted => rows,
    };
    f64::from(u32::try_from(counted.max(1)).unwrap_or(u32::MAX))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !std::env::args().any(|argument| argument == "--bench") {
        for kernel in KERNELS {
            let given = run_kernel(*kernel, &mut workload(*kernel, SMOKE_ROWS));
            assert!(
                given > 0,
                "{} gave no row over {SMOKE_ROWS} rows",
                kernel_name(*kernel)
            );
        }
        return Ok(());
    }
    let mut measures: Vec<String> = Vec::new();
    for kernel in KERNELS {
        for rows in SIZES {
            let mut work = workload(*kernel, rows);
            let times = timed(*kernel, &mut work);
            let per = divisor(*kernel, rows);
            measures.push(format!(
                "\"engine_kernel_ns_per_row/{}/{rows}\":{{\"ns_per_row\":{{\"value\":{:.3},\"lower_value\":{:.3},\"upper_value\":{:.3}}}}}",
                kernel_name(*kernel),
                times.median / per,
                times.lowest / per,
                times.highest / per,
            ));
        }
    }
    writeln!(std::io::stdout(), "{{{}}}", measures.join(","))?;
    Ok(())
}
