//! Every operator of the plan's one list is measured through `plan` and `run` only, with no test of its own: its instances
//! (`Witnesses`) run over every family of generic rows at a size and its double. Each instance's rows may grow no faster than
//! its declaration bounds them on that family, and its CPU no faster than the rows it read and wrote (work linear in its
//! input and output, the log of a sort within the slack). Every listed kind has an instance, and every keyed or banded kind
//! has an instance of every fan, against the fixed lists.
use super::fixtures::{COLUMNS, Family, Measured, inputs, once, plan};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use std::time::Duration;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::growth::service::composed;
use crate::domain::growth::types::{Bound, Degree, Fan, Growth, Witnessed};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::node_counters::value::NodeKind;
use std::borrow::Cow;

/// What a case checks.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// Every kind of the counters has an instance.
    EveryKind,
    /// Every keyed or banded kind has an instance of every fan.
    EveryFan,
    /// Every instance grows within its declaration on every family.
    EveryGrowth,
}

/// What a check answered.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// How many kinds the check held for.
    Held(usize),
    /// Each instance's rows written on each family at the size it was measured at, one line per instance.
    Rows(Cow<'static, str>),
}

/// What a check found broken.
#[derive(Debug, PartialEq, Eq)]
enum Breach {
    /// No instance has this kind.
    Unwitnessed(NodeKind),
    /// This kind's instances do not answer every fan: the fans they answer.
    FansDiffer(NodeKind, Vec<Fan>),
    /// Instances that broke a bound: one line each.
    Grew(String),
}

type Input = Given;
type Value = Answer;
type Failure = Breach;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_listed_operator_has_a_witness"),
        word: Word::Succeeds,
        input: || Given::EveryKind,
        expect: Expect::Value(Answer::Held(18)),
    },
    Case {
        name: CaseName("every_keyed_or_banded_kind_answers_every_fan"),
        word: Word::Succeeds,
        input: || Given::EveryFan,
        expect: Expect::Value(Answer::Held(2)),
    },
    Case {
        name: CaseName("every_witness_grows_within_its_declaration"),
        word: Word::Succeeds,
        input: || Given::EveryGrowth,
        expect: Expect::Value(Answer::Rows(Cow::Borrowed(ROWS))),
    },
];

/// The rows each instance wrote on each family at the size it was measured at: a product at 128 rows a side, rows that grow
/// linearly at 4,096, many files at 1,024. Recorded from a run.
const ROWS: &str = "\
     Filter#0 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     Lookup#0 AllEqualKeys=0 DeepChain=0 WideFanOut=0 ManyFiles=0 ManySmallFiles=0 WidePlan=0\n\
     Project#0 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     Extend#0 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     Probe#0 AllEqualKeys=16384 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     Probe#1 AllEqualKeys=16384 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     Probe#2 AllEqualKeys=16384 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     Probe#3 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     Probe#4 AllEqualKeys=0 DeepChain=0 WideFanOut=0 ManyFiles=0 ManySmallFiles=0 WidePlan=0\n\
     Probe#5 AllEqualKeys=1 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     Distinct#0 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     Group#0 AllEqualKeys=1 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     Reach#0 AllEqualKeys=4096 DeepChain=8256 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     SeededReach#0 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     Axis#0 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     OrderedWalk#0 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     Worklist#0 AllEqualKeys=1024 DeepChain=1 WideFanOut=1024 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024\n\
     Components#0 AllEqualKeys=0 DeepChain=0 WideFanOut=0 ManyFiles=0 ManySmallFiles=0 WidePlan=0\n\
     TextMap#0 AllEqualKeys=0 DeepChain=0 WideFanOut=0 ManyFiles=0 ManySmallFiles=0 WidePlan=0\n\
     TextMap#1 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     TextMap#2 AllEqualKeys=8192 DeepChain=8192 WideFanOut=8192 ManyFiles=2048 ManySmallFiles=32768 WidePlan=2048\n\
     TextMap#3 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     TextMap#4 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     TextMap#5 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     TextMap#6 AllEqualKeys=0 DeepChain=0 WideFanOut=0 ManyFiles=0 ManySmallFiles=0 WidePlan=0\n\
     TextMap#7 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024\n\
     BandJoin#0 AllEqualKeys=4095 DeepChain=0 WideFanOut=0 ManyFiles=0 ManySmallFiles=0 WidePlan=0\n\
     BandJoin#1 AllEqualKeys=8128 DeepChain=0 WideFanOut=0 ManyFiles=0 ManySmallFiles=0 WidePlan=0\n\
     BandJoin#2 AllEqualKeys=8128 DeepChain=0 WideFanOut=0 ManyFiles=0 ManySmallFiles=0 WidePlan=0\n\
     BandJoin#3 AllEqualKeys=8128 DeepChain=0 WideFanOut=0 ManyFiles=0 ManySmallFiles=0 WidePlan=0\n\
     OrderedMatch#0 AllEqualKeys=4095 DeepChain=0 WideFanOut=0 ManyFiles=0 ManySmallFiles=0 WidePlan=0\n\
     Render#0 AllEqualKeys=0 DeepChain=0 WideFanOut=0 ManyFiles=0 ManySmallFiles=0 WidePlan=0\n\
     ScanBound#0 AllEqualKeys=4096 DeepChain=4096 WideFanOut=4096 ManyFiles=1024 ManySmallFiles=16384 WidePlan=1024";

/// The slack a doubling of rows may take past its bound: the log of a sort.
const SLACK: f64 = 1.15;
/// The slack a doubling of CPU time may take past the doubling of the rows read and written: the log of a sort, the timer, and
/// a double size that no longer fits the cache. Every quadratic left the work it was measured at four times over.
const WORK_SLACK: f64 = 1.5;
/// The CPU time past which a run is not measured at a larger size: what it measured at the trial size is judged.
const LONGEST: Duration = Duration::from_millis(100);

/// The size a single file's family is measured at when its rows grow linearly.
const LINEAR: usize = 4_096;
/// The size a single file's product is measured at, so its double stays within the run budget.
const PRODUCT: usize = 128;
/// How many files a family of many files reads.
const FILES: usize = 1_024;

/// The CPU time a sample takes at least, so a fast run's ratio is not timer noise: a faster run is repeated within a sample.
const SAMPLE: Duration = Duration::from_millis(4);

/// How many samples the least is taken of.
const SAMPLES: usize = 5;

/// `witness` run over `family` at size `n`: the rows it read and wrote, the peak, and the least CPU time of the samples.
///
/// # Errors
///
/// The run's first fault.
fn measured(
    witness: &Witnessed<Lowering>,
    family: Family,
    n: usize,
) -> Result<Measured, PortableOperatorFault> {
    let copies = match family {
        Family::WidePlan => n,
        _ => 1,
    };
    let (plan, ran) = plan(witness, copies);
    let files = inputs(family, &witness.roles, n);
    // The first run sizes the samples: one fast run is repeated until a sample takes `SAMPLE`.
    let started = cpu_time::ThreadTime::now();
    let first = once(&plan, &ran, &files, &mut FileBuffers::default())?;
    let took = started.elapsed();
    let repeats = usize::try_from(SAMPLE.as_nanos() / took.as_nanos().max(1))
        .unwrap_or(usize::MAX)
        .clamp(1, 1_000);
    let mut least = Duration::MAX;
    for _ in 0..SAMPLES {
        let started = cpu_time::ThreadTime::now();
        for _ in 0..repeats {
            let _again = once(&plan, &ran, &files, &mut FileBuffers::default())?;
        }
        least = least.min(started.elapsed());
    }
    Ok(Measured {
        cpu: least / u32::try_from(repeats).unwrap_or(u32::MAX),
        ..first
    })
}

/// The kinds every listed operator's instances have.
fn witnesses() -> Vec<(NodeKind, Witnessed<Lowering>)> {
    Lowering::witnesses(COLUMNS)
}

/// The fan a keyed or banded growth answers.
const fn fan_in(growth: Growth) -> Option<Fan> {
    match growth {
        Growth::Keyed(fan) | Growth::Banded(fan) => Some(fan),
        Growth::AtMostInput
        | Growth::PerRowText
        | Growth::OnePerLeftRow
        | Growth::PerStartWalk
        | Growth::GroupOutput => None,
    }
}

/// `later` over `earlier`, an earlier of none counted as one.
#[expect(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    reason = "a ratio of row counts and nanoseconds far below 2^53"
)]
fn ratio(earlier: u128, later: u128) -> f64 {
    later as f64 / earlier.max(1) as f64
}

/// What `witness` measured on `family`, at its size and the double, or the fault it raised.
fn doubled(witness: &Witnessed<Lowering>, family: Family) -> Result<(usize, Measured, Measured), String> {
    let at = |n| measured(witness, family, n).map_err(|fault| format!("faulted {fault:?} at {n}"));
    match family {
        Family::ManyFiles | Family::ManySmallFiles | Family::WidePlan => Ok((FILES, at(FILES)?, at(FILES * 2)?)),
        Family::AllEqualKeys | Family::DeepChain | Family::WideFanOut => {
            // A product's rows are measured small; rows that grow linearly at the size the work shows at.
            let (small, double) = (at(PRODUCT)?, at(PRODUCT * 2)?);
            let scaled = double.cpu.saturating_mul(u32::try_from(LINEAR / PRODUCT).unwrap_or(u32::MAX));
            if ratio(small.wrote as u128, double.wrote as u128) > 2.0 * SLACK || scaled > LONGEST {
                return Ok((PRODUCT, small, double));
            }
            Ok((LINEAR, at(LINEAR)?, at(LINEAR * 2)?))
        }
    }
}

/// The breaches of one instance on one family, as lines, and its rows written.
fn judged(kind: NodeKind, place: usize, witness: &Witnessed<Lowering>, family: Family) -> (usize, Vec<String>) {
    let name = format!("{kind:?}#{place} {family:?}");
    let (n, small, double) = match doubled(witness, family) {
        Ok(found) => found,
        Err(fault) => return (0, vec![format!("{name}: {fault}")]),
    };
    let rows = Bound {
        rows: Degree(1),
        ..Bound::default()
    };
    let bound = composed(witness.node.growth(), &vec![rows; witness.roles.len()]);
    let declared = family.degree(bound.output).0;
    let allowed = f64::from(1_u32 << declared) * SLACK;
    let mut breaches = Vec::new();
    let grew = ratio(small.wrote as u128, double.wrote as u128);
    if (small.wrote == 0 && double.wrote > 0) || grew > allowed {
        breaches.push(format!(
            "{name}: rows {} at {n} -> {} at {}, x{grew:.2} over x{allowed:.2} (rows^{declared})",
            small.wrote,
            double.wrote,
            n * 2
        ));
    }
    let held = ratio(small.peak as u128, double.peak as u128);
    if held > allowed.max(2.0 * SLACK) {
        breaches.push(format!(
            "{name}: peak {} at {n} -> {} at {}, x{held:.2} over x{allowed:.2}",
            small.peak,
            double.peak,
            n * 2
        ));
    }
    let io = ratio(
        (small.read + small.wrote) as u128,
        (double.read + double.wrote) as u128,
    );
    // The worker's own counts of its work (the slots its walks' resets emptied, the nodes its file ends released) are exact:
    // held to the rows read and written with no timer's slack.
    let counted = ratio(small.work as u128, double.work as u128);
    if counted > io.max(1.0) * SLACK {
        breaches.push(format!(
            "{name}: work counted {} at {n} -> {} at {}, x{counted:.2} over rows in and out x{io:.2}",
            small.work,
            double.work,
            n * 2
        ));
    }
    let mut work = ratio(small.cpu.as_nanos(), double.cpu.as_nanos());
    // A doubling of time past the rows is measured once more before it is a breach: the timer, not the work, can be slow once.
    if work > io.max(1.0) * WORK_SLACK
        && let Ok((_, again, again_double)) = doubled(witness, family)
    {
        work = work.min(ratio(again.cpu.as_nanos(), again_double.cpu.as_nanos()));
    }
    if work > io.max(1.0) * WORK_SLACK {
        breaches.push(format!(
            "{name}: cpu {:?} at {n} -> {:?} at {}, x{work:.2} over rows in and out x{io:.2}",
            small.cpu,
            double.cpu,
            n * 2
        ));
    }
    (small.wrote, breaches)
}

fn subject(given: Input) -> Result<Value, Failure> {
    let all = witnesses();
    match given {
        Given::EveryKind => {
            for kind in NodeKind::ALL {
                if !all.iter().any(|(witnessed, _)| witnessed == kind) {
                    return Err(Breach::Unwitnessed(*kind));
                }
            }
            Ok(Answer::Held(NodeKind::ALL.len()))
        }
        Given::EveryFan => {
            let mut held = 0;
            for kind in NodeKind::ALL {
                let fans: Vec<Fan> = all
                    .iter()
                    .filter(|(witnessed, _)| witnessed == kind)
                    .filter_map(|(_, witness)| fan_in(witness.node.growth()))
                    .collect();
                if fans.is_empty() {
                    continue;
                }
                if !Fan::ALL.iter().all(|fan| fans.contains(fan)) {
                    return Err(Breach::FansDiffer(*kind, fans));
                }
                held += 1;
            }
            Ok(Answer::Held(held))
        }
        Given::EveryGrowth => {
            let mut lines = Vec::new();
            let mut breaches = Vec::new();
            let mut place = 0;
            let mut last = None;
            for (kind, witness) in &all {
                place = if last == Some(*kind) { place + 1 } else { 0 };
                last = Some(*kind);
                let mut line = format!("{kind:?}#{place}");
                for family in Family::ALL {
                    let (wrote, broke) = judged(*kind, place, witness, family);
                    line.push_str(&format!(" {family:?}={wrote}"));
                    breaches.extend(broke);
                }
                lines.push(line);
            }
            if lines.is_empty() {
                return Err(Breach::Grew("no instance".to_owned()));
            }
            if !breaches.is_empty() {
                return Err(Breach::Grew(breaches.join("\n")));
            }
            Ok(Answer::Rows(Cow::Owned(lines.join("\n"))))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
