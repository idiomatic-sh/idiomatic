//! Present checked output from the standard observers.
use super::{
    Observation, Observer, Profile, heap_totals, heap_trace, instructions, resources, same_stdout,
};
use std::{
    fs::{self, File},
    io,
    path::Path,
    process::Command,
};

/// Write the common timing, sample, heap, instruction and RSS sections. A
/// caller can put its application's counters before these sections.
///
/// # Errors
/// A captured result cannot be read or the report cannot be written.
pub fn report(output: &mut impl io::Write, profile: &Profile, directory: &Path) -> io::Result<()> {
    run_table(
        output,
        &[
            ("Ordinary", &profile.native),
            ("Software clock", &profile.samples),
            ("Heaptrack companion", &profile.heap),
            ("Cachegrind", &profile.instructions),
        ],
    )?;
    for (title, path) in [
        ("Software-clock samples by function", "samples/samples.txt"),
        (
            "Allocation counts and peak heap by site",
            "heap/allocations.txt",
        ),
        ("Instructions by function", "instructions/instructions.txt"),
    ] {
        writeln!(output, "\n## {title}\n")?;
        let report = directory.join(path);
        if title == "Instructions by function" && profile.instructions.status.code() == Some(124) {
            writeln!(output, "Skipped after the Cachegrind wall budget.")?;
        } else {
            if title == "Allocation counts and peak heap by site" {
                let heap = heap_totals(directory)?;
                writeln!(
                    output,
                    "| Image | Allocation events | Total requested bytes |\n|---|---:|---:|"
                )?;
                writeln!(
                    output,
                    "| Ordinary | Collection unavailable | Collection unavailable |\n| Heap-compatible | {} | {} |\n",
                    heap.allocations, heap.bytes
                )?;
                writeln!(
                    output,
                    "Allocation-count parity is unverified. Heaptrack provides cumulative requested bytes for the whole image and live bytes by site at the peak.\n"
                )?;
                let text = fs::read_to_string(&report)?;
                let peak = text
                    .lines()
                    .find_map(|line| line.strip_prefix("peak heap memory consumption: "))
                    .ok_or_else(|| io::Error::other("missing Heaptrack heap total at peak"))?;
                writeln!(output, "Heaptrack live heap at peak: {peak}.\n")?;
                heap_sites(output, &text)?;
                writeln!(output, "\nFull tool report: `{}`.", report.display())?;
                continue;
            }
            let text = fs::read_to_string(&report)?;
            if path.starts_with("samples/") {
                samples(output, &text)?;
            } else {
                writeln!(
                    output,
                    "Total executed instructions: {}.\n",
                    instructions(directory)?
                )?;
                functions(output, &text)?;
            }
            writeln!(output, "\nFull tool report: `{}`.", report.display())?;
        }
    }
    writeln!(output, "\n## Peak memory\n")?;
    writeln!(output, "| Image | Peak process RSS KiB |\n|---|---:|")?;
    for (label, run) in [
        ("Ordinary", &profile.native),
        ("Heap-compatible image with its observer", &profile.heap),
    ] {
        writeln!(output, "| {label} | {} |", resources(run)?.1)?;
    }
    Ok(())
}

/// Report clocks, peak RSS and one-minute load for these observed passes.
///
/// # Errors
/// A pass has an unreadable process meter, or the table cannot be written.
pub fn run_table(output: &mut impl io::Write, runs: &[(&str, &Observation)]) -> io::Result<()> {
    writeln!(
        output,
        "| Pass | Total CPU-s | Wall-s | Peak RSS KiB | Load before → after |"
    )?;
    writeln!(output, "|---|---:|---:|---:|---|")?;
    for (label, run) in runs {
        let [before, after] = run
            .load
            .map(|load| load.map_or("unavailable".into(), |v| format!("{v:.2}")));
        let (cpu, rss) = resources(run)?;
        writeln!(
            output,
            "| {label} | {cpu:.3} | {:.3} | {rss} | {before} → {after} |",
            run.elapsed.as_secs_f64()
        )?;
    }
    writeln!(output)
}

fn samples(output: &mut impl io::Write, report: &str) -> io::Result<()> {
    for line in report
        .lines()
        .filter(|line| line.starts_with("# Total Lost Samples:") || line.starts_with("# Samples:"))
    {
        writeln!(output, "{}  ", line.trim_start_matches("# "))?;
    }
    writeln!(
        output,
        "\n| Function | Image | Samples | Share |\n|---|---|---:|---:|"
    )?;
    for (share, count, symbol, image) in report
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let mut fields = line.split('\t').map(str::trim);
            Some((
                fields.next()?,
                fields.next()?,
                fields.next()?,
                fields.next()?,
            ))
        })
        .take(20)
    {
        let symbol = symbol.strip_prefix("[.] ").unwrap_or(symbol);
        writeln!(
            output,
            "| {} | {} | {count} | {share} |",
            symbol.replace('|', "\\|"),
            image.replace('|', "\\|")
        )?;
    }
    Ok(())
}

fn functions(output: &mut impl io::Write, report: &str) -> io::Result<()> {
    let (_, functions) = report
        .split_once("-- Function:file summary")
        .ok_or_else(|| io::Error::other("missing Cachegrind function summary"))?;
    writeln!(output, "| Function | Executed instructions |\n|---|---:|")?;
    for line in functions
        .lines()
        .filter_map(|line| line.strip_prefix('>'))
        .take(20)
    {
        let (count, symbol) = line
            .trim_start()
            .split_once(char::is_whitespace)
            .ok_or_else(|| io::Error::other("invalid Cachegrind function row"))?;
        let symbol = symbol.trim().trim_end_matches(':').replace('|', "\\|");
        writeln!(output, "| {symbol} | {count} |")?;
    }
    Ok(())
}

// Present the standard reporter's ranked peak entries. Heaptrack owns the
// stack attribution, grouping, quantities and peak calculation.
fn heap_sites(output: &mut impl io::Write, report: &str) -> io::Result<()> {
    let Some((_, peak)) = report.split_once("PEAK MEMORY CONSUMERS\n") else {
        return Err(io::Error::other("missing Heaptrack peak sites"));
    };
    writeln!(output, "| Allocation site | Calls | Live heap at peak |")?;
    writeln!(output, "|---|---:|---:|")?;
    let mut lines = peak.lines();
    while let Some(line) = lines.next() {
        if line == "MOST TEMPORARY ALLOCATIONS" {
            break;
        }
        if let Some((bytes, tail)) = line.split_once(" peak memory consumed over ")
            && let Some(calls) = tail.strip_suffix(" calls from")
            && let Some(site) = lines.next()
        {
            writeln!(
                output,
                "| {} | {calls} | {bytes} |",
                site.trim().replace('|', "\\|")
            )?;
        }
    }
    Ok(())
}

/// Compare the program totals reported by the standard tools for the same input.
///
/// # Errors
/// The tool's completed output cannot be read or the report cannot be written.
pub fn compare(
    output: &mut impl io::Write,
    base: (&Profile, &Path),
    other: (&Profile, &Path),
) -> io::Result<()> {
    if !same_stdout(&base.0.native, &other.0.native, Observer::Native)? {
        return Err(io::Error::other("builds produced different output"));
    }
    let old = heap_totals(base.1)?;
    let new = heap_totals(other.1)?;
    writeln!(output, "| Metric | First build | Second build | Change |")?;
    writeln!(output, "|---|---:|---:|---:|")?;
    let (old_cpu, old_peak) = resources(&base.0.native)?;
    let (new_cpu, new_peak) = resources(&other.0.native)?;
    writeln!(
        output,
        "| Ordinary CPU-s | {old_cpu:.3} | {new_cpu:.3} | {:+.3} |",
        new_cpu - old_cpu
    )?;
    let old_wall = base.0.native.elapsed.as_secs_f64();
    let new_wall = other.0.native.elapsed.as_secs_f64();
    writeln!(
        output,
        "| Ordinary wall-s | {old_wall:.3} | {new_wall:.3} | {:+.3} |",
        new_wall - old_wall
    )?;
    writeln!(
        output,
        "| Ordinary peak RSS KiB | {old_peak} | {new_peak} | {:+} |",
        i128::from(new_peak) - i128::from(old_peak)
    )?;
    writeln!(
        output,
        "| Allocation-image events | {} | {} | {:+} |",
        old.allocations,
        new.allocations,
        i128::from(new.allocations) - i128::from(old.allocations)
    )?;
    let bytes_delta = if new.bytes >= old.bytes {
        format!("+{}", new.bytes - old.bytes)
    } else {
        format!("-{}", old.bytes - new.bytes)
    };
    writeln!(
        output,
        "| Allocation-image bytes requested | {} | {} | {bytes_delta} |",
        old.bytes, new.bytes
    )?;
    if [base.0, other.0]
        .iter()
        .all(|p| p.instructions.status.code() != Some(124))
    {
        let first = instructions(base.1)?;
        let second = instructions(other.1)?;
        writeln!(
            output,
            "| Ordinary-image instructions | {first} | {second} | {:+} |",
            i128::from(second) - i128::from(first)
        )?;
    } else {
        writeln!(
            output,
            "| Ordinary-image instructions | Cachegrind budget expired | · | · |"
        )?;
    }
    let diff = other.1.join("differences");
    fs::create_dir_all(&diff)?;
    let mut perf = Command::new("perf");
    perf.args(["diff", "--sort", "symbol,dso"])
        .arg(base.1.join("samples/perf.data"))
        .arg(other.1.join("samples/perf.data"));
    tool_diff(&mut perf, &diff.join("samples.txt"))?;
    let mut heaptrack = Command::new("heaptrack_print");
    heaptrack
        .args(["-m", "false", "-n", "20", "-d"])
        .arg(heap_trace(&base.1.join("heap"))?)
        .arg("-f")
        .arg(heap_trace(&other.1.join("heap"))?);
    tool_diff(&mut heaptrack, &diff.join("allocations.txt"))?;
    writeln!(
        output,
        "\nFunction-sample and allocation-site differences are in `{}`.",
        diff.display()
    )?;
    if [base.0, other.0]
        .iter()
        .all(|p| p.instructions.status.code() != Some(124))
    {
        let mut cachegrind = Command::new("cg_annotate");
        cachegrind
            .args(["--diff", "--auto=no", "--show=Ir"])
            .arg(base.1.join("instructions/cachegrind.out"))
            .arg(other.1.join("instructions/cachegrind.out"));
        tool_diff(&mut cachegrind, &diff.join("instructions.txt"))?;
        writeln!(
            output,
            "Instruction differences by function are in `{}`.",
            diff.join("instructions.txt").display()
        )?;
    }
    Ok(())
}

fn tool_diff(tool: &mut Command, output: &Path) -> io::Result<()> {
    let status = tool.stdout(File::create(output)?).status()?;
    if !status.success() {
        return Err(io::Error::other(format!("{tool:?} exited {status}")));
    }
    Ok(())
}
