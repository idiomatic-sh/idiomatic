//! The engine crate's reference bench (scope-engine3 2.12): the cost per row of each stage loop, of the index build and of
//! the table fills, on synthetic columns of 10^4, 10^5 and 10^6 rows; and of whole fills made through the fill side's
//! seam on packs the contract's encoding writes (`engine_fill_ns_per_row`). Under `cargo bench` it prints one Bencher Metric
//! Format document, a measure `ns_per_row` for each loop and size; run as a test it runs each loop once on a small input
//! and checks that it gave rows.

use ryo_engine::domain::made_tables::fixtures::benches::kinds::{
    FILL_BENCHES, fill_name, fill_units,
};
use ryo_engine::domain::made_tables::fixtures::benches::{FillWorkload, fill_workload, run_fill};
use ryo_engine::domain::stages::fixtures::run_kernel;
use ryo_engine::domain::stages::fixtures::workloads::{
    KERNELS, KernelWorkload, ReferenceKernel, kernel_name, workload,
};
use std::io::Write as _;
use std::time::{Duration, Instant};

/// The sizes each loop is timed at.
const SIZES: [usize; 3] = [10_000, 100_000, 1_000_000];

/// The sizes a whole fill is timed at: 100 rows too, where what a fill costs before its first row shows.
const FILL_SIZES: [usize; 4] = [100, 10_000, 100_000, 1_000_000];

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
/// The times of the runs of one fill: at least `LEAST_RUNS` runs and `TIMED` of them, after one that is not timed.
fn timed_fill(work: &mut FillWorkload) -> RunTimes {
    let _warm: usize = std::hint::black_box(run_fill(work));
    let mut times: Vec<f64> = Vec::new();
    let started = Instant::now();
    while times.len() < LEAST_RUNS || started.elapsed() < TIMED {
        let run = Instant::now();
        let _made: usize = std::hint::black_box(run_fill(std::hint::black_box(work)));
        times.push(run.elapsed().as_secs_f64() * 1e9);
    }
    times.sort_by(f64::total_cmp);
    RunTimes {
        lowest: times.first().copied().unwrap_or(0.0),
        median: times.get(times.len() / 2).copied().unwrap_or(0.0),
        highest: times.last().copied().unwrap_or(0.0),
    }
}

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
        | ReferenceKernel::IndexCodes
        | ReferenceKernel::IndexFar
        | ReferenceKernel::ProbeEmpty
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
        for bench in FILL_BENCHES {
            let mut work =
                fill_workload(*bench, SMOKE_ROWS).ok_or("the encoding refused a fill's pack")?;
            let given = run_fill(&mut work);
            assert!(
                given > 0,
                "{} made no row over {SMOKE_ROWS} rows",
                fill_name(*bench)
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
    for bench in FILL_BENCHES {
        for rows in FILL_SIZES {
            let mut work =
                fill_workload(*bench, rows).ok_or("the encoding refused a fill's pack")?;
            let times = timed_fill(&mut work);
            let per = f64::from(u32::try_from(fill_units(*bench, rows)).unwrap_or(u32::MAX));
            measures.push(format!(
                "\"engine_fill_ns_per_row/{}/{rows}\":{{\"ns_per_row\":{{\"value\":{:.3},\"lower_value\":{:.3},\"upper_value\":{:.3}}}}}",
                fill_name(*bench),
                times.median / per,
                times.lowest / per,
                times.highest / per,
            ));
        }
    }
    writeln!(std::io::stdout(), "{{{}}}", measures.join(","))?;
    Ok(())
}
