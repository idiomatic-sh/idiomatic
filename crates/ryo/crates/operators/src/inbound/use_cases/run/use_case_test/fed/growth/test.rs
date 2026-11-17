//! Every operator of the plan's one list is measured through `plan` and `run` only, with no test of its own: its instances
//! (`Witnesses`) run over every family of generic rows at a size and its double. Each instance's rows may grow no faster than
//! its declaration bounds them on that family, and its CPU no faster than the rows it read and wrote (work linear in its
//! input and output, the log of a sort within the slack). Every listed kind has an instance, and every keyed or banded kind
//! has an instance of every fan, against the fixed lists.
use super::fixtures::{COLUMNS, Family, Measured, inputs, once, plan};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
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

/// The rows each instance wrote on each family at its fixed size: one file of 128 rows a side, or 256 files or copies.
/// Recorded from a run.
const ROWS: &str = "\
     Filter#0 AllEqualKeys=128 DeepChain=128 WideFanOut=128 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024 WideProgram=1024\n\
     Lookup#0 AllEqualKeys=128 DeepChain=128 WideFanOut=128 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024 WideProgram=1024\n\
     Project#0 AllEqualKeys=128 DeepChain=128 WideFanOut=128 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024 WideProgram=1024\n\
     Extend#0 AllEqualKeys=128 DeepChain=128 WideFanOut=128 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024 WideProgram=1024\n\
     Probe#0 AllEqualKeys=4096 DeepChain=64 WideFanOut=64 ManyFiles=1024 ManySmallFiles=2048 WidePlan=1024 WideProgram=1024\n\
     Probe#1 AllEqualKeys=4096 DeepChain=64 WideFanOut=64 ManyFiles=1024 ManySmallFiles=2048 WidePlan=1024 WideProgram=1024\n\
     Probe#2 AllEqualKeys=4096 DeepChain=64 WideFanOut=64 ManyFiles=1024 ManySmallFiles=2048 WidePlan=1024 WideProgram=1024\n\
     Probe#3 AllEqualKeys=32 DeepChain=32 WideFanOut=32 ManyFiles=256 ManySmallFiles=1024 WidePlan=256 WideProgram=256\n\
     Probe#4 AllEqualKeys=64 DeepChain=64 WideFanOut=64 ManyFiles=512 ManySmallFiles=2048 WidePlan=512 WideProgram=512\n\
     Probe#5 AllEqualKeys=64 DeepChain=96 WideFanOut=96 ManyFiles=512 ManySmallFiles=3072 WidePlan=512 WideProgram=512\n\
     Distinct#0 AllEqualKeys=224 DeepChain=224 WideFanOut=224 ManyFiles=1792 ManySmallFiles=7168 WidePlan=1792 WideProgram=1792\n\
     Group#0 AllEqualKeys=65 DeepChain=128 WideFanOut=128 ManyFiles=768 ManySmallFiles=4096 WidePlan=768 WideProgram=768\n\
     Reach#0 AllEqualKeys=256 DeepChain=16384 WideFanOut=256 ManyFiles=2048 ManySmallFiles=8192 WidePlan=2048 WideProgram=2048\n\
     SeededReach#0 AllEqualKeys=256 DeepChain=128 WideFanOut=256 ManyFiles=2048 ManySmallFiles=8192 WidePlan=2048 WideProgram=2048\n\
     Axis#0 AllEqualKeys=256 DeepChain=128 WideFanOut=256 ManyFiles=2048 ManySmallFiles=8192 WidePlan=2048 WideProgram=2048\n\
     OrderedWalk#0 AllEqualKeys=256 DeepChain=128 WideFanOut=256 ManyFiles=2048 ManySmallFiles=8192 WidePlan=2048 WideProgram=2048\n\
     Worklist#0 AllEqualKeys=64 DeepChain=1 WideFanOut=64 ManyFiles=512 ManySmallFiles=2048 WidePlan=512 WideProgram=512\n\
     Components#0 AllEqualKeys=2 DeepChain=128 WideFanOut=2 ManyFiles=512 ManySmallFiles=512 WidePlan=512 WideProgram=512\n\
     TextMap#0 AllEqualKeys=128 DeepChain=128 WideFanOut=128 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024 WideProgram=1024\n\
     TextMap#1 AllEqualKeys=128 DeepChain=128 WideFanOut=128 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024 WideProgram=1024\n\
     TextMap#2 AllEqualKeys=256 DeepChain=256 WideFanOut=256 ManyFiles=2048 ManySmallFiles=8192 WidePlan=2048 WideProgram=2048\n\
     TextMap#3 AllEqualKeys=128 DeepChain=128 WideFanOut=128 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024 WideProgram=1024\n\
     TextMap#4 AllEqualKeys=128 DeepChain=128 WideFanOut=128 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024 WideProgram=1024\n\
     TextMap#5 AllEqualKeys=128 DeepChain=128 WideFanOut=128 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024 WideProgram=1024\n\
     TextMap#6 AllEqualKeys=128 DeepChain=128 WideFanOut=128 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024 WideProgram=1024\n\
     TextMap#7 AllEqualKeys=128 DeepChain=128 WideFanOut=128 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024 WideProgram=1024\n\
     BandJoin#0 AllEqualKeys=64 DeepChain=32 WideFanOut=32 ManyFiles=512 ManySmallFiles=1024 WidePlan=512 WideProgram=512\n\
     BandJoin#1 AllEqualKeys=2048 DeepChain=32 WideFanOut=32 ManyFiles=512 ManySmallFiles=1024 WidePlan=512 WideProgram=512\n\
     BandJoin#2 AllEqualKeys=2048 DeepChain=32 WideFanOut=32 ManyFiles=512 ManySmallFiles=1024 WidePlan=512 WideProgram=512\n\
     BandJoin#3 AllEqualKeys=2048 DeepChain=32 WideFanOut=32 ManyFiles=512 ManySmallFiles=1024 WidePlan=512 WideProgram=512\n\
     OrderedMatch#0 AllEqualKeys=63 DeepChain=32 WideFanOut=32 ManyFiles=256 ManySmallFiles=1024 WidePlan=256 WideProgram=256\n\
     Render#0 AllEqualKeys=128 DeepChain=128 WideFanOut=128 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024 WideProgram=1024\n\
     ScanBound#0 AllEqualKeys=128 DeepChain=128 WideFanOut=128 ManyFiles=1024 ManySmallFiles=4096 WidePlan=1024 WideProgram=1024";

/// The slack a doubling of rows may take past its bound: the log of a sort.
const SLACK: f64 = 1.15;

/// The size every single-file family is measured at, and its double: small enough that a declared product stays within the
/// run budget; the worker's counted work, not a timer, shows a quadratic at any size.
const ROWS_SIZE: usize = 128;
/// How many files, or copies, a family of many files or copies reads.
const FILES: usize = 256;

/// `witness` run over `family` at size `n`: the rows it read and wrote, the peak, and the work the worker counted.
///
/// # Errors
///
/// The run's first fault.
fn measured(
    witness: &Witnessed<Lowering>,
    family: Family,
    n: usize,
) -> Result<Measured, PortableOperatorFault> {
    let copies = family.copies(n);
    let (plan, ran) = plan(witness, copies);
    let files = inputs(family, &witness.roles, n);
    once(&plan, (family, &ran), &files, &mut FileBuffers::default())
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

/// What `witness` measured on `family`, at its fixed size and the double, or the fault it raised.
fn doubled(witness: &Witnessed<Lowering>, family: Family) -> Result<(usize, Measured, Measured), String> {
    let n = family.size(ROWS_SIZE, FILES);
    let at = |n| measured(witness, family, n).map_err(|fault| format!("faulted {fault:?} at {n}"));
    Ok((n, at(n)?, at(n * 2)?))
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
    // An instance that writes no row at either size has no growth to judge: its rows prove nothing.
    if small.wrote == 0 && double.wrote == 0 {
        breaches.push(format!("{name}: wrote no row at {n} or {}", n * 2));
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
