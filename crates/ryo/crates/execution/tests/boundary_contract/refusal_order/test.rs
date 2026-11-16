//! Refusals and row errors around refused steps answer in the interpreter's order, in each file and after the files, observed
//! or not; each expected line the interpreter's answer at checkpoint-350 (64a99968a1) to the same input, recorded there.
use super::fixtures::programs::{bad, d, h, join_functions, p, u, w99};
use super::fixtures::{after_the_files, per_file};
use crate::*;

const PF: StepScope = StepScope::PerFile;
const XF: StepScope = StepScope::CrossFile;

/// The programs every pass runs, by name: each over the hosted text, emitting the step named last.
fn shapes() -> Vec<(&'static str, Vec<Operation>, u32)> {
    vec![
        ("nested", vec![h(), w99(0), p(1, &[0]), p(2, &[0])], 3),
        ("nested-p5", vec![h(), w99(0), p(1, &[0]), p(2, &[5])], 3),
        (
            "nested-p0-99",
            vec![h(), w99(0), p(1, &[0]), p(2, &[0, 99])],
            3,
        ),
        (
            "nested3",
            vec![h(), w99(0), p(1, &[0]), p(2, &[0]), p(3, &[0])],
            4,
        ),
        (
            "two-readers-union",
            vec![h(), w99(0), p(1, &[0]), p(1, &[1]), u(2, 3)],
            4,
        ),
        (
            "two-readers-union-rev",
            vec![h(), w99(0), p(1, &[1]), p(1, &[0]), u(3, 2)],
            4,
        ),
        (
            "two-readers-one-unread",
            vec![h(), w99(0), p(1, &[0]), p(1, &[1])],
            3,
        ),
        (
            "two-readers-diff",
            vec![h(), w99(0), p(1, &[0]), p(1, &[1]), d(2, 3)],
            4,
        ),
        ("unread-bad-then-refused", vec![h(), bad(0), p(0, &[99])], 2),
        ("read-bad-then-refused", vec![h(), bad(0), p(1, &[99])], 2),
        ("union-w99-bad", vec![h(), w99(0), bad(0), u(1, 2)], 3),
        ("union-bad-w99", vec![h(), w99(0), bad(0), u(2, 1)], 3),
        ("union-w99-h", vec![h(), w99(0), u(1, 0)], 2),
        ("union-h-w99", vec![h(), w99(0), u(0, 1)], 2),
        ("union-bad-p99", vec![h(), bad(0), p(0, &[99]), u(1, 2)], 3),
        ("union-p99-bad", vec![h(), bad(0), p(0, &[99]), u(2, 1)], 3),
        ("union-p1x-w99", vec![h(), w99(0), p(0, &[99]), u(2, 1)], 3),
        ("union-w99-p1x", vec![h(), w99(0), p(0, &[99]), u(1, 2)], 3),
        ("w99-of-bad", vec![h(), bad(0), w99(1), p(2, &[0])], 3),
        ("bad-of-w99", vec![h(), w99(0), bad(1), p(2, &[0])], 3),
        ("p-of-w99-of-w99", vec![h(), w99(0), w99(1)], 2),
    ]
}

/// What the interpreter answered for each program, in each file (PF) and after the files (XF).
const IN_EACH_PASS: &str = concat!(
    "PF nested pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "PF nested pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[]\n",
    "XF nested pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "XF nested pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF nested pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF nested pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF nested pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF nested pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF nested-p5 pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "PF nested-p5 pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[]\n",
    "XF nested-p5 pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "XF nested-p5 pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF nested-p5 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF nested-p5 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF nested-p5 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF nested-p5 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF nested-p0-99 pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "PF nested-p0-99 pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[]\n",
    "XF nested-p0-99 pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "XF nested-p0-99 pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF nested-p0-99 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF nested-p0-99 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF nested-p0-99 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF nested-p0-99 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF nested3 pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "PF nested3 pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[]\n",
    "XF nested3 pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "XF nested3 pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF nested3 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF nested3 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF nested3 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF nested3 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF two-readers-union pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "PF two-readers-union pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[]\n",
    "XF two-readers-union pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "XF two-readers-union pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF two-readers-union pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF two-readers-union pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF two-readers-union pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF two-readers-union pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF two-readers-union-rev pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "PF two-readers-union-rev pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[]\n",
    "XF two-readers-union-rev pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "XF two-readers-union-rev pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF two-readers-union-rev pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF two-readers-union-rev pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF two-readers-union-rev pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF two-readers-union-rev pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF two-readers-one-unread pop=false obs=false: filestop\n",
    "PF two-readers-one-unread pop=false obs=true: filestop fed=[]\n",
    "XF two-readers-one-unread pop=false obs=false: stopped Operating(Table(NoSuchColumn { index: 0 }))\n",
    "XF two-readers-one-unread pop=false obs=true: stopped Operating(Table(NoSuchColumn { index: 0 })) fed=[]\n",
    "PF two-readers-one-unread pop=true obs=false: filestop\n",
    "PF two-readers-one-unread pop=true obs=true: filestop fed=[]\n",
    "XF two-readers-one-unread pop=true obs=false: stopped Operating(Table(NoSuchColumn { index: 0 }))\n",
    "XF two-readers-one-unread pop=true obs=true: stopped Operating(Table(NoSuchColumn { index: 0 })) fed=[]\n",
    "PF two-readers-diff pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "PF two-readers-diff pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[]\n",
    "XF two-readers-diff pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "XF two-readers-diff pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF two-readers-diff pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF two-readers-diff pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF two-readers-diff pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF two-readers-diff pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF unread-bad-then-refused pop=false obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF unread-bad-then-refused pop=false obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF unread-bad-then-refused pop=false obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF unread-bad-then-refused pop=false obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF unread-bad-then-refused pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF unread-bad-then-refused pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF unread-bad-then-refused pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF unread-bad-then-refused pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF read-bad-then-refused pop=false obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF read-bad-then-refused pop=false obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF read-bad-then-refused pop=false obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF read-bad-then-refused pop=false obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF read-bad-then-refused pop=true obs=false: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }\n",
    "PF read-bad-then-refused pop=true obs=true: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" } fed=[]\n",
    "XF read-bad-then-refused pop=true obs=false: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }\n",
    "XF read-bad-then-refused pop=true obs=true: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-w99-bad pop=false obs=false: failed ShapeMismatch\n",
    "PF union-w99-bad pop=false obs=true: failed ShapeMismatch fed=[]\n",
    "XF union-w99-bad pop=false obs=false: failed ShapeMismatch\n",
    "XF union-w99-bad pop=false obs=true: failed ShapeMismatch fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-w99-bad pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF union-w99-bad pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF union-w99-bad pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF union-w99-bad pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-bad-w99 pop=false obs=false: failed ShapeMismatch\n",
    "PF union-bad-w99 pop=false obs=true: failed ShapeMismatch fed=[]\n",
    "XF union-bad-w99 pop=false obs=false: failed ShapeMismatch\n",
    "XF union-bad-w99 pop=false obs=true: failed ShapeMismatch fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-bad-w99 pop=true obs=false: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }\n",
    "PF union-bad-w99 pop=true obs=true: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" } fed=[]\n",
    "XF union-bad-w99 pop=true obs=false: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }\n",
    "XF union-bad-w99 pop=true obs=true: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-w99-h pop=false obs=false: failed ShapeMismatch\n",
    "PF union-w99-h pop=false obs=true: failed ShapeMismatch fed=[]\n",
    "XF union-w99-h pop=false obs=false: failed ShapeMismatch\n",
    "XF union-w99-h pop=false obs=true: failed ShapeMismatch fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-w99-h pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF union-w99-h pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF union-w99-h pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF union-w99-h pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-h-w99 pop=false obs=false: failed ShapeMismatch\n",
    "PF union-h-w99 pop=false obs=true: failed ShapeMismatch fed=[]\n",
    "XF union-h-w99 pop=false obs=false: failed ShapeMismatch\n",
    "XF union-h-w99 pop=false obs=true: failed ShapeMismatch fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-h-w99 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF union-h-w99 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF union-h-w99 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF union-h-w99 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-bad-p99 pop=false obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF union-bad-p99 pop=false obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF union-bad-p99 pop=false obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF union-bad-p99 pop=false obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-bad-p99 pop=true obs=false: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }\n",
    "PF union-bad-p99 pop=true obs=true: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" } fed=[]\n",
    "XF union-bad-p99 pop=true obs=false: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }\n",
    "XF union-bad-p99 pop=true obs=true: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-p99-bad pop=false obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF union-p99-bad pop=false obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF union-p99-bad pop=false obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF union-p99-bad pop=false obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-p99-bad pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF union-p99-bad pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF union-p99-bad pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF union-p99-bad pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-p1x-w99 pop=false obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF union-p1x-w99 pop=false obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF union-p1x-w99 pop=false obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF union-p1x-w99 pop=false obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-p1x-w99 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF union-p1x-w99 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF union-p1x-w99 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF union-p1x-w99 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-w99-p1x pop=false obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF union-w99-p1x pop=false obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF union-w99-p1x pop=false obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF union-w99-p1x pop=false obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF union-w99-p1x pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF union-w99-p1x pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF union-w99-p1x pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF union-w99-p1x pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF w99-of-bad pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "PF w99-of-bad pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[]\n",
    "XF w99-of-bad pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "XF w99-of-bad pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF w99-of-bad pop=true obs=false: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }\n",
    "PF w99-of-bad pop=true obs=true: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" } fed=[]\n",
    "XF w99-of-bad pop=true obs=false: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }\n",
    "XF w99-of-bad pop=true obs=true: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" } fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF bad-of-w99 pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "PF bad-of-w99 pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[]\n",
    "XF bad-of-w99 pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "XF bad-of-w99 pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF bad-of-w99 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF bad-of-w99 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF bad-of-w99 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF bad-of-w99 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF p-of-w99-of-w99 pop=false obs=false: complete [Symbol] RowCount(0)\n",
    "PF p-of-w99-of-w99 pop=false obs=true: complete [Symbol] RowCount(0) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "XF p-of-w99-of-w99 pop=false obs=false: complete [Symbol] RowCount(0)\n",
    "XF p-of-w99-of-w99 pop=false obs=true: complete [Symbol] RowCount(0) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([0, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
    "PF p-of-w99-of-w99 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "PF p-of-w99-of-w99 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "XF p-of-w99-of-w99 pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "XF p-of-w99-of-w99 pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[\"r0[RowsFed([0, 0, 0]), RowsFed([1, 0, 0]), RowsFed([0, 0, 0])]\"]\n",
);

/// What the interpreter answered for a refused step reading a node and a relation, with the relation's rows read or refused.
const NODE_AND_RELATION: &str = concat!(
    "NR join-node99-rel function=refused pop=false obs=false: filestop\n",
    "NR join-node99-rel function=refused pop=false obs=true: filestop fed=[]\n",
    "NR join-node99-rel function=refused pop=true obs=false: filestop\n",
    "NR join-node99-rel function=refused pop=true obs=true: filestop fed=[]\n",
    "NR join-node99-rel function=read pop=false obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "NR join-node99-rel function=read pop=false obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "NR join-node99-rel function=read pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "NR join-node99-rel function=read pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "NR join-w99-rel function=refused pop=false obs=false: filestop\n",
    "NR join-w99-rel function=refused pop=false obs=true: filestop fed=[]\n",
    "NR join-w99-rel function=refused pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "NR join-w99-rel function=refused pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
    "NR join-w99-rel function=read pop=false obs=false: failed Table(NoSuchColumn { index: 0 })\n",
    "NR join-w99-rel function=read pop=false obs=true: failed Table(NoSuchColumn { index: 0 }) fed=[]\n",
    "NR join-w99-rel function=read pop=true obs=false: failed Table(NoSuchColumn { index: 99 })\n",
    "NR join-w99-rel function=read pop=true obs=true: failed Table(NoSuchColumn { index: 99 }) fed=[]\n",
);

#[test]
fn refusals_around_refused_steps_answer_as_the_interpreter_in_each_pass() {
    let mut answered = String::new();
    for (name, operations, emit) in shapes() {
        for populated in [false, true] {
            for (pass, scope) in [("PF", PF), ("XF", XF)] {
                let steps: Vec<_> = operations
                    .iter()
                    .cloned()
                    .map(|each| (scope, each))
                    .collect();
                for observe in [false, true] {
                    let answer = if scope == PF {
                        per_file(&steps, emit, populated, observe, &[])
                    } else {
                        after_the_files(&steps, emit, vec![], populated, observe)
                    };
                    answered.push_str(&format!(
                        "{pass} {name} pop={populated} obs={observe}: {answer}\n"
                    ));
                }
            }
        }
    }
    assert_eq!(answered, IN_EACH_PASS);
}

#[test]
fn a_refused_step_reading_a_node_and_a_relation_answers_as_the_interpreter() {
    let table = source();
    let programs = [
        ("join-node99-rel", vec![h(), join_functions(0, 99)], 1),
        ("join-w99-rel", vec![h(), w99(0), join_functions(1, 0)], 2),
    ];
    let mut answered = String::new();
    for (name, operations, emit) in programs {
        let steps: Vec<_> = operations.iter().cloned().map(|each| (PF, each)).collect();
        for (relation, facts) in [
            ("refused", vec![]),
            ("read", vec![(Relation::Function, &table)]),
        ] {
            for populated in [false, true] {
                for observe in [false, true] {
                    let answer = per_file(&steps, emit, populated, observe, &facts);
                    answered.push_str(&format!(
                        "NR {name} function={relation} pop={populated} obs={observe}: {answer}\n"
                    ));
                }
            }
        }
    }
    assert_eq!(answered, NODE_AND_RELATION);
}
