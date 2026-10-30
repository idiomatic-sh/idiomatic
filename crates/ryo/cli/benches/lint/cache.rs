//! The cache rows over a copy of ruff, each run in a process of its own so that its peak memory is its own: no store,
//! cold (the store built and kept), warm, after a function is added to one file, after a function body is edited, and after an exported function is renamed. The sequence is run for each round over a copy
//! of its own, and a metric is the median of its rounds: one round moves with the load of the machine and with how many
//! workers hold their tables at once.

use std::process::Command;

use crate::bmf::Cost;
use crate::{phases, ruff};

/// The cache rows: a copy of ruff, and a process for each run over it, so the peak of each is its own. The sequence is run `rounds`
/// times, each over a copy of its own, and a metric is judged by the median of its rounds: one round moves with the load of the machine and
/// with how many workers hold their tables at once.
pub(crate) fn of_ruff(rounds: usize) -> Vec<Cost> {
    eprintln!(
        "cache, all rules that run, ruff copied, store kept in the copy, {rounds} rounds: {:>14} {:>9} {:>10} {:>8} {:>8}",
        "wall ms", "CPU ms", "files read", "rows", "peak MB"
    );
    let (mut peaks, mut warms, mut edits): (Vec<u128>, Vec<u128>, Vec<u128>) =
        (Vec::new(), Vec::new(), Vec::new());
    for round in 1..=rounds {
        let copy = ruff::ruff_copy();
        let directory = copy.path().to_string_lossy().into_owned();
        let mut cold_rows = String::new();
        for label in ["plain", "cold", "warm", "edit", "body", "rename"] {
            match label {
                "edit" => phases::edit_ruff_copy(copy.path()),
                "body" => phases::edit_ruff_body(copy.path()),
                "rename" => phases::rename_ruff_export(copy.path()),
                _ => {}
            }
            // The copy and the store in it are read once before each run, so that every run finds them in the page cache.
            phases::pre_read(copy.path());
            let output = Command::new(std::env::current_exe().expect("this bench"))
                .args(["--measure-cache-phase", label, &directory])
                .output()
                .expect("the measuring process runs");
            assert!(
                output.status.success(),
                "cache phase {label} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let mut reported = 0_i32;
            for line in String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| line.starts_with("cache"))
            {
                let cells: Vec<&str> = line.split('\t').collect();
                if let [_, reported_label, wall, cpu, reads, rows, peak] = cells.as_slice() {
                    assert_eq!(
                        *reported_label, label,
                        "the requested cache phase is measured"
                    );
                    reported += 1_i32;
                    if *reported_label == "cold" {
                        (*rows).clone_into(&mut cold_rows);
                    }
                    assert!(
                        *reported_label != "warm" || *rows == cold_rows,
                        "a warm run gives {rows} rows and the cold run gave {cold_rows}"
                    );
                    let measured = |text: &str| {
                        text.parse::<u128>()
                            .expect("the measurement is an unsigned integer")
                    };
                    match *reported_label {
                        "cold" => peaks.push(measured(peak)),
                        "warm" => warms.push(measured(wall)),
                        "edit" => edits.push(measured(wall)),
                        _ => {}
                    }
                    eprintln!(
                        "  round {round}: {:<54} {wall:>9} {cpu:>9} {reads:>10} {rows:>8} {peak:>8}",
                        match *reported_label {
                            "plain" => "no store (cold, nothing kept)",
                            "cold" => "cold, the store built and kept",
                            "warm" => "warm",
                            "edit" => "after a function is added to one file",
                            "body" => "after a function body is edited",
                            _ => "after an exported function is renamed",
                        }
                    );
                } else {
                    panic!("malformed cache measurement: {line}");
                }
            }
            assert_eq!(
                reported, 1_i32,
                "each requested cache phase reports exactly once"
            );
        }
    }
    eprintln!(
        "{rounds} rounds: peak [{}] MB, warm [{}] ms, edit [{}] ms",
        crate::bmf::listed(&peaks),
        crate::bmf::listed(&warms),
        crate::bmf::listed(&edits)
    );
    vec![
        Cost {
            target: "ruff",
            metric: "peak-megabytes",
            rounds: peaks,
        },
        Cost {
            target: "ruff",
            metric: "warm-milliseconds",
            rounds: warms,
        },
        Cost {
            target: "ruff",
            metric: "edit-milliseconds",
            rounds: edits,
        },
    ]
}
