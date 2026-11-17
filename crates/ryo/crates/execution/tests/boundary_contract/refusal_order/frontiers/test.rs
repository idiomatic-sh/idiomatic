//! Refusals around a frontier, each expected line the interpreter's answer at checkpoint-350 (64a99968a1) to the same input:
//! steps in the file handing rows to steps after the files (both passes over a facts fake that keeps what the file captured),
//! and steps after the files reading a frontier entry no file handed over.
use super::super::fixtures::programs::{bad, h, p, s, u, w99};
use super::super::fixtures::{after_the_files, both_passes};
use crate::*;

const PF: StepScope = StepScope::PerFile;
const XF: StepScope = StepScope::CrossFile;

/// What the interpreter answered for each program whose steps in the file hand their rows to steps after the files.
const ACROSS_THE_FILES: &str = concat!(
    "MX front-w99-p0 pop=false obs=false: file=deferred captured=[\"kept\"] phase=failed Table(NoSuchColumn { index: 0 })\n",
    "MX front-w99-p0 pop=false obs=true: file=deferred captured=[\"kept\"] phase=failed Table(NoSuchColumn { index: 0 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\", \"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "MX front-w99-p0 pop=true obs=false: file=deferred captured=[\"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" }))\n",
    "MX front-w99-p0 pop=true obs=true: file=deferred captured=[\"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" })) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "MX front-w99-p1 pop=false obs=false: file=deferred captured=[\"kept\"] phase=failed Table(NoSuchColumn { index: 1 })\n",
    "MX front-w99-p1 pop=false obs=true: file=deferred captured=[\"kept\"] phase=failed Table(NoSuchColumn { index: 1 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\", \"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "MX front-w99-p1 pop=true obs=false: file=deferred captured=[\"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" }))\n",
    "MX front-w99-p1 pop=true obs=true: file=deferred captured=[\"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" })) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "MX front-nested pop=false obs=false: file=deferred captured=[\"refused Table(NoSuchColumn { index: 0 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" }))\n",
    "MX front-nested pop=false obs=true: file=deferred captured=[\"refused Table(NoSuchColumn { index: 0 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" })) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "MX front-nested pop=true obs=false: file=deferred captured=[\"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" }))\n",
    "MX front-nested pop=true obs=true: file=deferred captured=[\"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" })) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "MX front-h-xw99-p0 pop=false obs=false: file=deferred captured=[\"kept\"] phase=failed Table(NoSuchColumn { index: 0 })\n",
    "MX front-h-xw99-p0 pop=false obs=true: file=deferred captured=[\"kept\"] phase=failed Table(NoSuchColumn { index: 0 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\", \"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "MX front-h-xw99-p0 pop=true obs=false: file=deferred captured=[\"kept\"] phase=failed Table(NoSuchColumn { index: 99 })\n",
    "MX front-h-xw99-p0 pop=true obs=true: file=deferred captured=[\"kept\"] phase=failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\", \"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "MX front-p99 pop=false obs=false: file=deferred captured=[\"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" }))\n",
    "MX front-p99 pop=false obs=true: file=deferred captured=[\"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" })) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "MX front-p99 pop=true obs=false: file=deferred captured=[\"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" }))\n",
    "MX front-p99 pop=true obs=true: file=deferred captured=[\"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" })) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "MX front-bad-p99 pop=false obs=false: file=deferred captured=[\"kept\", \"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" }))\n",
    "MX front-bad-p99 pop=false obs=true: file=deferred captured=[\"kept\", \"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" })) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "MX front-bad-p99 pop=true obs=false: file=deferred captured=[\"refused InvalidRegex { reason: \\\"regex parse error:\\\\n    (\\\\n    ^\\\\nerror: unclosed group\\\" }\", \"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" }))\n",
    "MX front-bad-p99 pop=true obs=true: file=deferred captured=[\"refused InvalidRegex { reason: \\\"regex parse error:\\\\n    (\\\\n    ^\\\\nerror: unclosed group\\\" }\", \"refused Table(NoSuchColumn { index: 99 })\"] phase=stopped Frontiers(Metadata(ObservationIncomplete { reason: \"frontier source proof does not match the prepared program\" })) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
);

/// What the interpreter answered for each program reading a frontier entry no file handed over: its only file skipped.
const SKIPPED_FRONTIERS: &str = concat!(
    "SK skip-w99-p0-99 pop=false obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-w99-p0-99 pop=false obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-w99-p0-99 pop=true obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-w99-p0-99 pop=true obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-w99-p0-99-p88 pop=false obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-w99-p0-99-p88 pop=false obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-w99-p0-99-p88 pop=true obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-w99-p0-99-p88 pop=true obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-w99-p0 pop=false obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-w99-p0 pop=false obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-w99-p0 pop=true obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-w99-p0 pop=true obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-h-p99 pop=false obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-h-p99 pop=false obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-h-p99 pop=true obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-h-p99 pop=true obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-h-w99 pop=false obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-h-w99 pop=false obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-h-w99 pop=true obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-h-w99 pop=true obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-h-w99-p0 pop=false obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-h-w99-p0 pop=false obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-h-w99-p0 pop=true obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-h-w99-p0 pop=true obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-h-p99-union pop=false obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-h-p99-union pop=false obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-h-p99-union pop=true obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-h-p99-union pop=true obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-h-p0 pop=false obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-h-p0 pop=false obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "SK skip-h-p0 pop=true obs=false: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }\n",
    "SK skip-h-p0 pop=true obs=true: unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
);

#[test]
fn refusals_handed_across_the_files_answer_as_the_interpreter() {
    let programs: Vec<(&str, Vec<(StepScope, Operation)>, u32, Vec<Input>)> = vec![
        (
            "front-w99-p0",
            vec![(PF, h()), (PF, w99(0)), (XF, p(1, &[0]))],
            2,
            vec![s(1)],
        ),
        (
            "front-w99-p1",
            vec![(PF, h()), (PF, w99(0)), (XF, p(1, &[1]))],
            2,
            vec![s(1)],
        ),
        (
            "front-nested",
            vec![(PF, h()), (PF, w99(0)), (PF, p(1, &[0])), (XF, p(2, &[0]))],
            3,
            vec![s(2)],
        ),
        (
            "front-h-xw99-p0",
            vec![(PF, h()), (XF, w99(0)), (XF, p(1, &[0]))],
            2,
            vec![s(0)],
        ),
        (
            "front-p99",
            vec![(PF, h()), (PF, p(0, &[99])), (XF, p(1, &[0]))],
            2,
            vec![s(1)],
        ),
        (
            "front-bad-p99",
            vec![(PF, h()), (PF, bad(0)), (PF, p(0, &[99])), (XF, u(1, 2))],
            3,
            vec![s(1), s(2)],
        ),
    ];
    let mut answered = String::new();
    for (name, steps, emit, frontier) in programs {
        for populated in [false, true] {
            for observe in [false, true] {
                let answer = both_passes(&steps, emit, frontier.clone(), populated, observe);
                answered.push_str(&format!(
                    "MX {name} pop={populated} obs={observe}: {answer}\n"
                ));
            }
        }
    }
    assert_eq!(answered, ACROSS_THE_FILES);
}

#[test]
fn refusals_over_a_frontier_no_file_handed_over_answer_as_the_interpreter() {
    let programs: Vec<(&str, Vec<(StepScope, Operation)>, u32, Vec<Input>)> = vec![
        (
            "skip-w99-p0-99",
            vec![(PF, h()), (PF, w99(0)), (XF, p(1, &[0, 99]))],
            2,
            vec![s(1)],
        ),
        (
            "skip-w99-p0-99-p88",
            vec![
                (PF, h()),
                (PF, w99(0)),
                (XF, p(1, &[0, 99])),
                (XF, p(2, &[88])),
            ],
            3,
            vec![s(1)],
        ),
        (
            "skip-w99-p0",
            vec![(PF, h()), (PF, w99(0)), (XF, p(1, &[0]))],
            2,
            vec![s(1)],
        ),
        (
            "skip-h-p99",
            vec![(PF, h()), (XF, p(0, &[99]))],
            1,
            vec![s(0)],
        ),
        ("skip-h-w99", vec![(PF, h()), (XF, w99(0))], 1, vec![s(0)]),
        (
            "skip-h-w99-p0",
            vec![(PF, h()), (XF, w99(0)), (XF, p(1, &[0]))],
            2,
            vec![s(0)],
        ),
        (
            "skip-h-p99-union",
            vec![(PF, h()), (XF, p(0, &[99])), (XF, u(0, 1))],
            2,
            vec![s(0)],
        ),
        (
            "skip-h-p0",
            vec![(PF, h()), (XF, p(0, &[0]))],
            1,
            vec![s(0)],
        ),
    ];
    let mut answered = String::new();
    for (name, steps, emit, frontier) in programs {
        for populated in [false, true] {
            for observe in [false, true] {
                let answer = after_the_files(&steps, emit, frontier.clone(), populated, observe);
                answered.push_str(&format!(
                    "SK {name} pop={populated} obs={observe}: {answer}\n"
                ));
            }
        }
    }
    assert_eq!(answered, SKIPPED_FRONTIERS);
}
